use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tracing::warn;

const HOUR: u64 = 60u64.pow(2);
const DAY: u64 = 24 * HOUR;
const WEEK: u64 = 7 * DAY;

// Caps how far usage can lift a weak match above a strong one. nucleo scores
// land in the low hundreds, so this is worth roughly one grade of match
const MAX_BONUS: f64 = 120.0;

#[derive(Debug, Clone, Copy, Deserialize, Serialize)]
struct Entry {
    launches: u32,
    last_used: u64,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct Ranking {
    #[serde(default)]
    entries: HashMap<String, Entry>,
    #[serde(skip)]
    path: PathBuf,
}

impl Ranking {
    pub fn load() -> Self {
        let path = fh_paths::state_dir().join("ranking.toml");
        let mut ranking = match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_else(|e| {
                warn!(%e, "ranking file is unreadable, starting fresh");
                Self::default()
            }),
            // No history yet, first run normality
            Err(_) => Self::default(),
        };

        ranking.path = path;
        ranking
    }

    pub fn record(&mut self, key: &str) {
        let entry = self.entries.entry(key.to_owned()).or_insert(Entry {
            launches: 0,
            last_used: 0,
        });

        entry.launches = entry.launches.saturating_add(1);
        entry.last_used = now();

        self.save();
    }

    // Launches weighted by how recently the thing was last used
    pub fn score(&self, key: &str) -> f64 {
        let Some(entry) = self.entries.get(key) else {
            return 0.0;
        };
        let age = now().saturating_sub(entry.last_used);
        let weight = if age < HOUR {
            4.0
        } else if age < DAY {
            2.0
        } else if age < WEEK {
            0.5
        } else {
            0.25
        };
        f64::from(entry.launches) * weight
    }

    pub fn bonus(&self, key: &str) -> f64 {
        self.score(key).min(MAX_BONUS)
    }

    fn save(&self) {
        // A store with no path is an in-memory one, as used by tests
        if self.path.as_os_str().is_empty() {
            return;
        }

        let Some(parent) = self.path.parent() else {
            return;
        };

        if let Err(error) = fs::create_dir_all(parent) {
            warn!(%error, "could not create the state directory");
            return;
        }

        let text = match toml::to_string_pretty(self) {
            Ok(text) => text,
            Err(e) => {
                warn!(%e, "could not encode ranking");
                return;
            }
        };

        // Write beside the target and rename, so a kill mid-write cannot
        // leave a truncated file behind
        let temp = self.path.with_extension("toml.tmp");

        if let Err(e) = fs::write(&temp, text) {
            warn!(%e, "could not write ranking");
            return;
        }
        if let Err(e) = fs::rename(&temp, &self.path) {
            warn!(%e, "could not replace ranking");
        }
    }
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|since| since.as_secs())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{DAY, Entry, Ranking, WEEK, now};
    use std::{collections::HashMap, env};

    fn ranking(entries: &[(&str, u32, u64)]) -> Ranking {
        Ranking {
            entries: entries
                .iter()
                .map(|(key, launches, age)| {
                    (
                        (*key).to_owned(),
                        Entry {
                            launches: *launches,
                            last_used: now().saturating_sub(*age),
                        },
                    )
                })
                .collect(),
            path: Default::default(),
        }
    }

    #[test]
    fn an_unknown_key_scores_nothing() {
        assert_eq!(ranking(&[]).score("firefox"), 0.0);
    }

    #[test]
    fn recent_use_outweighs_raw_volume() {
        let ranking = ranking(&[("recent", 10, 60), ("stale", 60, 4 * WEEK)]);
        // 10 * 4 beats 60 * 0.25
        assert!(ranking.score("recent") > ranking.score("stale"));
    }

    #[test]
    fn volume_still_decides_within_a_band() {
        let ranking = ranking(&[("often", 20, 2 * DAY), ("rarely", 3, 2 * DAY)]);
        assert!(ranking.score("often") > ranking.score("rarely"));
    }

    #[test]
    fn recording_accumulates() {
        let mut ranking = Ranking {
            entries: HashMap::new(),
            path: env::temp_dir().join("pl-ranking-test.toml"),
        };

        (0..=1).for_each(|_| {
            ranking.record("firefox");
        });
        assert_eq!(ranking.entries["firefox"].launches, 2);
    }
}

mod index;
mod ranking;

use crate::desktop_entries::index::Entry;
use crate::desktop_entries::ranking::Ranking;
use fh_ipc::{GpuPreference, IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_service::{Plugin, Usage};
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use std::cmp::Ordering;
use tracing::info;

const MAX_RESULTS: usize = 8;

pub struct DesktopEntries {
    entries: Vec<Entry>,
    matcher: Matcher,
    haystack_buffer: Vec<char>,
    selections: Vec<usize>,
    // Applications are the only results worth ranking, so the store lives
    // here rather than in the registry
    ranking: Ranking,
}

impl DesktopEntries {
    pub fn load() -> Self {
        let entries = index::load();
        info!(count = entries.len(), "indexed desktop entries");

        Self::with_entries(entries)
    }

    fn with_entries(entries: Vec<Entry>) -> Self {
        Self::with_entries_and_ranking(Ranking::load(), entries)
    }

    fn with_entries_and_ranking(ranking: Ranking, entries: Vec<Entry>) -> Self {
        Self {
            entries,
            matcher: Matcher::new(Config::DEFAULT),
            haystack_buffer: Vec::new(),
            selections: Vec::new(),
            ranking,
        }
    }

    fn resolve(&self, id: Indice) -> Option<&Entry> {
        let index = *self.selections.get(id as usize)?;
        self.entries.get(index)
    }
}

impl Plugin for DesktopEntries {
    fn name(&self) -> &'static str {
        "desktop entries"
    }

    fn accepts(&self, _query: &str) -> bool {
        true
    }

    fn usage(&self) -> Vec<Usage> {
        vec![Usage {
            prefix: String::new(),
            example: "firefox".into(),
            description: "Search applications. Empty shows most used".into(),
        }]
    }

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        // Destructured so the scoring closure can hold the matcher and buffer
        // mutably while iterating the entries immutably
        let Self {
            entries,
            matcher,
            haystack_buffer,
            selections,
            ranking,
        } = self;

        selections.clear();
        let query = query.trim();
        let mut scored: Vec<(f64, usize)> = if query.is_empty() {
            // An empty query is a request for what is used most
            entries
                .iter()
                .enumerate()
                .map(|(idx, entry)| (ranking.score(&entry.path.to_string_lossy()), idx))
                .filter(|(score, _)| *score > 0.0)
                .collect()
        } else {
            let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);

            entries
                .iter()
                .enumerate()
                .filter_map(|(idx, entry)| {
                    let haystack = Utf32Str::new(&entry.haystack, haystack_buffer);
                    let score = pattern.score(haystack, matcher)?;

                    Some((
                        f64::from(score) + ranking.bonus(&entry.path.to_string_lossy()),
                        idx,
                    ))
                })
                .collect()
        };

        // Name breaks score ties
        scored.sort_by(|left, right| {
            right
                .0
                .partial_cmp(&left.0)
                .unwrap_or(Ordering::Equal)
                .then_with(|| entries[left.1].name.cmp(&entries[right.1].name))
        });
        scored.truncate(MAX_RESULTS);

        scored
            .into_iter()
            .enumerate()
            .map(|(id, (_, idx))| {
                selections.push(idx);

                let entry = &entries[idx];

                PluginSearchResult {
                    id: id as Indice,
                    name: entry.name.clone(),
                    description: entry.description.clone(),
                    keywords: None,
                    icon: entry.icon.clone().map(IconSource::Name),
                    exec: None,
                    window: None,
                }
            })
            .collect()
    }

    fn activate(&mut self, id: Indice) -> Vec<PluginResponse> {
        let Some(entry) = self.resolve(id) else {
            return Vec::new();
        };

        let path = entry.path.clone();
        let gpu_preference = if entry.prefers_non_default_gpu {
            GpuPreference::NonDefault
        } else {
            GpuPreference::Default
        };

        // The borrow of self ends above, so the store can be written here
        self.ranking.record(&path.to_string_lossy());

        vec![
            PluginResponse::DesktopEntry {
                path,
                gpu_preference,
            },
            PluginResponse::Close,
        ]
    }

    fn complete(&mut self, id: Indice) -> Option<String> {
        self.resolve(id).map(|entry| entry.name.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::ranking::Ranking;
    use super::{DesktopEntries, MAX_RESULTS};
    use crate::desktop_entries::index::Entry;
    use fh_ipc::PluginResponse;
    use fh_service::Plugin;

    fn entry(name: &str, description: &str) -> Entry {
        Entry {
            path: format!("/usr/share/applications/{name}.desktop").into(),
            name: name.into(),
            description: description.into(),
            icon: Some(name.to_lowercase()),
            prefers_non_default_gpu: false,
            haystack: format!("{name} {description}"),
        }
    }

    fn entries() -> Vec<Entry> {
        vec![
            entry("Firefox", "Web Browser"),
            entry("Konsole", "Terminal"),
            entry("System Settings", "Configure KDE Plasma"),
        ]
    }

    fn plugin() -> DesktopEntries {
        DesktopEntries::with_entries_and_ranking(Ranking::default(), entries())
    }

    #[test]
    fn abbreviations_match_subsequences() {
        let mut plugin = plugin();
        let results = plugin.search("fx");

        assert_eq!(
            results.first().map(|result| result.name.as_str()),
            Some("Firefox"),
        );
    }

    #[test]
    fn the_description_is_searchable() {
        let mut plugin = plugin();
        let results = plugin.search("browser");

        assert_eq!(
            results.first().map(|result| result.name.as_str()),
            Some("Firefox"),
        );
    }

    #[test]
    fn an_empty_query_returns_nothing_without_history() {
        let mut plugin = plugin();
        assert!(plugin.search(" ").is_empty());
    }

    #[test]
    fn an_empty_query_returns_the_most_used() {
        let mut ranking = Ranking::default();

        // Keyed off the fixture so the two cannot drift apart
        ranking.record(&entry("Konsole", "Terminal").path.to_string_lossy());

        let mut plugin = DesktopEntries::with_entries_and_ranking(ranking, entries());
        let results = plugin.search("");

        assert_eq!(
            results.first().map(|result| result.name.as_str()),
            Some("Konsole")
        );
    }

    #[test]
    fn ids_are_positional_and_resolve_to_entries() {
        let mut plugin = plugin();
        let results = plugin.search("o");

        for result in &results {
            let entry = plugin.resolve(result.id).expect("id came from this search");
            assert_eq!(entry.name, result.name);
        }
    }

    #[test]
    fn ids_from_a_stale_search_do_not_resolve() {
        let mut plugin = plugin();
        let results = plugin.search("konsole");
        assert_eq!(results.len(), 1);

        plugin.search("nothing matches this");
        assert!(plugin.resolve(0).is_none());
    }

    #[test]
    fn activation_launches_and_closes() {
        let mut plugin = plugin();
        plugin.search("konsole");

        let responses = plugin.activate(0);
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[1], PluginResponse::Close);
    }

    #[test]
    fn results_are_capped() {
        let entries = (0..50)
            .map(|num| entry(&format!("App{num}"), "test"))
            .collect();
        let mut plugin = DesktopEntries::with_entries_and_ranking(Ranking::default(), entries);

        assert_eq!(plugin.search("app").len(), MAX_RESULTS);
    }
}

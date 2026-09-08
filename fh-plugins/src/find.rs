use crate::{paths::expand, spawn::detached};
use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_service::{Plugin, Usage};
use ignore::{WalkBuilder, WalkState};
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use serde::Deserialize;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

const MAX_RESULTS: usize = 8;
const MAX_CANDIDATES: usize = 200;
const MIN_TERM: usize = 2;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub prefix: String,
    pub roots: Vec<String>,
    pub max_depth: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            prefix: "find".into(),
            roots: vec!["~".into()],
            max_depth: 6,
        }
    }
}

pub struct Find {
    settings: Settings,
    matcher: Matcher,
    buffer: Vec<char>,
    matches: Vec<PathBuf>,
}

impl Find {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            matcher: Matcher::new(Config::DEFAULT),
            buffer: Vec::new(),
            matches: Vec::new(),
        }
    }

    fn term<'a>(&self, query: &'a str) -> Option<&'a str> {
        let (prefix, rest) = query.trim().split_once(char::is_whitespace)?;

        if prefix != self.settings.prefix {
            return None;
        }

        let rest = rest.trim();
        (rest.chars().count() >= MIN_TERM).then_some(rest)
    }

    fn candidates(&self, term: &str) -> Vec<PathBuf> {
        let Some((first, rest)) = self.settings.roots.split_first() else {
            return Vec::new();
        };
        let mut builder = WalkBuilder::new(expand(first));

        rest.iter().for_each(|root| {
            builder.add(expand(root));
        });

        builder
            .hidden(true)
            .git_ignore(true)
            .max_depth(Some(self.settings.max_depth))
            .threads(0);

        let found = Arc::new(Mutex::new(Vec::new()));
        let needle = term.to_lowercase();

        builder.build_parallel().run(|| {
            let found = Arc::clone(&found);
            let needle = needle.clone();

            Box::new(move |entry| {
                let Ok(entry) = entry else {
                    return WalkState::Continue;
                };

                if !entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .contains(&needle)
                {
                    return WalkState::Continue;
                }

                let mut found = found.lock().expect("a walker thread panicked");

                if found.len() >= MAX_CANDIDATES {
                    return WalkState::Quit;
                }

                found.push(entry.path().to_path_buf());

                WalkState::Continue
            })
        });

        Arc::try_unwrap(found)
            .map(|found| found.into_inner().expect("walker thread panicked"))
            .unwrap_or_default()
    }
}

impl Plugin for Find {
    fn name(&self) -> &'static str {
        "find"
    }

    fn accepts(&self, query: &str) -> bool {
        self.term(query).is_some()
    }

    fn isolates(&self, query: &str) -> bool {
        query.trim().starts_with(&self.settings.prefix)
    }

    fn usage(&self) -> Vec<Usage> {
        vec![Usage {
            prefix: self.settings.prefix.clone(),
            example: format!("{} report.odt", self.settings.prefix),
            description: format!("Search files under {}", self.settings.roots.join(", ")),
        }]
    }

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        self.matches.clear();

        let Some(term) = self.term(query) else {
            return Vec::new();
        };
        let candidates = self.candidates(term);
        let pattern = Pattern::parse(term, CaseMatching::Ignore, Normalization::Smart);
        let Self {
            matcher, buffer, ..
        } = self;
        let mut scored: Vec<(u32, PathBuf)> = candidates
            .into_iter()
            .filter_map(|path| {
                let name = path.file_name()?.to_string_lossy().into_owned();
                let score = pattern.score(Utf32Str::new(&name, buffer), matcher)?;

                Some((score, path))
            })
            .collect();

        scored.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(&right.1)));
        scored.truncate(MAX_RESULTS);

        self.matches = scored.into_iter().map(|(_, path)| path).collect();

        self.matches
            .iter()
            .enumerate()
            .map(|(id, path)| PluginSearchResult {
                id: id as Indice,
                name: path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned(),
                description: path
                    .parent()
                    .map(|parent| parent.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                keywords: None,
                icon: Some(IconSource::Name("text-x-generic".to_owned())),
                exec: None,
                window: None,
            })
            .collect()
    }

    fn activate(&mut self, id: Indice) -> Vec<PluginResponse> {
        let Some(path) = self.matches.get(id as usize) else {
            return Vec::new();
        };
        detached("xdg-open", &[&path.to_string_lossy()]);
        vec![PluginResponse::Close]
    }
}

#[cfg(test)]
mod tests {
    use super::{Find, Settings};
    use fh_service::Plugin;

    fn find() -> Find {
        Find::new(Settings::default())
    }

    #[test]
    fn a_short_term_does_not_start_a_walk() {
        let find = find();
        assert!(!find.accepts("find a"));
        assert!(find.accepts("find ab"));
    }

    #[test]
    fn another_prefix_is_ignored() {
        let find = find();
        assert!(!find.accepts("finder something"));
        assert!(!find.accepts("g something"));
    }

    #[test]
    fn the_prefix_isolates_before_the_term_is_long_enough() {
        let find = find();

        // Otherwise the applications would flash underneath while typing
        assert!(find.isolates("find a"));
    }
}

mod index;

use crate::desktop_entries::index::Entry;
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use pl_ipc::{GpuPreference, IconSource, Indice, PluginResponse, PluginSearchResult};
use pl_service::Plugin;
use tracing::info;

const MAX_RESULTS: usize = 8;

pub struct DesktopEntries {
    entries: Vec<Entry>,
    matcher: Matcher,
    haystack_buffer: Vec<char>,
    selections: Vec<usize>,
}

impl DesktopEntries {
    pub fn load() -> Self {
        let entries = index::load();
        info!(count = entries.len(), "indexed desktop entries");

        Self::with_entries(entries)
    }

    fn with_entries(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            matcher: Matcher::new(Config::DEFAULT),
            haystack_buffer: Vec::new(),
            selections: Vec::new(),
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

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        // Destructured so the scoring closure can hold the matcher and buffer
        // mutably while iterating the entries immutably
        let Self {
            entries,
            matcher,
            haystack_buffer,
            selections,
        } = self;

        selections.clear();
        let query = query.trim();
        if query.is_empty() {
            return Vec::new();
        }

        let pattern = Pattern::parse(query, CaseMatching::Ignore, Normalization::Smart);
        let mut scored: Vec<(u32, usize)> = entries
            .iter()
            .enumerate()
            .filter_map(|(idx, entry)| {
                let haystack = Utf32Str::new(&entry.haystack, haystack_buffer);
                pattern.score(haystack, matcher).map(|score| (score, idx))
            })
            .collect();

        // Name breaks score ties
        scored.sort_by(|left, right| {
            right
                .0
                .cmp(&left.0)
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
        let gpu_preference = if entry.prefers_non_default_gpu {
            GpuPreference::NonDefault
        } else {
            GpuPreference::Default
        };

        vec![
            PluginResponse::DesktopEntry {
                path: entry.path.clone(),
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
    use super::{DesktopEntries, MAX_RESULTS};
    use crate::desktop_entries::index::Entry;
    use pl_ipc::PluginResponse;
    use pl_service::Plugin;

    fn entry(name: &str, description: &str) -> Entry {
        Entry {
            path: format!("/usr/share/applicatons/{name}.desktop").into(),
            name: name.into(),
            description: description.into(),
            icon: Some(name.to_lowercase()),
            prefers_non_default_gpu: false,
            haystack: format!("{name} {description}"),
        }
    }

    fn plugin() -> DesktopEntries {
        DesktopEntries::with_entries(vec![
            entry("Firefox", "Web Browser"),
            entry("Konsole", "Terminal"),
            entry("System Settings", "Configure KDE Plasma"),
        ])
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
    fn an_empty_query_returns_nothing() {
        let mut plugin = plugin();
        assert!(plugin.search(" ").is_empty());
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
        let mut plugin = DesktopEntries::with_entries(entries);

        assert_eq!(plugin.search("app").len(), MAX_RESULTS);
    }
}

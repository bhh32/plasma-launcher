use crate::index::Entry;
use nucleo_matcher::{
    Config, Matcher, Utf32Str,
    pattern::{CaseMatching, Normalization, Pattern},
};
use pl_ipc::{IconSource, Indice, SearchResult};

const MAX_RESULTS: usize = 8;

pub struct Searcher {
    entries: Vec<Entry>,
    matcher: Matcher,
    haystack_buffer: Vec<char>,
    selections: Vec<usize>,
}

impl Searcher {
    pub fn new(entries: Vec<Entry>) -> Self {
        Self {
            entries,
            matcher: Matcher::new(Config::DEFAULT),
            haystack_buffer: Vec::new(),
            selections: Vec::new(),
        }
    }

    pub fn search(&mut self, query: &str) -> Vec<SearchResult> {
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

        // Name breaks score ties so the order does not shuffle between
        // keystrokes that happen to score identically
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

                SearchResult {
                    id: id as Indice,
                    name: entry.name.clone(),
                    description: entry.description.clone(),
                    icon: entry.icon.clone().map(IconSource::Name),
                    category_icon: None,
                    window: None,
                }
            })
            .collect()
    }

    pub fn resolve(&self, id: Indice) -> Option<&Entry> {
        let index = *self.selections.get(id as usize)?;
        self.entries.get(index)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_RESULTS, Searcher};
    use crate::index::Entry;

    fn entry(name: &str, description: &str) -> Entry {
        Entry {
            path: format!("/usr/share/applications/{name}.desktop").into(),
            name: name.to_owned(),
            description: description.to_owned(),
            icon: Some(name.to_lowercase()),
            prefers_non_default_gpu: false,
            haystack: format!("{name} {description}"),
        }
    }

    fn searcher() -> Searcher {
        Searcher::new(vec![
            entry("Firefox", "Web Browser"),
            entry("Konsole", "Terminal"),
            entry("System Settings", "Configure KDE Plasma"),
        ])
    }

    #[test]
    fn abbreviations_match_subsequences() {
        let mut searcher = searcher();
        let results = searcher.search("fx");

        assert_eq!(
            results.first().map(|result| result.name.as_str()),
            Some("Firefox")
        );
    }

    #[test]
    fn the_description_is_searchable() {
        let mut searcher = searcher();
        let results = searcher.search("browser");

        assert_eq!(
            results.first().map(|result| result.name.as_str()),
            Some("Firefox")
        );
    }

    #[test]
    fn an_empty_query_returns_nothing() {
        let mut searcher = searcher();
        assert!(searcher.search(" ").is_empty());
    }

    #[test]
    fn ids_are_positional_and_resolve_to_entries() {
        let mut searcher = searcher();
        let results = searcher.search("o");

        for result in &results {
            let entry = searcher
                .resolve(result.id)
                .expect("id came from this search");
            assert_eq!(entry.name, result.name);
        }
    }

    #[test]
    fn ids_from_a_stale_search_do_not_resolve() {
        let mut searcher = searcher();
        let results = searcher.search("konsole");
        assert_eq!(results.len(), 1);

        searcher.search("nothing matches this");
        assert!(searcher.resolve(0).is_none());
    }

    #[test]
    fn results_are_capped() {
        let entries = (0..50)
            .map(|num| entry(&format!("App{num}"), "test"))
            .collect();
        let mut searcher = Searcher::new(entries);

        assert_eq!(searcher.search("app").len(), MAX_RESULTS);
    }
}

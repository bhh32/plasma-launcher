use crate::Ranking;

use super::Plugin;
use pl_ipc::{Indice, PluginResponse, SearchResult};
use tracing::debug;

const MAX_RESULTS: usize = 8;

#[derive(Clone, Copy)]
struct Selection {
    plugin: usize,
    local: Indice,
}

pub struct Registry {
    plugins: Vec<Box<dyn Plugin>>,
    selections: Vec<Selection>,
    ranking: Ranking,
}

impl Registry {
    pub fn new(plugins: Vec<Box<dyn Plugin>>) -> Self {
        Self {
            plugins,
            selections: Vec::new(),
            ranking: Ranking::load(),
        }
    }

    pub fn search(&mut self, query: &str) -> Vec<SearchResult> {
        self.selections.clear();

        let query = query.trim();

        // An isolating plugin takes the list to itself
        let participating: Vec<usize> = match self
            .plugins
            .iter()
            .position(|plugin| plugin.isolates(query))
        {
            Some(idx) => vec![idx],
            None => (0..self.plugins.len())
                .filter(|&idx| self.plugins[idx].accepts(query))
                .collect(),
        };
        let mut results = Vec::new();

        for idx in participating {
            for result in self.plugins[idx].search(query, &self.ranking) {
                if results.len() == MAX_RESULTS {
                    break;
                }

                self.selections.push(Selection {
                    plugin: idx,
                    local: result.id,
                });
                results.push(SearchResult {
                    id: results.len() as Indice,
                    name: result.name,
                    description: result.description,
                    icon: result.icon,
                    category_icon: None,
                    window: result.window,
                });
            }
        }

        results
    }

    pub fn activate(&mut self, id: Indice) -> Vec<PluginResponse> {
        let Some(sel) = self.selections.get(id as usize).copied() else {
            debug!(id, "activate for an id outside the last result set");
            return Vec::new();
        };
        if let Some(key) = self.plugins[sel.plugin].key(sel.local) {
            self.ranking.record(&key);
        }

        self.plugins[sel.plugin].activate(sel.local)
    }

    pub fn complete(&mut self, id: Indice) -> Option<String> {
        let sel = self.selections.get(id as usize).copied()?;
        self.plugins[sel.plugin].complete(sel.local)
    }
}

#[cfg(test)]
mod tests {
    use crate::Ranking;

    use super::{super::Plugin, Registry};
    use pl_ipc::{Indice, PluginResponse, PluginSearchResult};

    struct Fake {
        name: &'static str,
        isolating: bool,
        count: usize,
    }

    impl Plugin for Fake {
        fn name(&self) -> &'static str {
            self.name
        }

        fn accepts(&self, _query: &str) -> bool {
            true
        }

        fn isolates(&self, query: &str) -> bool {
            self.isolating && query.starts_with('=')
        }

        fn search(&mut self, _query: &str, _ranking: &Ranking) -> Vec<PluginSearchResult> {
            (0..self.count)
                .map(|num| PluginSearchResult {
                    id: num as Indice,
                    name: format!("{} {num}", self.name),
                    description: String::new(),
                    keywords: None,
                    icon: None,
                    exec: None,
                    window: None,
                })
                .collect()
        }

        fn activate(&mut self, id: Indice) -> Vec<PluginResponse> {
            vec![PluginResponse::Fill(format!("{} {id}", self.name))]
        }
    }

    fn registry() -> Registry {
        Registry::new(vec![
            Box::new(Fake {
                name: "calc",
                isolating: true,
                count: 1,
            }),
            Box::new(Fake {
                name: "apps",
                isolating: false,
                count: 3,
            }),
        ])
    }

    #[test]
    fn global_ids_are_contiguous_across_plugins() {
        let mut registry = registry();
        let results = registry.search("x");

        assert_eq!(results.len(), 4);
        assert_eq!(
            results.iter().map(|result| result.id).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        );
    }

    #[test]
    fn a_global_id_activates_the_plugin_that_produced_it() {
        let mut registry = registry();
        registry.search("x");

        // Global id 2 is the second result from the second plugin, whose
        // own id for it is 1
        assert_eq!(
            registry.activate(2),
            vec![PluginResponse::Fill("apps 1".into())]
        );
    }

    #[test]
    fn an_isolating_plugin_is_the_only_source() {
        let mut registry = registry();
        let results = registry.search("= 2+2");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "calc 0");
    }

    #[test]
    fn stale_ids_do_not_activate() {
        let mut registry = registry();
        registry.search("x");
        registry.search("= 2+2");

        assert!(registry.activate(3).is_empty());
    }
}

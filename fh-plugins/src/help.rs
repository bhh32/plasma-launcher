use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_service::{Plugin, Usage};

pub struct Topic {
    plugin: String,
    entries: Vec<Usage>,
}

impl Topic {
    pub fn new(plugin: String, entries: Vec<Usage>) -> Self {
        Self { plugin, entries }
    }

    pub fn of(plugin: &impl Plugin) -> Self {
        Self {
            plugin: plugin.name().to_owned(),
            entries: plugin.usage(),
        }
    }
}

pub struct Help {
    topics: Vec<Topic>,
    outcome: Vec<String>,
}

impl Help {
    pub fn new(topics: Vec<Topic>) -> Self {
        let mut topics = topics;

        topics.insert(
            0,
            Topic {
                plugin: "help".into(),
                entries: vec![Usage {
                    prefix: "?".into(),
                    example: "? web".into(),
                    description: "Help for one plugin".into(),
                }],
            },
        );

        Self {
            topics,
            outcome: Vec::new(),
        }
    }

    // One row per plugin, described by the prefixes it answers to
    fn plugins(&self) -> Vec<(String, String, String)> {
        self.topics
            .iter()
            .map(|topic| {
                let prefixes: Vec<&str> = topic
                    .entries
                    .iter()
                    .map(|usage| usage.prefix.as_str())
                    .filter(|prefix| !prefix.is_empty())
                    .collect();
                let description = if prefixes.is_empty() {
                    "typed directly".to_owned()
                } else {
                    prefixes.join(", ")
                };

                (
                    format!("? {}", topic.plugin),
                    topic.plugin.clone(),
                    description,
                )
            })
            .collect()
    }

    fn entries(&self, wanted: &str) -> Vec<(String, String, String)> {
        self.topics
            .iter()
            .filter(|topic| topic.plugin.to_lowercase().starts_with(wanted))
            .flat_map(|topic| topic.entries.iter())
            .map(|usage| {
                let fill = if usage.prefix.is_empty() {
                    String::new()
                } else {
                    usage.prefix.to_string()
                };

                (fill, usage.example.clone(), usage.description.clone())
            })
            .collect()
    }
}

impl Plugin for Help {
    fn name(&self) -> &'static str {
        "help"
    }

    fn accepts(&self, query: &str) -> bool {
        query.trim_start().starts_with('?')
    }

    fn isolates(&self, query: &str) -> bool {
        self.accepts(query)
    }

    fn usage(&self) -> Vec<Usage> {
        Vec::new()
    }

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        self.outcome.clear();

        let Some(rest) = query.trim().strip_prefix('?') else {
            return Vec::new();
        };
        let wanted = rest.trim().to_lowercase();

        let rows = if wanted.is_empty() {
            self.plugins()
        } else {
            self.entries(&wanted)
        };

        rows.into_iter()
            .enumerate()
            .map(|(id, (fill, name, description))| {
                self.outcome.push(fill);

                PluginSearchResult {
                    id: id as Indice,
                    name,
                    description,
                    keywords: None,
                    icon: Some(IconSource::Name("help-about".to_owned())),
                    exec: None,
                    window: None,
                }
            })
            .collect()
    }

    fn activate(&mut self, id: Indice) -> Vec<PluginResponse> {
        let Some(fill) = self.outcome.get(id as usize) else {
            return Vec::new();
        };

        // No Close, so the launcher stays open with the ext ready to use
        if fill.is_empty() {
            return Vec::new();
        }

        vec![PluginResponse::Fill(fill.clone())]
    }
}

#[cfg(test)]
mod tests {
    use super::{Help, Topic};
    use fh_ipc::PluginResponse;
    use fh_service::{Plugin, Usage};

    fn topic(plugin: &str, prefix: &str) -> Topic {
        Topic {
            plugin: plugin.into(),
            entries: vec![Usage {
                prefix: prefix.into(),
                example: format!("{prefix} thing"),
                description: "does a thing".into(),
            }],
        }
    }

    fn help() -> Help {
        Help::new(vec![topic("web", "g"), topic("terminal", "t")])
    }

    #[test]
    fn default_is_plugins() {
        let mut help = help();
        let results = help.search("?");

        // Two topics + help's
        assert_eq!(results.len(), 3);
        assert_eq!(results[0].name, "help");
        assert_eq!(results[1].name, "web");
        assert_eq!(results[1].description, "g");
    }

    #[test]
    fn a_plugin_row_drills_into_that_plugin() {
        let mut help = help();
        help.search("?");
        assert_eq!(help.activate(1), vec![PluginResponse::Fill("? web".into())])
    }

    #[test]
    fn a_named_plugin_narrows_the_list() {
        let mut help = help();
        let results = help.search("? web");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "g thing");
    }

    #[test]
    fn the_name_can_be_abbreviated() {
        let mut help = help();
        assert_eq!(help.search("? term").len(), 1);
    }

    #[test]
    fn an_unknown_name_lists_nothing() {
        let mut help = help();
        assert!(help.search("? nosuch").is_empty());
    }

    #[test]
    fn an_entry_row_primes_its_prefix() {
        let mut help = help();
        help.search("? web");
        assert_eq!(help.activate(0), vec![PluginResponse::Fill("g".into())]);
    }
}

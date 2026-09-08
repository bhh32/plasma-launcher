use crate::{find, terminal, web};
use fh_config::Config;
use serde::Deserialize;
use std::fs;
use tracing::warn;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub web: web::Settings,
    pub terminal: terminal::Settings,
    pub find: find::Settings,
}

// Only the [plugins] table matters here. Serde ignores everything else.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Document {
    plugins: Settings,
}

impl Settings {
    pub fn load() -> Self {
        let path = Config::path();
        let Ok(text) = fs::read_to_string(&path) else {
            return Self::default();
        };

        match toml::from_str::<Document>(&text) {
            Ok(doc) => doc.plugins,
            Err(e) => {
                warn!(%e, "plugin settings are unreadable, using defaults");
                Self::default()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Document;

    #[test]
    fn an_absent_section_gives_built_in_defaults() {
        let doc: Document = toml::from_str("").expect("parses");

        assert_eq!(doc.plugins.terminal.prefix, "t");
        assert!(doc.plugins.web.keywords.contains_key("g"));
    }

    #[test]
    fn appearance_keys_are_ignored_here() {
        let doc: Document = toml::from_str("[appearance]\ncard_width = 720").expect("parses");
        assert_eq!(doc.plugins.terminal.prefix, "t");
    }

    #[test]
    fn a_configured_keyword_list_replaces_the_built_ins() {
        let doc: Document =
            toml::from_str("[plugins.web.keywords]\nq = \"https://example.com/{}\"")
                .expect("parses");

        assert_eq!(doc.plugins.web.keywords.len(), 1);
        assert!(!doc.plugins.web.keywords.contains_key("g"));
    }
}

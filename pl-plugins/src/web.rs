use crate::spawn::detached;
use pl_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use pl_service::{Plugin, Ranking};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub keywords: BTreeMap<String, String>,
}

impl Default for Settings {
    fn default() -> Self {
        let keywords = [
            ("ddg", "https://duckduckgo.com/?q={}"),
            ("g", "https://google.com/search?q={}"),
            ("cb", "https://codeberg.org/{}"),
            ("gh", "https://github.com/{}"),
            ("rs", "https://docs.rs/{}"),
            ("crate", "https://crates.io/crates/{}"),
            ("w", "https://en.wikipedia.org/w/index.php?search={}"),
            ("http", "{}"),
        ];

        Self {
            keywords: keywords
                .into_iter()
                .map(|(prefix, url)| (prefix.to_owned(), url.to_owned()))
                .collect(),
        }
    }
}

pub struct Web {
    settings: Settings,
    outcome: Option<String>,
}

impl Web {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            outcome: None,
        }
    }

    // A query belongs here when its first word is a known keyword and
    // something follows it.
    fn split<'a>(&'a self, query: &'a str) -> Option<(&'a str, &'a str)> {
        let (prefix, terms) = query.trim().split_once(char::is_whitespace)?;
        let terms = terms.trim();

        if terms.is_empty() {
            return None;
        }

        let template = self.settings.keywords.get(prefix)?;

        Some((template, terms))
    }
}

impl Plugin for Web {
    fn name(&self) -> &'static str {
        "web"
    }

    fn accepts(&self, query: &str) -> bool {
        self.split(query).is_some()
    }

    fn isolates(&self, query: &str) -> bool {
        self.split(query).is_some()
    }

    fn search(&mut self, query: &str, _ranking: &Ranking) -> Vec<PluginSearchResult> {
        let Some((template, terms)) = self.split(query) else {
            self.outcome = None;
            return Vec::new();
        };
        let url = template.replace("{}", &encode(terms));
        let name = terms.to_owned();

        self.outcome = Some(url.clone());

        vec![PluginSearchResult {
            id: 0,
            name,
            description: url,
            keywords: None,
            icon: Some(IconSource::Name("internet-web-browser".to_owned())),
            exec: None,
            window: None,
        }]
    }

    fn activate(&mut self, _id: Indice) -> Vec<PluginResponse> {
        let Some(url) = self.outcome.as_deref() else {
            return Vec::new();
        };

        // xdg-open resolves the scheme through the desktop's default
        // web browser setting. So, the default browser changing doesn't
        // change the behavior, just the browser
        detached("xdg-open", &[url]);

        vec![PluginResponse::Close]
    }
}

// Percent encoding for a query string. Everything outside the unreserved set
// is escaped and spaces become + rather than %20 so search engines read them
// as term separators.
fn encode(terms: &str) -> String {
    let mut encoded = String::with_capacity(terms.len());

    for byte in terms.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }

    encoded
}

#[cfg(test)]
mod tests {
    use super::{Settings, Web, encode};
    use pl_service::{Plugin, Ranking};

    fn web() -> Web {
        Web::new(Settings::default())
    }

    #[test]
    fn spaces_become_plus_and_specials_are_escaped() {
        assert_eq!(encode("rust traits"), "rust+traits");
        assert_eq!(encode("a&b=c"), "a%26b%3Dc");
        assert_eq!(encode("café"), "caf%C3%A9");
    }

    #[test]
    fn a_known_keyword_with_terms_produces_a_url() {
        let mut web = web();
        let results = web.search("g rust traits", &Ranking::default());

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "rust traits");
        assert_eq!(
            results[0].description,
            "https://google.com/search?q=rust+traits"
        );
    }

    #[test]
    fn a_keyword_alone_is_not_a_search() {
        let web = web();

        assert!(!web.accepts("g"));
        assert!(!web.accepts("g  "));
        assert!(web.accepts("g rust"));
    }

    #[test]
    fn an_unknown_keyword_is_ignored() {
        let web = web();

        assert!(!web.accepts("firefox browser"));
        assert!(!web.accepts("notakeyword thing"));
    }

    #[test]
    fn a_match_takes_the_whole_list() {
        let web = web();

        assert!(web.isolates("rs serde"));
        assert!(!web.isolates("serde"));
    }
}

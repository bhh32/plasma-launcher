use std::env;

use crate::spawn::detached;
use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_service::{Plugin, Usage};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub prefix: String,
    // Empty means &TERMINAL, falling back to konsole
    pub command: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            prefix: "t".into(),
            command: String::new(),
        }
    }
}

pub struct Terminal {
    prefix: String,
    command: String,
    outcome: Option<String>,
}

impl Terminal {
    pub fn new(settings: Settings) -> Self {
        let command = if settings.command.is_empty() {
            env::var("TERMINAL").unwrap_or_else(|_| "konsole".to_owned())
        } else {
            settings.command
        };

        Self {
            prefix: settings.prefix,
            command,
            outcome: None,
        }
    }

    fn split<'a>(&self, query: &'a str) -> Option<&'a str> {
        let (prefix, rest) = query.trim().split_once(char::is_whitespace)?;

        if prefix != self.prefix {
            return None;
        }

        let rest = rest.trim();

        (!rest.is_empty()).then_some(rest)
    }
}

impl Plugin for Terminal {
    fn name(&self) -> &'static str {
        "terminal"
    }

    fn accepts(&self, query: &str) -> bool {
        self.split(query).is_some()
    }

    fn isolates(&self, query: &str) -> bool {
        self.split(query).is_some()
    }

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        let Some(cmd) = self.split(query) else {
            self.outcome = None;
            return Vec::new();
        };

        self.outcome = Some(cmd.to_owned());
        vec![PluginSearchResult {
            id: 0,
            name: cmd.to_owned(),
            description: format!("Run in {}", self.command),
            keywords: None,
            icon: Some(IconSource::Name("utilities-terminal".to_owned())),
            exec: None,
            window: None,
        }]
    }

    fn activate(&mut self, _id: Indice) -> Vec<PluginResponse> {
        let Some(cmd) = self.outcome.as_deref() else {
            return Vec::new();
        };

        // -e is the one flag konsole, xterm, alacritty, and foot agree on,
        // and sh -c is what lets the command carry its own arguments
        detached(&self.command, &["-e", "sh", "-c", cmd]);

        vec![PluginResponse::Close]
    }

    fn usage(&self) -> Vec<Usage> {
        vec![Usage {
            prefix: self.prefix.clone(),
            example: format!("{} htop", self.prefix),
            description: format!("Run a command in {}", self.command),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::{Settings, Terminal};
    use fh_service::Plugin;

    fn terminal() -> Terminal {
        Terminal::new(Settings {
            prefix: "t".into(),
            command: "konsole".into(),
        })
    }

    #[test]
    fn the_prefix_is_stripped_from_the_command() {
        let mut terminal = terminal();
        let results = terminal.search("t htop");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "htop");
    }

    #[test]
    fn arguments_survive_the_split() {
        let mut terminal = terminal();
        let results = terminal.search("t journalctl --user -f");

        assert_eq!(results[0].name, "journalctl --user -f");
    }

    #[test]
    fn the_prefix_alone_is_not_a_command() {
        let terminal = terminal();

        assert!(!terminal.accepts("t"));
        assert!(!terminal.accepts("terminal thing"));
        assert!(terminal.accepts("t htop"));
    }
}

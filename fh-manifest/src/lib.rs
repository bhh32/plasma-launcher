use fh_service::{Trigger, Usage};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use tracing::warn;

const FILE: &str = "plugin.toml";

// What a plugin.toml declares. Everything the launcher needs to decide
// whether to wake a plugin, and describe it in help, without running it.
#[derive(Debug, Deserialize)]
struct File {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon: Option<String>,
    bin: String,
    #[serde(default)]
    query: Query,
    #[serde(default)]
    usage: Vec<UsageEntry>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Query {
    prefixes: Vec<String>,
    starts_with: Vec<char>,
    digits: bool,
    isolate: Vec<String>,
    always: bool,
}

#[derive(Debug, Deserialize)]
struct UsageEntry {
    #[serde(default)]
    prefix: String,
    example: String,
    description: String,
}

#[derive(Debug)]
pub struct Manifest {
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub command: PathBuf,
    pub trigger: Trigger,
    pub usage: Vec<Usage>,
}

// User plugins shadow bundled ones of the same directory name.
pub fn discover() -> Vec<Manifest> {
    let mut found: BTreeMap<String, Manifest> = BTreeMap::new();

    // plugin_dirs lists the user directory first; reversing it means the
    // user's copy is inserted last and wins.
    for dir in fh_paths::plugin_dirs().into_iter().rev() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(key) = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
            else {
                continue;
            };
            if let Some(manifest) = read(&path) {
                found.insert(key, manifest);
            }
        }
    }
    found.into_values().collect()
}

fn read(dir: &Path) -> Option<Manifest> {
    let path = dir.join(FILE);
    if !path.exists() {
        return None;
    }

    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            warn!(%e, path = %path.display(), "could not read manifest");
            return None;
        }
    };
    let file: File = match toml::from_str(&text) {
        Ok(file) => file,
        Err(e) => {
            warn!(%e, path = %path.display(), "could not parse manifest");
            return None;
        }
    };
    // A bare name is relative to the plugin's own directory, so a plugin is
    // one self-contained directory that can be moved or copied.
    let command = if file.bin.starts_with('/') {
        PathBuf::from(&file.bin)
    } else {
        dir.join(&file.bin)
    };
    if !command.exists() {
        warn!(command = %command.display(), "manifest names a binary that is not there");
        return None;
    }

    Some(Manifest {
        name: file.name,
        description: file.description,
        icon: file.icon,
        command,
        trigger: Trigger {
            prefixes: file.query.prefixes,
            starts_with: file.query.starts_with,
            digits: file.query.digits,
            isolate: file.query.isolate,
            always: file.query.always,
        },
        usage: file
            .usage
            .into_iter()
            .map(|entry| Usage {
                prefix: entry.prefix,
                example: entry.example,
                description: entry.description,
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::{File, Query};

    #[test]
    fn a_min_manifest_parses() {
        let file: File = toml::from_str(
            r#"
            name = "Calculator"
            bin = "calc"
            "#,
        )
        .expect("parses");

        assert_eq!(file.name, "Calculator");
        assert_eq!(file.bin, "calc");
        assert!(file.usage.is_empty());
        assert!(!file.query.always);
    }

    #[test]
    fn a_query_section_becomes_a_trigger() {
        let file: File = toml::from_str(
            r#"
            name = "Calculator"
            bin = "calc"

            [query]
            starts_with = ["=", "(", "-", "."]
            digits = true
            isolate = ["="]
            "#,
        )
        .expect("parses");

        assert_eq!(file.query.starts_with, vec!['=', '(', '-', '.']);
        assert!(file.query.digits);
        assert_eq!(file.query.isolate, vec!["="]);
    }

    #[test]
    fn usage_entries_are_a_list() {
        let file: File = toml::from_str(
            r#"
            name = "Web"
            bin = "web"

            [[usage]]
            prefix = "g"
            example = "g <terms>"
            description = "google.com"

            [[usage]]
            prefix = "gh"
            example = "gh <path>"
            description = "github.com"
            "#,
        )
        .expect("parses");

        assert_eq!(file.usage.len(), 2);
        assert_eq!(file.usage[1].prefix, "gh");
    }

    #[test]
    fn a_missing_name_is_an_err_rather_than_a_default() {
        assert!(toml::from_str::<File>("bin = \"calc\"").is_err());
    }

    #[test]
    fn an_unknown_key_is_ignored() {
        // Forward compat: an older launcher reading a newer manifest
        let file: File = toml::from_str(
            r#"
            name = "Calculator"
            bin = "calc"
            future_option = 3
            "#,
        )
        .expect("parses");

        assert_eq!(file.name, "Calculator");
    }

    #[test]
    fn default_query_accepts_nothing() {
        assert!(!Query::default().always);
    }
}

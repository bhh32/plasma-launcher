use fh_paths::plugin_dirs;
use fh_service::{Trigger, Usage};
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
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

impl From<Query> for Trigger {
    fn from(query: Query) -> Self {
        Self {
            prefixes: query.prefixes,
            starts_with: query.starts_with,
            digits: query.digits,
            isolate: query.isolate,
            always: query.always,
        }
    }
}

// The [plugins] table of the launcher's own config, where a user retargets a
// plugin without write access to an installed manifest. Every field is
// optional so an override changes only what it names; adding a prefix leaves
// the manifest's starts_with and digits alone.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct Override {
    prefixes: Option<Vec<String>>,
    starts_with: Option<Vec<char>>,
    digits: Option<bool>,
    isolate: Option<Vec<String>>,
    always: Option<bool>,
}

impl Override {
    // A section naming no trigger field is not an override. The launcher
    // config also carries plugin settings such as [plugins.web.keywords].
    fn is_empty(&self) -> bool {
        self.prefixes.is_none()
            && self.starts_with.is_none()
            && self.digits.is_none()
            && self.isolate.is_none()
            && self.always.is_none()
    }

    fn apply(self, trigger: &mut Trigger) {
        if let Some(prefixes) = self.prefixes {
            trigger.prefixes = prefixes;
        }
        if let Some(starts_with) = self.starts_with {
            trigger.starts_with = starts_with;
        }
        if let Some(digits) = self.digits {
            trigger.digits = digits;
        }
        if let Some(isolate) = self.isolate {
            trigger.isolate = isolate;
        }
        if let Some(always) = self.always {
            trigger.always = always;
        }
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Overrides {
    plugins: BTreeMap<String, Override>,
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

    let mut overrides = overrides();
    for manifest in found.values_mut() {
        if let Some(over) = overrides.remove(&manifest.name) {
            over.apply(&mut manifest.trigger);
        }
    }
    found.into_values().collect()
}

// The file a user edits to retarget a plugin. Exposed so the launcher can
// watch it and re-register when it moves.
pub fn config_path() -> PathBuf {
    fh_paths::config_dir().join("config.toml")
}

pub fn stamp() -> Option<SystemTime> {
    let mut newest = modified(&config_path());
    plugin_dirs().iter().for_each(|dir| {
        // A plugin added or removed changes the directory it lives in
        newest = newest.max(modified(dir));
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };

        entries.filter_map(Result::ok).for_each(|entry| {
            newest = newest.max(modified(&entry.path()));
        });
    });
    newest
}

fn modified(path: &Path) -> Option<SystemTime> {
    fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

fn overrides() -> BTreeMap<String, Override> {
    let path = config_path();
    let Ok(text) = fs::read_to_string(&path) else {
        return BTreeMap::new();
    };
    match toml::from_str::<Overrides>(&text) {
        Ok(o) => o
            .plugins
            .into_iter()
            .filter(|(_, over)| !over.is_empty())
            .collect(),
        Err(e) => {
            warn!(%e, path = %path.display(), "could not parse plugin overrides");
            BTreeMap::new()
        }
    }
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
        trigger: file.query.into(),
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
    use super::{File, Override, Overrides, Query};
    use fh_service::Trigger;

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

    #[test]
    fn an_override_changes_only_what_it_names() {
        let o: Overrides = toml::from_str(
            r#"
            [plugins.calculator]
            prefixes = ["calc"]
            "#,
        )
        .expect("parses");

        // The manifest's trigger, as the calculator ships it
        let mut trigger = Trigger {
            starts_with: vec!['=', '(', '-', '.'],
            digits: true,
            isolate: vec!["=".to_owned()],
            ..Trigger::default()
        };
        o.plugins["calculator"].clone().apply(&mut trigger);

        // The new prefix works and the original triggers still do
        assert!(trigger.accepts("calc 36*3"));
        assert!(trigger.accepts("36*3"));
        assert!(trigger.accepts("= 2+2"));
        assert!(trigger.isolates("= 2+2"));
    }

    #[test]
    fn a_named_field_replaces_rather_than_appends() {
        let over = Override {
            digits: Some(false),
            ..Override::default()
        };
        let mut trigger = Trigger {
            digits: true,
            ..Trigger::default()
        };
        over.apply(&mut trigger);

        assert!(!trigger.digits);
    }

    #[test]
    fn plugin_settings_are_not_an_override() {
        // The launcher config carries these too; they name no trigger, so
        // they must not wipe what the manifest declared
        let o: Overrides = toml::from_str(
            r#"
            [plugins.web.keywords]
            g = "https://google.com/search?q={}"
            "#,
        )
        .expect("parses");

        assert!(o.plugins["web"].is_empty());
    }
}

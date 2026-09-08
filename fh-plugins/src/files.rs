use crate::{paths::expand, spawn::detached};
use fh_ipc::{IconSource, Indice, PluginResponse, PluginSearchResult};
use fh_service::{Plugin, Usage};
use std::{fs, path::PathBuf};

const MAX_RESULTS: usize = 50;

struct Entry {
    path: PathBuf,
    directory: bool,
}

#[derive(Default)]
pub struct Files {
    entries: Vec<Entry>,
}

impl Files {
    fn divide(query: &str) -> (&str, &str) {
        match query.rsplit_once('/') {
            Some(("", tail)) => ("/", tail),
            Some((head, tail)) => (head, tail),
            None => (query, ""),
        }
    }
}

impl Plugin for Files {
    fn name(&self) -> &'static str {
        "files"
    }

    fn accepts(&self, query: &str) -> bool {
        let query = query.trim_start();
        query.starts_with('/') || query.starts_with('~')
    }

    fn isolates(&self, query: &str) -> bool {
        self.accepts(query)
    }

    fn usage(&self) -> Vec<Usage> {
        vec![Usage {
            prefix: "~/".into(),
            example: "~/Doc".into(),
            description: "Browse paths, Enter descends".into(),
        }]
    }

    fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
        self.entries.clear();

        let (dir, partial) = Self::divide(query.trim());
        let partial = partial.to_lowercase();
        let Ok(reader) = fs::read_dir(expand(dir)) else {
            return Vec::new();
        };
        let mut entries: Vec<Entry> = reader
            .filter_map(Result::ok)
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .to_lowercase()
                    .starts_with(&partial)
            })
            .map(|entry| Entry {
                directory: entry.path().is_dir(),
                path: entry.path(),
            })
            .collect();

        // Directories first, then by name
        entries.sort_by(|left, right| {
            right
                .directory
                .cmp(&left.directory)
                .then_with(|| left.path.file_name().cmp(&right.path.file_name()))
        });
        entries.truncate(MAX_RESULTS);

        self.entries = entries;

        self.entries
            .iter()
            .enumerate()
            .map(|(id, entry)| {
                let name = entry.path.file_name().unwrap_or_default().to_string_lossy();

                PluginSearchResult {
                    id: id as Indice,
                    name: if entry.directory {
                        format!("{name}/")
                    } else {
                        name.into_owned()
                    },
                    description: entry.path.to_string_lossy().into_owned(),
                    keywords: None,
                    icon: Some(IconSource::Name(
                        if entry.directory {
                            "folder"
                        } else {
                            "text-x-generic"
                        }
                        .to_owned(),
                    )),
                    exec: None,
                    window: None,
                }
            })
            .collect()
    }

    fn activate(&mut self, id: Indice) -> Vec<PluginResponse> {
        let Some(entry) = self.entries.get(id as usize) else {
            return Vec::new();
        };

        // Entering directory continues the navigation rather than ending it
        if entry.directory {
            return vec![PluginResponse::Fill(format!(
                "{}/",
                entry.path.to_string_lossy()
            ))];
        }

        detached("xdg-open", &[&entry.path.to_string_lossy()]);
        vec![PluginResponse::Close]
    }

    fn complete(&mut self, id: Indice) -> Option<String> {
        let entry = self.entries.get(id as usize)?;
        let path = entry.path.to_string_lossy().into_owned();

        Some(if entry.directory {
            format!("{path}/")
        } else {
            path
        })
    }
}

#[cfg(test)]
mod tests {
    use super::Files;

    #[test]
    fn the_last_segment_is_the_prefix_being_typed() {
        assert_eq!(Files::divide("~/Doc"), ("~", "Doc"));
        assert_eq!(Files::divide("~/Documents/"), ("~/Documents", ""));
        assert_eq!(Files::divide("/usr/bi"), ("/usr", "bi"));
        assert_eq!(Files::divide("~"), ("~", ""));
    }
}

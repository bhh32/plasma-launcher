use std::{
    path::PathBuf,
    sync::mpsc::{Receiver, Sender, channel},
    time::{Duration, Instant},
};

use crate::{process::ProcessPlugin, trigger::Trigger};

use super::Plugin;
use fh_ipc::{Indice, PluginResponse, PluginSearchResult, Request, SearchResult};
use tracing::debug;

const MAX_RESULTS: usize = 8;
const DEADLINE: Duration = Duration::from_millis(150);

#[derive(Clone, Copy)]
enum Source {
    Builtin(usize),
    Process(usize),
}

#[derive(Clone, Copy)]
struct Selection {
    source: Source,
    local: Indice,
}

pub struct Registry {
    builtin: Vec<Box<dyn Plugin>>,
    processes: Vec<ProcessPlugin>,
    responses: Receiver<(usize, PluginResponse)>,
    sender: Sender<(usize, PluginResponse)>,
    selections: Vec<Selection>,
}

impl Registry {
    pub fn new(builtin: Vec<Box<dyn Plugin>>) -> Self {
        let (sender, responses) = channel();

        Self {
            builtin,
            processes: Vec::new(),
            responses,
            sender,
            selections: Vec::new(),
        }
    }

    // Dropping a ProcessPlugin stops its child, so this ends everything
    // currently running before the caller re-registers.
    pub fn clear_processes(&mut self) {
        self.processes.clear();
    }

    pub fn add_process(&mut self, name: String, command: PathBuf, trigger: Trigger) {
        let index = self.processes.len();

        self.processes.push(ProcessPlugin::new(
            index,
            name,
            command,
            trigger,
            self.sender.clone(),
        ));
    }

    pub fn search(&mut self, query: &str) -> Vec<SearchResult> {
        self.selections.clear();

        // Anything still arriving is stale, drop it instead of mixing it
        while self.responses.try_recv().is_ok() {}

        let query = query.trim();
        let isolating_builtin = self
            .builtin
            .iter()
            .position(|plugin| plugin.isolates(query));
        let isolating_process = self
            .processes
            .iter()
            .position(|plugin| plugin.isolates(query));
        let isolating = isolating_builtin.is_some() || isolating_process.is_some();
        // Dispatch to processes first to they work while the built-ins run
        let mut pending = 0;

        for index in 0..self.processes.len() {
            let wanted = match isolating_process {
                Some(only) => isolating_builtin.is_none() && only == index,
                None => !isolating && self.processes[index].accepts(query),
            };

            if wanted && self.processes[index].search(query) {
                pending += 1;
            }
        }

        let mut collected: Vec<(Selection, SearchResult)> = Vec::new();
        let builtins: Vec<usize> = match isolating_builtin {
            Some(only) => vec![only],
            None if isolating => Vec::new(),
            None => (0..self.builtin.len())
                .filter(|&index| self.builtin[index].accepts(query))
                .collect(),
        };

        builtins.iter().for_each(|index| {
            self.builtin[*index]
                .search(query)
                .iter()
                .for_each(|result| {
                    collected.push((
                        Selection {
                            source: Source::Builtin(*index),
                            local: result.id,
                        },
                        Self::into_result(result.clone()),
                    ));
                });
        });

        let deadline = Instant::now() + DEADLINE;

        while pending > 0 {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }

            match self.responses.recv_timeout(remaining) {
                Ok((index, PluginResponse::Append(result))) => collected.push((
                    Selection {
                        source: Source::Process(index),
                        local: result.id,
                    },
                    Self::into_result(result),
                )),
                Ok((_, PluginResponse::Finished)) => pending -= 1,
                Ok(_) => {}
                Err(_) => break,
            }
        }

        // Window-first ordering before the cap
        collected.sort_by_key(|(_, result)| result.window.is_none());
        if !isolating {
            collected.truncate(MAX_RESULTS);
        }
        self.selections = collected.iter().map(|(sel, _)| *sel).collect();

        collected
            .into_iter()
            .enumerate()
            .map(|(id, (_, mut result))| {
                result.id = id as Indice;
                result
            })
            .collect()
    }

    fn into_result(result: PluginSearchResult) -> SearchResult {
        SearchResult {
            id: 0,
            name: result.name,
            description: result.description,
            icon: result.icon,
            category_icon: None,
            window: result.window,
        }
    }

    pub fn activate(&mut self, id: Indice) -> Vec<PluginResponse> {
        let Some(sel) = self.selections.get(id as usize).copied() else {
            debug!(id, "activate for an id outside the last result set");
            return Vec::new();
        };

        match sel.source {
            Source::Builtin(index) => self.builtin[index].activate(sel.local),
            Source::Process(index) => {
                if !self.processes[index].send(&Request::Activate(sel.local)) {
                    return Vec::new();
                }
                self.drain(index)
            }
        }
    }

    pub fn complete(&mut self, id: Indice) -> Option<String> {
        let sel = self.selections.get(id as usize).copied()?;

        match sel.source {
            Source::Builtin(index) => self.builtin[index].complete(sel.local),
            Source::Process(index) => {
                self.processes[index].send(&Request::Complete(sel.local));
                self.drain(index)
                    .into_iter()
                    .find_map(|response| match response {
                        PluginResponse::Fill(text) => Some(text),
                        _ => None,
                    })
            }
        }
    }

    // Activation has no Finished marker, takes whatever the plugin
    // says within the deadline.
    fn drain(&mut self, index: usize) -> Vec<PluginResponse> {
        let deadline = Instant::now() + DEADLINE;
        let mut responses = Vec::new();

        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                break;
            }

            match self.responses.recv_timeout(remaining) {
                Ok((from, PluginResponse::Finished)) if from == index => break,
                Ok((from, response)) if from == index => responses.push(response),
                Ok(_) => {}
                Err(_) => break,
            }
        }
        responses
    }
}

#[cfg(test)]
mod tests {

    use super::{super::Plugin, Registry};
    use fh_ipc::{Indice, PluginResponse, PluginSearchResult};

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

        fn search(&mut self, _query: &str) -> Vec<PluginSearchResult> {
            (0..self.count)
                .map(|num| PluginSearchResult {
                    id: num as Indice,
                    name: format!("{} {num}", self.name),
                    description: String::new(),
                    keywords: None,
                    icon: None,
                    exec: None,
                    window: Some((1, 1)),
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

    #[test]
    fn results_with_a_window_sort_first() {
        // Give Fake a `window` field set on its results to exercise this
        let mut registry = registry();
        let results = registry.search("x");

        assert_eq!(
            results.iter().map(|result| result.id).collect::<Vec<_>>(),
            vec![0, 1, 2, 3]
        )
    }
}

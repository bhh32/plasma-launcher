use crate::Trigger;
use fh_ipc::{PluginResponse, Request, decode_line, encode_line};
use std::{
    io::{BufRead, BufReader, Write},
    path::PathBuf,
    process::{Child, ChildStdin, Command, Stdio},
    sync::mpsc::Sender,
    thread,
};
use tracing::warn;

pub struct ProcessPlugin {
    index: usize,
    name: String,
    command: PathBuf,
    trigger: Trigger,
    sender: Sender<(usize, PluginResponse)>,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
}

impl ProcessPlugin {
    pub fn new(
        index: usize,
        name: String,
        command: PathBuf,
        trigger: Trigger,
        sender: Sender<(usize, PluginResponse)>,
    ) -> Self {
        Self {
            index,
            name,
            command,
            trigger,
            sender,
            child: None,
            stdin: None,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn accepts(&self, query: &str) -> bool {
        self.trigger.accepts(query)
    }

    pub fn isolates(&self, query: &str) -> bool {
        self.trigger.isolates(query)
    }

    // False: plugin could not be reached after a restart and caller should not count
    // it as pending
    // Routing is the launcher's business, so the prefix that selected this
    // plugin is removed before the query reaches it.
    pub fn search(&mut self, query: &str) -> bool {
        let query = self.trigger.strip(query).to_owned();
        self.send(&Request::Search(query))
    }

    pub fn send(&mut self, request: &Request) -> bool {
        if self.stdin.is_none() && !self.start() {
            return false;
        }
        if self.write(request) {
            return true;
        }
        // A plugin that died between searches is ordinary, so one restart isn't worth
        // logging as an error.
        self.stop();

        self.start() & self.write(request)
    }

    fn write(&mut self, request: &Request) -> bool {
        let Some(stdin) = self.stdin.as_mut() else {
            return false;
        };
        let Ok(line) = encode_line(request) else {
            return false;
        };
        stdin
            .write_all(line.as_bytes())
            .and_then(|()| stdin.flush())
            .is_ok()
    }

    fn start(&mut self) -> bool {
        // stderr is inherited so a plugin's own logging lands in the journal
        // beside the launcher's
        let spawned = Command::new(&self.command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn();

        let mut child = match spawned {
            Ok(child) => child,
            Err(e) => {
                warn!(%e, plugin = %self.name, "could not start plugin");
                return false;
            }
        };
        let Some(stdout) = child.stdout.take() else {
            return false;
        };
        let sender = self.sender.clone();
        let index = self.index;

        thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else {
                    break;
                };
                let Ok(response) = decode_line::<PluginResponse>(&line) else {
                    continue;
                };

                if sender.send((index, response)).is_err() {
                    break;
                }
            }
        });

        self.stdin = child.stdin.take();
        self.child = Some(child);
        true
    }

    fn stop(&mut self) {
        // Dropping stdin closes the pipe, which is how the plugins learn to exit.
        self.stdin = None;

        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for ProcessPlugin {
    fn drop(&mut self) {
        self.stop();
    }
}

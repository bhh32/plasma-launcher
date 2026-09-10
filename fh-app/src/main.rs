use color_eyre::Result;
use fh_ipc::{Error as IpcError, PluginResponse, Request, Response, decode_line, encode_line};
use fh_plugins::{DesktopEntries, Files, Help, Settings, Topic, Web};
use fh_service::{Plugin, Registry};
use std::{
    io::{BufRead, Write, stderr, stdin, stdout},
    sync::mpsc::channel,
    thread,
    time::SystemTime,
};
use tracing::{debug, info, warn};

enum Event {
    Line(String),
    Wake,
    Eof,
}

fn main() -> Result<()> {
    color_eyre::install()?;

    // stdout carries the protocol. Every diagnostic goes to stderr or the
    // stream is corrupted for the frontend
    tracing_subscriber::fmt().with_writer(stderr).init();

    let settings = Settings::load();

    let web = Web::new(settings.web);
    let files = Files::default();
    let desktop = DesktopEntries::load();
    let manifests = fh_manifest::discover();
    let mut topics = vec![Topic::of(&web), Topic::of(&files), Topic::of(&desktop)];

    // Add the user plugins to the help topics
    topics.extend(
        manifests
            .iter()
            .map(|manifest| Topic::new(manifest.name.clone(), manifest.usage.clone())),
    );

    let help = Help::new(topics);

    let plugins: Vec<Box<dyn Plugin>> = vec![
        Box::new(web),
        Box::new(help),
        Box::new(files),
        Box::new(desktop),
    ];

    let (events, incoming) = channel();
    let (wake, wakes) = channel();

    // Reading stdin blocks off the main loop
    {
        let events = events.clone();

        thread::spawn(move || {
            for line in stdin().lock().lines() {
                let Ok(line) = line else {
                    break;
                };
                if events.send(Event::Line(line)).is_err() {
                    return;
                }
            }
            let _ = events.send(Event::Eof);
        });
    }

    thread::spawn(move || {
        while wakes.recv().is_ok() {
            if events.send(Event::Wake).is_err() {
                return;
            }
        }
    });

    let mut registry = Registry::new(plugins, wake);
    let mut stamp = config_stamp();
    register_plugins(&mut registry);
    let mut stdout = stdout().lock();
    let mut last = String::new();

    for event in incoming {
        let line = match event {
            Event::Line(line) => line,
            Event::Eof => break,
            Event::Wake => {
                if last.is_empty() {
                    continue;
                }
                let results = registry.search(&last);
                respond(&mut stdout, &Response::Update(results))?;
                continue;
            }
        };

        let request = match decode_line::<Request>(&line) {
            Ok(request) => request,
            Err(IpcError::Empty) => continue,
            Err(e) => {
                warn!(%e, "discarding message");
                continue;
            }
        };

        match request {
            Request::Search(query) => {
                // Trigger overrides live in the launcher's config, so an edit
                // takes effect on the next keystroke rather than a restart
                let current = config_stamp();
                if current != stamp {
                    stamp = current;
                    registry.clear_processes();
                    register_plugins(&mut registry);
                }
                let results = registry.search(&query);
                last = query;
                respond(&mut stdout, &Response::Update(results))?;
            }
            Request::Activate(id) => {
                for response in registry.activate(id) {
                    forward(&mut stdout, response)?;
                }
            }
            Request::Complete(id) => {
                if let Some(text) = registry.complete(id) {
                    respond(&mut stdout, &Response::Fill(text))?;
                }
            }
            Request::Exit => break,
            Request::Interrupt => {}
            Request::Context(_) | Request::ActivateContext { .. } | Request::Quit(_) => {
                debug!("context and window requests are not served yet");
            }
        }
    }

    Ok(())
}

fn forward<W: Write>(out: &mut W, response: PluginResponse) -> Result<()> {
    // Append, Clear, and Finished belong to the streaming search protocol an
    // out-of-process plugin speaks. Activation never produces them
    let response = match response {
        PluginResponse::Close => Response::Close,
        PluginResponse::Fill(text) => Response::Fill(text),
        PluginResponse::DesktopEntry {
            path,
            gpu_preference,
        } => Response::DesktopEntry {
            path,
            gpu_preference,
        },
        PluginResponse::Context { id, options } => Response::Context { id, options },
        other => {
            debug!(?other, "plugin response not valid for activation");
            return Ok(());
        }
    };

    respond(out, &response)
}

fn register_plugins(registry: &mut Registry) {
    for manifest in fh_manifest::discover() {
        info!(plugin = %manifest.name, "registered plugin");
        registry.add_process(manifest.name, manifest.command, manifest.trigger);
    }
}

fn config_stamp() -> Option<SystemTime> {
    std::fs::metadata(fh_manifest::config_path())
        .and_then(|meta| meta.modified())
        .ok()
}

fn respond<W: Write>(out: &mut W, response: &Response) -> Result<()> {
    out.write_all(encode_line(response)?.as_bytes())?;

    // Piped stdout is block buffered, and the frontend is blocked on a line.
    out.flush()?;

    Ok(())
}

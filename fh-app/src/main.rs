use color_eyre::Result;
use fh_ipc::{Error as IpcError, PluginResponse, Request, Response, decode_line, encode_line};
use fh_plugins::{Calculator, DesktopEntries, Files, Find, Help, Settings, Terminal, Topic, Web};
use fh_service::{Plugin, Registry};
use std::io::{BufRead, Write, stderr, stdin, stdout};
use tracing::{debug, info, warn};

fn main() -> Result<()> {
    color_eyre::install()?;

    // stdout carries the protocol. Every diagnostic goes to stderr or the
    // stream is corrupted for the frontend
    tracing_subscriber::fmt().with_writer(stderr).init();

    let settings = Settings::load();

    let calculator = Calculator::default();
    let web = Web::new(settings.web);
    let terminal = Terminal::new(settings.terminal);
    let files = Files::default();
    let find = Find::new(settings.find);
    let desktop = DesktopEntries::load();
    let help = Help::new(vec![
        Topic::of(&calculator),
        Topic::of(&web),
        Topic::of(&terminal),
        Topic::of(&files),
        Topic::of(&find),
        Topic::of(&desktop),
    ]);

    let plugins: Vec<Box<dyn Plugin>> = vec![
        Box::new(calculator),
        Box::new(web),
        Box::new(terminal),
        Box::new(help),
        Box::new(files),
        Box::new(find),
        Box::new(desktop),
    ];

    let mut registry = Registry::new(plugins);
    for manifest in fh_manifest::discover() {
        info!(plugin = %manifest.name, "registered plugin");
        registry.add_process(manifest.name, manifest.command, manifest.trigger);
    }
    let mut stdout = stdout().lock();

    for line in stdin().lock().lines() {
        let line = line?;
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
                let results = registry.search(&query);
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

fn respond<W: Write>(out: &mut W, response: &Response) -> Result<()> {
    out.write_all(encode_line(response)?.as_bytes())?;

    // Piped stdout is block buffered, and the frontend is blocked on a line.
    out.flush()?;

    Ok(())
}

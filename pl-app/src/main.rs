mod index;
mod search;

use crate::search::Searcher;
use color_eyre::Result;
use pl_ipc::{decode_line, encode_line, Error as IpcError, GpuPreference, Request, Response};
use std::io::{stderr, stdin, stdout, BufRead, Write};
use tracing::{debug, info, warn};

fn main() -> Result<()> {
    color_eyre::install()?;

    // stdout carries the protocol. Every diagnostic goes to stderr or the
    // stream is corrupted for the frontend
    tracing_subscriber::fmt().with_writer(stderr).init();

    let entries = index::load();
    info!(count = entries.len(), "indexed desktop entries");

    let mut searcher = Searcher::new(entries);
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
                let results = searcher.search(&query);
                respond(&mut stdout, &Response::Update(results))?;
            }
            Request::Activate(id) => {
                let Some(entry) = searcher.resolve(id) else {
                    warn!(id, "activate for an id outside the last result set");
                    continue;
                };
                let gpu_preference = if entry.prefers_non_default_gpu {
                    GpuPreference::NonDefault
                } else {
                    GpuPreference::Default
                };
                let path = entry.path.clone();

                respond(
                    &mut stdout,
                    &Response::DesktopEntry {
                        path,
                        gpu_preference,
                    },
                )?;
                respond(&mut stdout, &Response::Close)?;
            }
            Request::Complete(id) => {
                let Some(entry) = searcher.resolve(id) else {
                    continue;
                };
                respond(&mut stdout, &Response::Fill(entry.name.clone()))?;
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

fn respond<W: Write>(out: &mut W, response: &Response) -> Result<()> {
    out.write_all(encode_line(response)?.as_bytes())?;

    // Piped stdout is block buffered, and the frontend is blocked on a line.
    out.flush()?;

    Ok(())
}

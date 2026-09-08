use fh_ipc::{
    ContextOption, Indice, PluginResponse, PluginSearchResult, Request, decode_line, encode_line,
};
use std::io::{self, BufRead, Write};

pub trait Source {
    fn search(&mut self, query: &str) -> Vec<PluginSearchResult>;
    fn activate(&mut self, id: Indice) -> Vec<PluginResponse>;
    fn complete(&mut self, _id: Indice) -> Option<String> {
        None
    }
    fn context(&mut self, _id: Indice) -> Vec<ContextOption> {
        Vec::new()
    }
    fn activate_context(&mut self, _id: Indice, _context: Indice) -> Vec<PluginResponse> {
        Vec::new()
    }
    fn quit(&mut self, _id: Indice) -> Vec<PluginResponse> {
        Vec::new()
    }
    // Called when the launcher abandons a search.
    fn interrupt(&mut self) {}
}

pub fn log_to_stderr() {
    let _ = tracing_subscriber::fmt().with_writer(io::stderr).try_init();
}

pub fn run<S: Source>(source: S) {
    let stdin = io::stdin();
    let stdout = io::stdout();

    serve(source, stdin.lock(), stdout.lock());
}

pub fn serve<S: Source, R: BufRead, W: Write>(mut source: S, reader: R, mut writer: W) {
    for line in reader.lines() {
        let Ok(line) = line else {
            return;
        };
        let Ok(request) = decode_line::<Request>(&line) else {
            continue;
        };
        let responses = match request {
            Request::Search(query) => {
                let found = source.search(&query);
                let mut responses: Vec<PluginResponse> =
                    found.into_iter().map(PluginResponse::Append).collect();

                // Finished is what lets the launcher stop waiting on this
                // plugin before its deadline expires
                responses.push(PluginResponse::Finished);
                responses
            }
            Request::Activate(id) => source.activate(id),
            Request::Complete(id) => source
                .complete(id)
                .map(PluginResponse::Fill)
                .into_iter()
                .collect(),
            Request::Context(id) => {
                let options = source.context(id);
                if options.is_empty() {
                    Vec::new()
                } else {
                    vec![PluginResponse::Context { id, options }]
                }
            }
            Request::ActivateContext { id, context } => source.activate_context(id, context),
            Request::Quit(id) => source.quit(id),
            Request::Interrupt => {
                source.interrupt();
                Vec::new()
            }
            Request::Exit => return,
        };

        for response in responses {
            if emit(&mut writer, &response).is_err() {
                return;
            }
        }
    }
}

fn emit<W: Write>(writer: &mut W, response: &PluginResponse) -> io::Result<()> {
    let line = encode_line(response).map_err(io::Error::other)?;
    writer.write_all(line.as_bytes())?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::{Source, serve};
    use fh_ipc::{ContextOption, IconSource, Indice, PluginResponse, PluginSearchResult};
    use std::io::Cursor;

    #[derive(Default)]
    struct Fake {
        interrupted: bool,
    }

    impl Fake {
        fn result(id: Indice, name: &str) -> PluginSearchResult {
            PluginSearchResult {
                id,
                name: name.to_owned(),
                description: String::new(),
                keywords: None,
                icon: Some(IconSource::Name("x".to_owned())),
                exec: None,
                window: None,
            }
        }
    }

    impl Source for Fake {
        fn search(&mut self, query: &str) -> Vec<PluginSearchResult> {
            if query == "none" {
                return Vec::new();
            }
            vec![Self::result(0, "first"), Self::result(1, "second")]
        }

        fn activate(&mut self, _id: Indice) -> Vec<PluginResponse> {
            vec![PluginResponse::Close]
        }

        fn complete(&mut self, _id: Indice) -> Option<String> {
            Some("completed".to_owned())
        }

        fn context(&mut self, _id: Indice) -> Vec<ContextOption> {
            vec![ContextOption {
                id: 0,
                name: "option".to_owned(),
            }]
        }

        fn interrupt(&mut self) {
            self.interrupted = true;
        }
    }

    fn exchange(input: &str) -> Vec<String> {
        let mut output = Vec::new();
        serve(Fake::default(), Cursor::new(input.to_owned()), &mut output);

        String::from_utf8(output)
            .expect("output is utf8")
            .lines()
            .map(str::to_owned)
            .collect()
    }

    #[test]
    fn a_search_appends_each_result_then_finishes() {
        let lines = exchange("{\"Search\":\"x\"}\n");

        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("\"Append\""));
        assert!(lines[0].contains("first"));
        assert!(lines[1].contains("second"));
        assert_eq!(lines[2], "\"Finished\"");
    }

    #[test]
    fn an_empty_search_still_finishes() {
        // Without this the launcher waits on the deadline for nothing
        assert_eq!(exchange("{\"Search\":\"none\"}\n"), vec!["\"Finished\""]);
    }

    #[test]
    fn exit_stops_the_loop() {
        let lines = exchange("\"Exit\"\n{\"Search\":\"x\"|\n");
        assert!(lines.is_empty());
    }

    #[test]
    fn junk_and_blank_lines_are_skipped_rather_than_fatal() {
        let lines = exchange("\n not json \n{\"Search\":\"none\"}\n");
        assert_eq!(lines, vec!["\"Finished\""]);
    }

    #[test]
    fn complete_emits_a_fill_only_when_there_is_one() {
        assert_eq!(
            exchange("{\"Complete\":0}\n"),
            vec!["{\"Fill\":\"completed\"}"]
        )
    }

    #[test]
    fn context_carries_the_id_it_was_asked_about() {
        let lines = exchange("{\"Context\":7}\n");

        assert_eq!(lines.len(), 1);
        assert!(lines[0].contains("\"id\":7"));
        assert!(lines[0].contains("option"));
    }

    #[test]
    fn interrupt_produces_no_output() {
        assert!(exchange("\"Interrupt\"\n").is_empty());
    }
}

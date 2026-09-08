use crate::search::Indice;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Request {
    Activate(Indice),
    ActivateContext { id: Indice, context: Indice },
    Complete(Indice),
    Context(Indice),
    Exit,
    Interrupt,
    Quit(Indice),
    Search(String),
}

#[cfg(test)]
mod tests {
    use super::Request;
    use crate::codec::{decode_line, encode_line};

    fn wire(request: Request, json: &str) {
        assert_eq!(encode_line(&request).expect("encodes"), format!("{json}\n"));
        assert_eq!(decode_line::<Request>(json).expect("decodes"), request);
    }

    #[test]
    fn every_variant_matches_pop_launcher() {
        wire(Request::Activate(0), r#"{"Activate":0}"#);
        wire(
            Request::ActivateContext { id: 1, context: 2 },
            r#"{"ActivateContext":{"id":1,"context":2}}"#,
        );
        wire(Request::Complete(3), r#"{"Complete":3}"#);
        wire(Request::Context(4), r#"{"Context":4}"#);
        wire(Request::Exit, r#""Exit""#);
        wire(Request::Interrupt, r#""Interrupt""#);
        wire(Request::Quit(5), r#"{"Quit":5}"#);
        wire(Request::Search("fire".into()), r#"{"Search":"fire"}"#);
    }
}

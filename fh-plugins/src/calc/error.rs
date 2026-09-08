use thiserror::Error;

#[derive(Debug, PartialEq, Eq, Error)]
pub enum Error {
    #[error("invalid number {0:?}")]
    InvalidNumber(String),
    #[error("unknown word {0:?}")]
    UnknownWord(String),
    #[error("unexpected character {0:?}")]
    UnexpectedChar(char),
    #[error("unexpected end of expression")]
    UnexpectedEnd,
    #[error("unexpected token")]
    UnexpectedToken,
    #[error("unclosed parenthesis")]
    Unclosed,
}

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("empty message")]
    Empty,
    #[error("malformed message: {0}")]
    Malformed(#[from] serde_json::Error),
}

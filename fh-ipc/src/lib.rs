mod codec;
mod error;
mod request;
mod response;
mod search;

pub use codec::{decode_line, encode_line};
pub use error::Error;
pub use request::Request;
pub use response::{PluginResponse, Response};
pub use search::{
    ContextOption, GpuPreference, IconSource, Indice, PluginSearchResult, SearchResult,
};

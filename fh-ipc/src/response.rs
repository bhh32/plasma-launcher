use crate::search::{ContextOption, GpuPreference, Indice, PluginSearchResult, SearchResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum Response {
    Close,
    Context {
        id: Indice,
        options: Vec<ContextOption>,
    },
    DesktopEntry {
        path: PathBuf,
        gpu_preference: GpuPreference,
    },
    Update(Vec<SearchResult>),
    Fill(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum PluginResponse {
    Append(PluginSearchResult),
    Clear,
    Close,
    Context {
        id: Indice,
        options: Vec<ContextOption>,
    },
    DesktopEntry {
        path: PathBuf,
        gpu_preference: GpuPreference,
    },
    Fill(String),
    Refresh,
    Finished,
}

#[cfg(test)]
mod tests {
    use super::{PluginResponse, Response};
    use crate::{
        codec::{decode_line, encode_line},
        search::{GpuPreference, IconSource, SearchResult},
    };

    #[test]
    fn update_carries_the_whole_list() {
        let response = Response::Update(vec![SearchResult {
            id: 0,
            name: "Firefox".into(),
            description: "Web Browser".into(),
            icon: Some(IconSource::Name("firefox".into())),
            category_icon: None,
            window: None,
        }]);

        assert_eq!(
            encode_line(&response).expect("encodes"),
            "{\"Update\":[{\"id\":0,\"name\":\"Firefox\",\"description\":\"Web Browser\",\
            \"icon\":{\"Name\":\"firefox\"}}]}\n"
        );
    }

    #[test]
    fn close_is_a_bare_string() {
        assert_eq!(
            encode_line(&Response::Close).expect("encodes"),
            "\"Close\"\n"
        );
        assert_eq!(
            decode_line::<PluginResponse>(r#""Finished""#).expect("decodes"),
            PluginResponse::Finished
        );
    }

    #[test]
    fn desktop_entry_paths_round_trip() {
        let json = r#"{"DesktopEntry":{"path":"/usr/share/applications/firefox.desktop","gpu_preference":"Default"}}"#;
        let Response::DesktopEntry {
            path,
            gpu_preference,
        } = decode_line::<Response>(json).expect("decodes")
        else {
            panic!("expected a DesktopEntry response");
        };

        assert_eq!(
            path.to_str(),
            Some("/usr/share/applications/firefox.desktop")
        );
        assert_eq!(gpu_preference, GpuPreference::Default);
    }
}

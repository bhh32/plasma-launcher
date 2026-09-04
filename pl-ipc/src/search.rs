use serde::{Deserialize, Serialize};

pub type Indice = u32;

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum IconSource {
    Name(String),
    Mime(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum GpuPreference {
    Default,
    NonDefault,
    SpecificIdx(u32),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct ContextOption {
    pub id: Indice,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SearchResult {
    pub id: Indice,
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category_icon: Option<IconSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<(u32, u32)>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct PluginSearchResult {
    pub id: Indice,
    pub name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub keywords: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<IconSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exec: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<(u32, u32)>,
}

#[cfg(test)]
mod tests {
    use super::{GpuPreference, IconSource, SearchResult};
    use crate::codec::{decode_line, encode_line};

    #[test]
    fn icon_source_is_external_tagged() {
        let icon = IconSource::Name("firefox".into());
        assert_eq!(
            encode_line(&icon).expect("encodes"),
            "{\"Name\":\"firefox\"}\n"
        );
        assert_eq!(
            decode_line::<IconSource>(r#"{"Mime":"text/plain"}"#).expect("decodes"),
            IconSource::Mime("text/plain".into())
        );
    }

    #[test]
    fn gpu_preference_unit_variants_are_bare_strings() {
        assert_eq!(
            encode_line(&GpuPreference::NonDefault).expect("encodes"),
            "\"NonDefault\"\n"
        );
        assert_eq!(
            encode_line(&GpuPreference::SpecificIdx(1)).expect("encodes"),
            "{\"SpecificIdx\":1}\n"
        );
    }

    #[test]
    fn absent_optional_fields_are_omitted_not_null() {
        let result = SearchResult {
            id: 0,
            name: "Firefox".into(),
            description: "Web Browser".into(),
            icon: None,
            category_icon: None,
            window: None,
        };

        assert_eq!(
            encode_line(&result).expect("encodes"),
            "{\"id\":0,\"name\":\"Firefox\",\"description\":\"Web Browser\"}\n"
        );
    }

    #[test]
    fn window_is_a_two_element_array() {
        let decoded =
            decode_line::<SearchResult>(r#"{"id":0,"name":"a","description":"b","window":[7,9]}"#)
                .expect("decodes");

        assert_eq!(decoded.window, Some((7, 9)));
    }
}

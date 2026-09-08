use fh_ipc::{Indice, PluginResponse, PluginSearchResult};

pub trait Plugin {
    fn name(&self) -> &'static str;
    fn accepts(&self, query: &str) -> bool;
    fn isolates(&self, _query: &str) -> bool {
        false
    }
    fn search(&mut self, query: &str) -> Vec<PluginSearchResult>;
    fn activate(&mut self, id: Indice) -> Vec<PluginResponse>;
    fn complete(&mut self, _id: Indice) -> Option<String> {
        None
    }

    fn usage(&self) -> Vec<Usage> {
        Vec::new()
    }
}

#[derive(Debug, Clone)]
pub struct Usage {
    // Empty for a plugin with no prefix
    pub prefix: String,
    pub example: String,
    pub description: String,
}

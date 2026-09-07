use pl_ipc::{Indice, PluginResponse, PluginSearchResult};

use crate::Ranking;

pub trait Plugin {
    fn name(&self) -> &'static str;
    fn accepts(&self, query: &str) -> bool;
    fn isolates(&self, _query: &str) -> bool {
        false
    }
    fn search(&mut self, query: &str, ranking: &Ranking) -> Vec<PluginSearchResult>;
    fn activate(&mut self, id: Indice) -> Vec<PluginResponse>;
    fn complete(&mut self, _id: Indice) -> Option<String> {
        None
    }
    /// A stable identifier for a result. None means the plugin's results
    /// are not worth ranking.
    fn key(&self, _id: Indice) -> Option<String> {
        None
    }
}

mod kde;

use std::{
    env,
    path::{Path, PathBuf},
};

pub struct Scheme {
    pub base: String,
    pub mantle: String,
    pub surface0: String,
    pub surface1: String,
    pub text: String,
    pub subtext0: String,
    pub subtext1: String,
    pub accent: String,
    pub source: PathBuf,
}

pub fn detect(config_dir: &Path) -> Option<Scheme> {
    let current = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();

    if current.split(':').any(|name| name == "KDE") {
        return kde::scheme(config_dir);
    }

    None
}

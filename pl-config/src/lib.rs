mod desktop;

use serde::{Deserialize, Serialize};
use std::{
    env, fs,
    io::{self, ErrorKind},
    path::PathBuf,
};
use thiserror::Error;
use toml::de;

use crate::desktop::Scheme;

const TEMPLATE: &str = r##"# plasma-launcher configuration
#
# Everything within this configuration file is optional, and if it's missing
# it'll just fall back to the launcher/DE defaults. Below are the accepted
# type of each key.

# [appearance]
# card_width = 600      # integer, pixels
# top_margin = 16       # integer, pixels below the panel
# row_height = 56       # integer, pixels per result
# visible_rows = 8      # integer, rows before the list scrolls
# icon_size = 32        # integer, pixels

# [appearance.radius]
# card = 16             # integer, pixels
# field = 10
# row = 8

# [appearance.font]
# family = ""           # string, empty means the system font
# input = 19            # integer, pixel size
# name = 15
# description = 12

# Colors are `#rrggbb`. While these are absent from the launcher follows
# your DE color scheme; setting one pins it and stops following the DE color
# scheme.
# [appearance.colors]
# base = "#303446"      # card background
# mantle = "#292c3c"    # search field
# surface0 = "#414559"  # borders and the divider
# surface1 = "#51576d"  # selected row
# text = "#c6d0f5"      # result name
# subtext0 = "#a5adce"  # placeholder text
# subtext1 = "#b5bf32"  # result description
# accent = "#babbf1"    # selected row border

# Web keywords map a prefix to a URL template, where {} is replaced by the
# encoded search terms. Uncommenting this section replaces the built-in set
# rather than adding to it, so list every keyword you want.
# [plugins.web.keywords]
# ddg = "https://duckduckgo.com/?q={}"
# g = "https://google.com/search?q={}"
# cb = "https://codeberg.org/{}"
# gh = "https://github.com/{}"
# rs = "https://docs.rs/{}"
# crate = "https://crates.io/crate/{}"
# w = "https://en.wikipedia.org/w/index.php?search={}"

#[plugins.terminal]
# prefix = "t"           # string, the word that triggers this plugin
# command = ""           # string, empty means $TERMINAL then konsole
"##;

#[derive(Debug, Error)]
pub enum Error {
    #[error("could not read {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("could not parse {path}: {source}")]
    Parse { path: PathBuf, source: de::Error },
    #[error("could not encode config as json: {0}")]
    Encode(#[from] serde_json::Error),
    #[error("could not write {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub appearance: Appearance,
    // Output only. The frontend watches these for changes
    #[serde(skip_deserializing)]
    pub paths: Paths,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Paths {
    pub config: PathBuf,
    pub desktop: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Appearance {
    pub card_width: u32,
    pub top_margin: u32,
    pub row_height: u32,
    pub visible_rows: u32,
    pub icon_size: u32,
    pub colors: Colors,
    pub radius: Radius,
    pub font: Font,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Colors {
    pub base: Option<String>,
    pub mantle: Option<String>,
    pub surface0: Option<String>,
    pub surface1: Option<String>,
    pub text: Option<String>,
    pub subtext0: Option<String>,
    pub subtext1: Option<String>,
    pub accent: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Radius {
    pub card: u32,
    pub field: u32,
    pub row: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(default)]
pub struct Font {
    pub family: String,
    pub input: u32,
    pub name: u32,
    pub description: u32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            card_width: 600,
            top_margin: 16,
            row_height: 56,
            visible_rows: 8,
            icon_size: 32,
            colors: Colors::default(),
            radius: Radius::default(),
            font: Font::default(),
        }
    }
}

impl Default for Radius {
    fn default() -> Self {
        Self {
            card: 16,
            field: 10,
            row: 8,
        }
    }
}

impl Default for Font {
    fn default() -> Self {
        Self {
            // Empty means whatever Qt resolves as the system font
            family: String::new(),
            input: 19,
            name: 15,
            description: 12,
        }
    }
}

impl Colors {
    fn resolve(&mut self, scheme: Option<&Scheme>) {
        let Some(scheme) = scheme else {
            return;
        };

        self.base.get_or_insert_with(|| scheme.base.clone());
        self.mantle.get_or_insert_with(|| scheme.mantle.clone());
        self.surface0.get_or_insert_with(|| scheme.surface0.clone());
        self.surface1.get_or_insert_with(|| scheme.surface1.clone());
        self.text.get_or_insert_with(|| scheme.text.clone());
        self.subtext0.get_or_insert_with(|| scheme.subtext0.clone());
        self.subtext1.get_or_insert_with(|| scheme.subtext1.clone());
        self.accent.get_or_insert_with(|| scheme.accent.clone());
    }
}

fn config_dir() -> PathBuf {
    env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .unwrap_or_else(|| PathBuf::from(".config"))
}

impl Config {
    pub fn path() -> PathBuf {
        config_dir().join("plasma-launcher").join("config.toml")
    }

    pub fn load() -> Result<Self, Error> {
        let path = Self::path();

        // No config is normal, not an failure
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(source) => return Err(Error::Read { path, source }),
        };

        let mut config: Self =
            toml::from_str(&text).map_err(|source| Error::Parse { path, source })?;
        config.finish();

        Ok(config)
    }

    pub fn to_json(&self) -> Result<String, Error> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    fn finish(&mut self) {
        let scheme = desktop::detect(&config_dir());
        self.appearance.colors.resolve(scheme.as_ref());
        self.paths = Paths {
            config: Self::path(),
            desktop: scheme.map(|scheme| scheme.source),
        };
    }

    pub fn write_default_if_missing() -> Result<(), Error> {
        let path = Self::path();
        if path.exists() {
            return Ok(());
        }

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|source| Error::Write {
                path: parent.to_path_buf(),
                source,
            })?;
        }

        fs::write(&path, TEMPLATE).map_err(|source| Error::Write { path, source })
    }
}

#[cfg(test)]
mod tests {
    use super::{Appearance, Colors, Config};
    use crate::desktop::Scheme;
    use std::path::PathBuf;

    fn scheme() -> Scheme {
        Scheme {
            base: "#202326".into(),
            mantle: "#141618".into(),
            surface0: "#292c30".into(),
            surface1: "#1e5774".into(),
            text: "#fcfcfc".into(),
            subtext0: "#a1a9b1".into(),
            subtext1: "#a1a9b1".into(),
            accent: "#3daee9".into(),
            source: PathBuf::from("/dev/null"),
        }
    }

    #[test]
    fn an_empty_file_is_all_defaults() {
        let config: Config = toml::from_str("").expect("parses");
        let defaults = Appearance::default();

        assert_eq!(config.appearance.card_width, defaults.card_width);
        assert_eq!(config.appearance.colors.accent, defaults.colors.accent);
    }

    #[test]
    fn a_partial_file_overrides_only_what_it_names() {
        let config: Config = toml::from_str(
            "[appearance]\ncard_width = 720\n\n[appearance.colors]\naccent = \"#e78284\"",
        )
        .expect("parses");

        assert_eq!(config.appearance.card_width, 720);
        assert_eq!(config.appearance.colors.accent.as_deref(), Some("#e78284"));

        // Untouched fields keep their defaults
        assert_eq!(config.appearance.row_height, 56);
        assert!(config.appearance.colors.base.is_none());
    }

    #[test]
    fn the_desktop_fills_only_what_the_file_left_unset() {
        let mut colors = Colors {
            accent: Some("#e78284".into()),
            ..Colors::default()
        };

        colors.resolve(Some(&scheme()));

        assert_eq!(colors.accent.as_deref(), Some("#e78284"));
        assert_eq!(colors.base.as_deref(), Some("#202326"));
    }

    #[test]
    fn no_desktop_leaves_colors_for_the_frontend_to_default() {
        let mut colors = Colors::default();
        colors.resolve(None);

        assert!(colors.base.is_none());
    }

    #[test]
    fn a_bad_value_is_a_parse_error() {
        assert!(toml::from_str::<Config>("[appearance]\ncard_width = \"wide\"").is_err())
    }
}

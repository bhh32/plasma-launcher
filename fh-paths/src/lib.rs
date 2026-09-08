use std::path::PathBuf;

// The only place the application name is written. Every path below derives
// from it, so renaming is this line plus the two [[bin]] entries.
pub const APP: &str = "foothold";

// The XDG config base, not the application's directory. Needed for reading
// other programs' files, such as kdeglobals.
pub fn config_home() -> PathBuf {
    base("XDG_CONFIG_HOME", ".config")
}

pub fn config_dir() -> PathBuf {
    config_home().join(APP)
}

pub fn state_dir() -> PathBuf {
    base("XDG_STATE_HOME", ".local/state").join(APP)
}

pub fn data_dir() -> PathBuf {
    base("XDG_DATA_HOME", ".local/share").join(APP)
}

// User plugins first, so one of them shadows a bundled plugin of the same name.
pub fn plugin_dirs() -> Vec<PathBuf> {
    vec![
        data_dir().join("plugins"),
        PathBuf::from("/usr/share").join(APP).join("plugins"),
    ]
}

fn base(variable: &str, fallback: &str) -> PathBuf {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(fallback)))
        .unwrap_or_else(|| PathBuf::from(fallback))
}

#[cfg(test)]
mod tests {
    use super::{APP, config_dir, config_home, data_dir, plugin_dirs, state_dir};

    #[test]
    fn every_directory_ends_in_the_app_name() {
        for dir in [config_dir(), state_dir(), data_dir()] {
            assert_eq!(dir.file_name().and_then(|n| n.to_str()), Some(APP));
        }
    }

    #[test]
    fn the_config_home_is_the_parent_of_the_config_dir() {
        assert_eq!(config_dir().parent(), Some(config_home().as_path()));
    }

    #[test]
    fn user_plugins_come_before_bundled_ones() {
        let dirs = plugin_dirs();

        assert_eq!(dirs.len(), 2);
        assert!(dirs[0].starts_with(data_dir()));
        assert!(dirs[1].starts_with("/usr/share"));
    }
}

use std::{env, path::PathBuf};

pub fn expand(path: &str) -> PathBuf {
    let Some(rest) = path.strip_prefix('~') else {
        return PathBuf::from(path);
    };
    let Some(home) = env::var_os("HOME") else {
        return PathBuf::from(path);
    };

    PathBuf::from(home).join(rest.trim_start_matches('/'))
}

#[cfg(test)]
mod tests {
    use std::{env, path::PathBuf};

    use super::expand;

    #[test]
    fn a_leading_tilde_becomes_home() {
        let home = env::var("HOME").expect("HOME is set");

        assert_eq!(expand("~"), PathBuf::from(&home));
        assert_eq!(
            expand("~/Documents"),
            PathBuf::from(&home).join("Documents")
        );
        assert_eq!(expand("/usr/bin").to_str(), Some("/usr/bin"));
    }
}

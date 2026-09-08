use super::Scheme;
use std::{collections::HashMap, fs, path::Path};

pub fn scheme(config_dir: &Path) -> Option<Scheme> {
    let source = config_dir.join("kdeglobals");
    let entries = parse(&fs::read_to_string(&source).ok()?);
    let role = |sect: &str, key: &str| -> Option<String> {
        to_hex(entries.get(&format!("{sect}/{key}"))?)
    };

    Some(Scheme {
        base: role("Colors:Window", "BackgroundNormal")?,
        mantle: role("Colors:View", "BackgroundNormal")?,
        surface0: role("Colors:Window", "BackgroundAlternate")?,
        surface1: role("Colors:Selection", "BackgroundAlternate")?,
        text: role("Colors:Window", "ForegroundNormal")?,
        subtext0: role("Colors:Window", "ForegroundInactive")?,
        subtext1: role("Colors:View", "ForegroundInactive")?,
        accent: role("Colors:Selection", "BackgroundNormal")?,
        source,
    })
}

fn parse(text: &str) -> HashMap<String, String> {
    let mut entries = HashMap::new();
    let mut section = String::new();

    for line in text.lines() {
        let line = line.trim();

        if let Some(name) = line
            .strip_prefix('[')
            .and_then(|rest| rest.strip_suffix(']'))
        {
            section = name.to_owned();
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            entries.insert(format!("{section}/{}", key.trim()), value.trim().to_owned());
        }
    }

    entries
}

fn to_hex(value: &str) -> Option<String> {
    let mut channels = value.split(',');
    let red: u8 = channels.next()?.trim().parse().ok()?;
    let green: u8 = channels.next()?.trim().parse().ok()?;
    let blue: u8 = channels.next()?.trim().parse().ok()?;

    Some(format!("#{red:02x}{green:02x}{blue:02x}"))
}

#[cfg(test)]
mod tests {
    use super::{parse, to_hex};

    #[test]
    fn sections_qualify_their_keys() {
        let entries = parse(
            "[Colors:Window]\nBackgroundNormal=32,35,38\n\n[Colors:View]\nBackgroundNormal=20,22,24\n",
        );

        assert_eq!(
            entries
                .get("Colors:Window/BackgroundNormal")
                .map(String::as_str),
            Some("32,35,38")
        );
        assert_eq!(
            entries
                .get("Colors:View/BackgroundNormal")
                .map(String::as_str),
            Some("20,22,24")
        );
    }

    #[test]
    fn triplets_become_hex() {
        assert_eq!(to_hex("61,174,233").as_deref(), Some("#3daee9"));
        assert_eq!(to_hex("not a color"), None);
    }
}

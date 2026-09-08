use freedesktop_desktop_entry::{DesktopEntry, desktop_entries, get_languages_from_env};
use std::{env, path::PathBuf};

pub struct Entry {
    pub path: PathBuf,
    pub name: String,
    pub description: String,
    pub icon: Option<String>,
    pub prefers_non_default_gpu: bool,
    pub haystack: String,
}

pub fn load() -> Vec<Entry> {
    let locales = get_languages_from_env();
    let current = env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let desktops: Vec<&str> = current.split(':').filter(|name| !name.is_empty()).collect();

    desktop_entries(&locales)
        .into_iter()
        .filter(|entry| is_launchable(entry, &desktops))
        .filter_map(|entry| build(entry, &locales))
        .collect()
}

fn is_launchable(entry: &DesktopEntry, desktops: &[&str]) -> bool {
    if entry.type_() != Some("Application") || entry.no_display() || entry.hidden() {
        return false;
    }

    if entry.exec().is_none() {
        return false;
    }

    if let Some(only) = entry.only_show_in()
        && !only.iter().any(|name| desktops.contains(name))
    {
        return false;
    }

    if let Some(not) = entry.not_show_in()
        && not.iter().any(|name| desktops.contains(name))
    {
        return false;
    }

    true
}

fn build(entry: DesktopEntry, locales: &[String]) -> Option<Entry> {
    let name = entry.name(locales)?.into_owned();
    let description = entry
        .comment(locales)
        .or_else(|| entry.generic_name(locales))
        .map(|text| text.into_owned())
        .unwrap_or_default();
    let keywords = entry
        .keywords(locales)
        .map(|words| words.join(" "))
        .unwrap_or_default();

    // necleo scores one haystack per candidate, so everything searchable is
    // joined once at load instead of on every keystroke
    let haystack = format!("{name} {description} {keywords}");
    // Read every borrowed field before moving `path` out of the entry
    let icon = entry.icon().map(str::to_owned);
    let prefers_non_default_gpu = entry.prefers_non_default_gpu();

    Some(Entry {
        path: entry.path,
        name,
        description,
        icon,
        prefers_non_default_gpu,
        haystack,
    })
}

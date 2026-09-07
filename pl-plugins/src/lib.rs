mod calc;
mod desktop_entries;
mod settings;
mod spawn;
mod terminal;
mod web;

pub use calc::Calculator;
pub use desktop_entries::DesktopEntries;
pub use settings::Settings;
pub use terminal::{Settings as TermSettings, Terminal};
pub use web::{Settings as WebSettings, Web};

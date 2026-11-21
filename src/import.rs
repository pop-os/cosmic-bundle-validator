use std::path::Path;

use cosmic_theme::Theme;
use cosmic_theme_bundle_config::{Bundle, Store};

pub struct Imported {
    pub store: Store,
    pub bundle: Bundle,
    pub theme: Theme,
}

/// Point HOME and every XDG dir at a new temp dir
pub fn isolate_home() -> tempfile::TempDir {
    let home = tempfile::tempdir().expect("temp dir");
    // SAFETY: called first thing in main, before any other thread exists.
    unsafe {
        std::env::set_var("HOME", home.path());
        for (key, dir) in [
            ("XDG_CONFIG_HOME", "config"),
            ("XDG_DATA_HOME", "data"),
            ("XDG_STATE_HOME", "state"),
            ("XDG_CACHE_HOME", "cache"),
            ("XDG_DATA_DIRS", "system"),
            ("XDG_CONFIG_DIRS", "system-config"),
        ] {
            std::env::set_var(key, home.path().join(dir));
        }
    }
    home
}

pub fn import(zip: &Path) -> Result<Imported, String> {
    let store = Store::new().map_err(|err| err.to_string())?;
    let bundle = store
        .import_archive(zip, None)
        .map_err(|err| err.to_string())?;
    let theme = store
        .bundle_theme(&bundle.slug)
        .map_err(|err| err.to_string())?;
    Ok(Imported {
        store,
        bundle,
        theme,
    })
}

impl Imported {
    pub fn print(&self) {
        println!(
            "ok         {} ({}) {:?}",
            self.bundle.name, self.bundle.slug, self.bundle.mode
        );
        for (label, color) in [
            ("background", self.theme.bg_color()),
            ("primary", self.theme.primary_container_color()),
            ("accent", self.theme.accent_color()),
        ] {
            let c = color.into_format::<u8, u8>();
            println!(
                "{label:<10} #{:02x}{:02x}{:02x}{:02x}",
                c.red, c.green, c.blue, c.alpha
            );
        }
    }
}

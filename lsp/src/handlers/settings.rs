use clap::Parser;
use std::sync::OnceLock;

static SETTINGS: OnceLock<Settings> = OnceLock::new();

#[derive(Debug, Clone, Parser)]
#[command(name = "cw-lsp")]
#[command(about = "Language Server Protocol implementation for Clausewitz script files")]
pub struct Settings {
    /// Target game type (stellaris, victoria3)
    #[arg(long, short = 'g', default_value = "stellaris")]
    pub game: String,

    /// Enable localisation validation
    #[arg(long)]
    pub validate_localisation: bool,

    /// Report unknown scopes during validation
    #[arg(long)]
    pub report_unknown_scopes: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            game: "stellaris".to_string(),
            validate_localisation: false,
            report_unknown_scopes: false,
        }
    }
}

impl Settings {
    /// Create settings from command line arguments
    pub fn from_args() -> Self {
        Self::parse()
    }

    /// Create settings from command line arguments, with fallback to default
    pub fn from_args_or_default() -> Self {
        Self::try_parse().unwrap_or_default()
    }

    /// Initialize the global settings (should be called once at startup)
    pub fn init_global(settings: Settings) {
        SETTINGS
            .set(settings)
            .expect("Settings already initialized");
    }

    /// Get a reference to the global settings
    pub fn global() -> &'static Settings {
        SETTINGS.get().expect("Settings not initialized")
    }

    /// Initialize global settings from command line arguments
    pub fn init_global_from_args() {
        let settings = Self::from_args_or_default();
        Self::init_global(settings);
    }
}

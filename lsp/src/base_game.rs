//! Base game abstraction layer that conditionally imports game-specific modules
//! based on Rust feature flags.

use anyhow::Result;
use cw_model::{GameMod, LoadMode, Modifier};
use std::collections::HashSet;
use std::path::PathBuf;

#[cfg(feature = "stellaris")]
use cw_games::stellaris;

#[cfg(feature = "victoria_3")]
use cw_games::victoria_3;

#[cfg(feature = "stellaris")]
pub use stellaris::BaseGame;

#[cfg(feature = "victoria_3")]
pub use victoria_3::BaseGame;

#[cfg(feature = "stellaris")]
pub use stellaris::STELLARIS_INSTALL_PATH as GAME_INSTALL_PATH;

#[cfg(feature = "victoria_3")]
pub use victoria_3::VICTORIA_3_INSTALL_PATH as GAME_INSTALL_PATH;

/// Game-agnostic wrapper functions
pub mod game {
    use crate::interner::get_interner;

    use super::*;

    /// Load the base game as a mod definition
    pub fn load_global_as_mod_definition(
        load_mode: LoadMode,
        file_index: Option<&HashSet<String>>,
    ) -> &'static GameMod {
        BaseGame::load_global_as_mod_definition(load_mode, get_interner(), file_index, false)
    }

    /// Load the base game with optional custom install path
    pub fn load_as_mod_definition(
        install_path: Option<&std::path::Path>,
        load_mode: LoadMode,
        file_index: Option<&HashSet<String>>,
    ) -> Result<GameMod> {
        BaseGame::load_as_mod_definition(install_path, load_mode, get_interner(), file_index, false)
    }

    /// Get the game installation directory on Windows
    pub fn get_install_directory_windows() -> Option<PathBuf> {
        BaseGame::get_install_directory_windows()
    }

    /// Load modifiers from the game logs directory
    pub fn load_modifiers() -> Result<Vec<Modifier>> {
        BaseGame::load_modifiers(get_interner())
    }

    /// Get the default config path for the current game
    pub fn get_default_config_path() -> PathBuf {
        #[cfg(feature = "stellaris")]
        {
            std::path::Path::new(r"D:\dev\github\cwtools-stellaris-config\config").to_path_buf()
        }
        #[cfg(feature = "victoria_3")]
        {
            std::path::Path::new(r"D:\dev\github\cwtools-vic3-config\config").to_path_buf()
        }
    }

    /// Get the executable name for the current game
    pub fn get_executable_name() -> &'static str {
        BaseGame::get_executable_name()
    }

    /// Get the glob patterns for the current game
    pub fn get_glob_patterns() -> Vec<&'static str> {
        BaseGame::get_glob_patterns()
    }

    /// Detect the base directory (game or mod root) by walking up the directory tree
    /// looking for game-specific files
    pub fn detect_base_directory(path: &std::path::Path) -> Option<PathBuf> {
        BaseGame::detect_base_directory(path)
    }
}

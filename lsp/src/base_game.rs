//! Base game abstraction layer that dynamically selects game-specific modules
//! based on runtime settings.

use anyhow::Result;
use cw_model::{GameMod, LoadMode, Modifier};
use std::collections::HashSet;
use std::path::PathBuf;

// Always import both games for runtime selection
use cw_games::stellaris;
use cw_games::victoria_3;

use crate::handlers::settings::Settings;

/// Game-agnostic wrapper functions that dispatch to the appropriate game implementation
/// based on the current settings.
pub mod game {
    use super::*;
    use crate::interner::get_interner;

    /// Determine which game implementation to use based on settings
    fn get_current_game() -> &'static str {
        Settings::global().game.as_str()
    }

    /// Load the base game as a mod definition
    pub fn load_global_as_mod_definition(
        load_mode: LoadMode,
        file_index: Option<&HashSet<String>>,
    ) -> &'static GameMod {
        match get_current_game() {
            "victoria3" => victoria_3::BaseGame::load_global_as_mod_definition(
                load_mode,
                get_interner(),
                file_index,
                false,
            ),
            _ => stellaris::BaseGame::load_global_as_mod_definition(
                load_mode,
                get_interner(),
                file_index,
                false,
            ),
        }
    }

    /// Load the base game with optional custom install path
    pub fn load_as_mod_definition(
        install_path: Option<&std::path::Path>,
        load_mode: LoadMode,
        file_index: Option<&HashSet<String>>,
    ) -> Result<GameMod> {
        match get_current_game() {
            "victoria3" => victoria_3::BaseGame::load_as_mod_definition(
                install_path,
                load_mode,
                get_interner(),
                file_index,
                false,
            ),
            _ => stellaris::BaseGame::load_as_mod_definition(
                install_path,
                load_mode,
                get_interner(),
                file_index,
                false,
            ),
        }
    }

    /// Get the game installation directory on Windows
    pub fn get_install_directory_windows() -> Option<PathBuf> {
        match get_current_game() {
            "victoria3" => victoria_3::BaseGame::get_install_directory_windows(),
            _ => stellaris::BaseGame::get_install_directory_windows(),
        }
    }

    /// Load modifiers from the game logs directory
    pub fn load_modifiers() -> Result<Vec<Modifier>> {
        match get_current_game() {
            "victoria3" => victoria_3::BaseGame::load_modifiers(get_interner()),
            _ => stellaris::BaseGame::load_modifiers(get_interner()),
        }
    }

    /// Get the default config path for the current game
    pub fn get_default_config_path() -> PathBuf {
        match get_current_game() {
            "victoria3" => {
                std::path::Path::new(r"D:\dev\github\cwtools-vic3-config\config").to_path_buf()
            }
            _ => {
                std::path::Path::new(r"D:\dev\github\cwtools-stellaris-config\config").to_path_buf()
            }
        }
    }

    /// Get the executable name for the current game
    pub fn get_executable_name() -> &'static str {
        match get_current_game() {
            "victoria3" => victoria_3::BaseGame::get_executable_name(),
            _ => stellaris::BaseGame::get_executable_name(),
        }
    }

    /// Get the glob patterns for the current game
    pub fn get_glob_patterns() -> Vec<&'static str> {
        match get_current_game() {
            "victoria3" => victoria_3::BaseGame::get_glob_patterns(),
            _ => stellaris::BaseGame::get_glob_patterns(),
        }
    }

    /// Detect the base directory (game or mod root) by walking up the directory tree
    /// looking for game-specific files
    pub fn detect_base_directory(path: &std::path::Path) -> Option<PathBuf> {
        match get_current_game() {
            "victoria3" => victoria_3::BaseGame::detect_base_directory(path),
            _ => stellaris::BaseGame::detect_base_directory(path),
        }
    }
}

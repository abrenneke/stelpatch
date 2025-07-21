//! Base game abstraction layer that conditionally imports game-specific modules
//! based on Rust feature flags.

use std::path::PathBuf;
use anyhow::Result;
use cw_model::{GameMod, LoadMode, Modifier};

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
    use super::*;
    
    /// Load the base game as a mod definition
    pub fn load_global_as_mod_definition(load_mode: LoadMode) -> &'static GameMod {
        BaseGame::load_global_as_mod_definition(load_mode)
    }
    
    /// Load the base game with optional custom install path
    pub fn load_as_mod_definition(
        install_path: Option<&std::path::Path>,
        load_mode: LoadMode,
    ) -> Result<GameMod> {
        BaseGame::load_as_mod_definition(install_path, load_mode)
    }
    
    /// Get the game installation directory on Windows
    pub fn get_install_directory_windows() -> Option<PathBuf> {
        BaseGame::get_install_directory_windows()
    }
    
    /// Load modifiers from the game logs directory
    pub fn load_modifiers() -> Result<Vec<Modifier>> {
        BaseGame::load_modifiers()
    }
} 
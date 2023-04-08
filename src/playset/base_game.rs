use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use winreg::{enums::HKEY_CURRENT_USER, RegKey};

use super::{game_mod::GameMod, mod_definition::ModDefinition};

lazy_static! {
    pub static ref STELLARIS_INSTALL_PATH: Option<PathBuf> =
        BaseGame::get_install_directory_windows();
    pub static ref BASE_MOD: GameMod = BaseGame::load_as_mod_definition(None).unwrap();
}

pub struct BaseGame {}

impl BaseGame {
    pub fn load_as_mod_definition(
        install_path: Option<PathBuf>,
    ) -> Result<GameMod, Box<dyn std::error::Error>> {
        let install_path = if let Some(path) = install_path {
            Some(path)
        } else {
            STELLARIS_INSTALL_PATH.clone()
        };
        match install_path {
            Some(path) => {
                let definition = ModDefinition {
                    name: "Stellaris".to_string(),
                    path: Some(path.to_string_lossy().to_string()),
                    version: None,
                    tags: vec![],
                    picture: None,
                    supported_version: None,
                    remote_file_id: None,
                    dependencies: vec![],
                    archive: None,
                };

                let game_mod = GameMod::load_parallel(definition)?;

                Ok(game_mod)
            }
            None => Err("Could not find Stellaris installation directory".into()),
        }
    }

    pub fn get_install_directory_windows() -> Option<PathBuf> {
        // Get the Steam installation path from the registry
        let key = RegKey::predef(HKEY_CURRENT_USER)
            .open_subkey("SOFTWARE\\Valve\\Steam")
            .unwrap();
        let steam_path: String = key.get_value("SteamPath").unwrap();

        // Parse the libraryfolders.vdf file to find the folders that contain games
        let libraryfolders_path = Path::new(&steam_path)
            .join("steamapps")
            .join("libraryfolders.vdf");
        let libraryfolders_file = File::open(libraryfolders_path).unwrap();
        let libraryfolders_reader = BufReader::new(libraryfolders_file);
        let mut steam_library_paths: Vec<String> = vec![steam_path];

        for line in libraryfolders_reader.lines() {
            let line_str = line.unwrap();
            if line_str.contains("path") {
                let path_str = line_str.split('"').nth(3).unwrap();
                steam_library_paths.push(path_str.to_string());
            }
        }

        // Check each library folder for the Stellaris game folder
        let mut stellaris_path = None;
        for library_path in steam_library_paths {
            let common_path = Path::new(&library_path).join("steamapps").join("common");
            for entry in common_path.read_dir().unwrap() {
                let entry_path = entry.unwrap().path();
                if entry_path.file_name().unwrap().to_str() == Some("Stellaris") {
                    stellaris_path = Some(common_path.join("Stellaris"));
                    break;
                }
            }
            if stellaris_path.is_some() {
                break;
            }
        }

        stellaris_path
    }
}

#[cfg(test)]
mod tests {
    use crate::playset::base_game::BaseGame;

    #[test]
    fn test_get_install_directory_windows() {
        dbg!(BaseGame::get_install_directory_windows());
    }

    #[test]
    fn load_base_game_as_mod() {
        let base_game = BaseGame::load_as_mod_definition(None).unwrap();

        assert!(base_game.modules.len() > 0);
        dbg!(base_game.modules.len());
    }
}

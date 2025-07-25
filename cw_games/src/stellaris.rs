use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::anyhow;
use cw_model::{GameMod, LoadMode, ModDefinition, Modifier, parse_modifier_log};
use lazy_static::lazy_static;
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

lazy_static! {
    pub static ref STELLARIS_INSTALL_PATH: Option<PathBuf> =
        BaseGame::get_install_directory_windows();
}

pub struct BaseGame {}

static BASE_MOD: OnceLock<GameMod> = OnceLock::new();

impl BaseGame {
    pub fn load_global_as_mod_definition(load_mode: LoadMode) -> &'static GameMod {
        BASE_MOD.get_or_init(|| {
            Self::load_as_mod_definition(None, load_mode).expect("Could not load base game")
        })
    }

    pub fn load_as_mod_definition(
        install_path: Option<&Path>,
        load_mode: LoadMode,
    ) -> Result<GameMod, anyhow::Error> {
        let install_path = if let Some(path) = install_path {
            Some(path)
        } else {
            STELLARIS_INSTALL_PATH.as_ref().map(|path| path.as_path())
        };
        match install_path {
            Some(path) => {
                let definition = ModDefinition {
                    ast: None,
                    name: "Stellaris".to_string(),
                    path: Some(path.to_path_buf()),
                    version: None,
                    tags: vec![],
                    picture: None,
                    supported_version: None,
                    remote_file_id: None,
                    dependencies: vec![],
                    archive: None,
                };

                let game_mod = GameMod::load(definition, load_mode)?;

                // BASE_MOD
                //     .set(game_mod)
                //     .map_err(|_| anyhow!("Could not set base mod"))?;

                Ok(game_mod)
                // Ok(BASE_MOD.get().unwrap())
            }
            None => Err(anyhow!("Could not find Stellaris installation directory")),
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

    /// Loads modifiers from the Stellaris logs directory
    pub fn load_modifiers() -> Result<Vec<Modifier>, anyhow::Error> {
        load_stellaris_modifiers()
    }
}

/// Gets /Users/Username/Documents/Paradox Interactive/Stellaris
pub fn stellaris_documents_dir() -> Result<PathBuf, anyhow::Error> {
    let home_dir =
        dirs::document_dir().ok_or_else(|| anyhow!("Could not find Documents directory"))?;
    let path = vec![
        home_dir.to_str().ok_or_else(|| {
            anyhow!(
                "Could not convert Documents directory to string: {}",
                home_dir.display()
            )
        })?,
        "Paradox Interactive",
        "Stellaris",
    ]
    .iter()
    .collect::<PathBuf>();
    Ok(path.into())
}

/// Gets the path to the modifiers log file
pub fn stellaris_modifiers_log_path() -> Result<PathBuf, anyhow::Error> {
    let docs_dir = stellaris_documents_dir()?;
    let path = docs_dir
        .join("logs")
        .join("script_documentation")
        .join("modifiers.log");
    Ok(path)
}

/// Loads and parses modifiers from the Stellaris modifiers log
pub fn load_stellaris_modifiers() -> Result<Vec<Modifier>, anyhow::Error> {
    let log_path = stellaris_modifiers_log_path()?;

    if !log_path.exists() {
        return Err(anyhow!(
            "Modifiers log not found at: {}",
            log_path.display()
        ));
    }

    let log_content = std::fs::read_to_string(&log_path)
        .map_err(|e| anyhow!("Failed to read modifiers log: {}", e))?;

    let modifiers = parse_modifier_log(&log_content);

    if modifiers.is_empty() {
        return Err(anyhow!("No modifiers found in log file"));
    }

    Ok(modifiers)
}

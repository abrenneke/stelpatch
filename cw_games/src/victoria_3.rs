use std::{
    collections::HashSet,
    fs::File,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    sync::OnceLock,
};

use anyhow::anyhow;
use cw_model::{
    CaseInsensitiveInterner, GameMod, LoadMode, ModDefinition, Modifier, parse_modifier_log,
};
use lazy_static::lazy_static;
use winreg::{RegKey, enums::HKEY_CURRENT_USER};

lazy_static! {
    pub static ref VICTORIA_3_INSTALL_PATH: Option<PathBuf> =
        BaseGame::get_install_directory_windows();
}

pub struct BaseGame {}

static BASE_MOD: OnceLock<GameMod> = OnceLock::new();

impl BaseGame {
    pub fn load_global_as_mod_definition(
        load_mode: LoadMode,
        interner: &CaseInsensitiveInterner,
        file_index: Option<&HashSet<String>>,
        preserve_ast: bool,
    ) -> &'static GameMod {
        BASE_MOD.get_or_init(|| {
            Self::load_as_mod_definition(None, load_mode, interner, file_index, preserve_ast)
                .expect("Could not load base game")
        })
    }

    pub fn load_as_mod_definition(
        install_path: Option<&Path>,
        load_mode: LoadMode,
        interner: &CaseInsensitiveInterner,
        file_index: Option<&HashSet<String>>,
        preserve_ast: bool,
    ) -> Result<GameMod, anyhow::Error> {
        let install_path = if let Some(path) = install_path {
            Some(path)
        } else {
            VICTORIA_3_INSTALL_PATH.as_ref().map(|path| path.as_path())
        };
        match install_path {
            Some(path) => {
                let definition = ModDefinition {
                    ast: None,
                    name: "Victoria 3".to_string(),
                    path: Some(path.to_path_buf()),
                    version: None,
                    tags: vec![],
                    picture: None,
                    supported_version: None,
                    remote_file_id: None,
                    dependencies: vec![],
                    archive: None,
                };

                let game_mod = GameMod::load(
                    definition,
                    load_mode,
                    interner,
                    Self::get_glob_patterns(),
                    file_index,
                    preserve_ast,
                )?;

                // BASE_MOD
                //     .set(game_mod)
                //     .map_err(|_| anyhow!("Could not set base mod"))?;

                Ok(game_mod)
                // Ok(BASE_MOD.get().unwrap())
            }
            None => Err(anyhow!("Could not find Victoria 3 installation directory")),
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

        // Check each library folder for the Victoria 3 game folder
        let mut victoria_3_path = None;
        for library_path in steam_library_paths {
            let common_path = Path::new(&library_path).join("steamapps").join("common");
            for entry in common_path.read_dir().unwrap() {
                let entry_path = entry.unwrap().path();
                if entry_path.file_name().unwrap().to_str() == Some("Victoria 3") {
                    victoria_3_path = Some(common_path.join("Victoria 3"));
                    break;
                }
            }
            if victoria_3_path.is_some() {
                break;
            }
        }

        victoria_3_path
    }

    /// Loads modifiers from the Victoria 3 logs directory
    pub fn load_modifiers(
        interner: &CaseInsensitiveInterner,
    ) -> Result<Vec<Modifier>, anyhow::Error> {
        load_victoria_3_modifiers(interner)
    }

    /// Get the executable name for this game
    pub fn get_executable_name() -> &'static str {
        "victoria3.exe"
    }

    pub fn get_glob_patterns() -> Vec<&'static str> {
        let glob_patterns = vec![
            // Victoria 3 patterns (modular structure)
            // Game-specific files
            "game/common/**/*.txt",
            "game/interface/**/*.txt",
            "game/events/**/*.txt",
            "game/gfx/**/*.gfx",
            "game/gfx/**/*.asset",
            "game/gfx/**/*.txt",
            "game/gui/**/*.gui",
            "game/gui/**/*.gfx",
            "game/map_data/**/*.txt",
            "game/music/**/*.txt",
            "game/music/**/*.asset",
            "game/sound/**/*.txt",
            "game/sound/**/*.asset",
            // Framework files (jomini)
            "jomini/common/**/*.txt",
            "jomini/gfx/**/*.gfx",
            "jomini/gfx/**/*.asset",
            "jomini/gui/**/*.gui",
            "jomini/gui/**/*.gfx",
            // Engine files (clausewitz)
            "clausewitz/gfx/**/*.gfx",
            "clausewitz/gfx/**/*.asset",
            "clausewitz/gui/**/*.gui",
            "clausewitz/gui/**/*.gfx",
        ];

        glob_patterns
    }
}

/// Gets /Users/Username/Documents/Paradox Interactive/Victoria 3
pub fn victoria_3_documents_dir() -> Result<PathBuf, anyhow::Error> {
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
        "Victoria 3",
    ]
    .iter()
    .collect::<PathBuf>();
    Ok(path.into())
}

/// Gets the path to the modifiers log file
pub fn victoria_3_modifiers_log_path() -> Result<PathBuf, anyhow::Error> {
    let docs_dir = victoria_3_documents_dir()?;
    let path = docs_dir
        .join("logs")
        .join("script_documentation")
        .join("modifiers.log");
    Ok(path)
}

/// Loads and parses modifiers from the Victoria 3 modifiers log
pub fn load_victoria_3_modifiers(
    interner: &CaseInsensitiveInterner,
) -> Result<Vec<Modifier>, anyhow::Error> {
    let log_path = victoria_3_modifiers_log_path()?;

    if !log_path.exists() {
        return Err(anyhow!(
            "Modifiers log not found at: {}",
            log_path.display()
        ));
    }

    let log_content = std::fs::read_to_string(&log_path)
        .map_err(|e| anyhow!("Failed to read modifiers log: {}", e))?;

    let modifiers = parse_modifier_log(&log_content, &interner);

    if modifiers.is_empty() {
        return Err(anyhow!("No modifiers found in log file"));
    }

    Ok(modifiers)
}

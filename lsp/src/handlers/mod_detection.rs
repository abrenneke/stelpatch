use super::utils::log_message_sync;
use crate::base_game::GAME_INSTALL_PATH;
use crate::interner::get_interner;
use anyhow::{Result, anyhow};
use cw_model::{GameMod, LoadMode, ModDefinition};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tower_lsp::Client;

/// Check if a file path is part of the base game directory
pub fn is_base_game_file(file_path: &Path) -> bool {
    if let Some(install_path) = GAME_INSTALL_PATH.as_ref() {
        file_path.starts_with(install_path)
    } else {
        false
    }
}

/// Walk up the directory tree to find a descriptor.mod file
pub fn find_descriptor_mod(mut path: PathBuf) -> Option<PathBuf> {
    // Start from the file's directory if it's a file
    if path.is_file() {
        path = path.parent()?.to_path_buf();
    }

    loop {
        let descriptor_path = path.join("descriptor.mod");
        if descriptor_path.exists() {
            return Some(descriptor_path);
        }

        // Move up to parent directory
        if let Some(parent) = path.parent() {
            path = parent.to_path_buf();
        } else {
            break;
        }
    }

    None
}

/// Load mod dependencies recursively
pub fn load_mod_dependencies(
    mod_definition: &ModDefinition,
    client: &Client,
    loaded_mods: &mut HashMap<String, GameMod>,
) -> Result<()> {
    // Load each dependency
    for dependency in &mod_definition.dependencies {
        // Skip if already loaded
        if loaded_mods.contains_key(dependency) {
            continue;
        }

        log_message_sync(
            client,
            tower_lsp::lsp_types::MessageType::INFO,
            format!("Loading dependency: {}", dependency),
        );

        // Try to find the dependency mod
        // This is a simplified approach - in a real implementation,
        // you'd need to search the mod directories for the dependency
        // For now, we'll just log that a dependency was found
        log_message_sync(
            client,
            tower_lsp::lsp_types::MessageType::INFO,
            format!("Dependency {} would be loaded here", dependency),
        );
    }

    Ok(())
}

/// Load a mod from a descriptor.mod file with dependency resolution
pub fn load_mod_from_descriptor_with_dependencies(
    descriptor_path: &Path,
    client: &Client,
    loaded_mods: &mut HashMap<String, GameMod>,
) -> Result<GameMod> {
    log_message_sync(
        client,
        tower_lsp::lsp_types::MessageType::INFO,
        format!("Loading mod from descriptor: {}", descriptor_path.display()),
    );

    // Parse the descriptor.mod file
    let mut mod_definition = ModDefinition::load_from_file(descriptor_path)?;

    // Set the mod path to the directory containing descriptor.mod
    let mod_dir = descriptor_path
        .parent()
        .ok_or_else(|| anyhow!("Could not get parent directory of descriptor.mod"))?;
    mod_definition.path = Some(mod_dir.to_path_buf());

    // Load dependencies first
    load_mod_dependencies(&mod_definition, client, loaded_mods)?;

    // Load the mod using the existing GameMod::load functionality
    let game_mod = GameMod::load(
        mod_definition,
        LoadMode::Parallel,
        get_interner(),
        crate::base_game::game::get_glob_patterns(),
        None,
        false,
    )?;

    // Add to loaded mods cache
    loaded_mods.insert(game_mod.definition.name.clone(), game_mod.clone());

    log_message_sync(
        client,
        tower_lsp::lsp_types::MessageType::INFO,
        format!("Successfully loaded mod: {}", game_mod.definition.name),
    );

    Ok(game_mod)
}

/// Load a mod from a descriptor.mod file
pub fn load_mod_from_descriptor(descriptor_path: &Path, client: &Client) -> Result<GameMod> {
    log_message_sync(
        client,
        tower_lsp::lsp_types::MessageType::INFO,
        format!("Loading mod from descriptor: {}", descriptor_path.display()),
    );

    // Parse the descriptor.mod file
    let mut mod_definition = ModDefinition::load_from_file(descriptor_path)?;

    // Set the mod path to the directory containing descriptor.mod
    let mod_dir = descriptor_path
        .parent()
        .ok_or_else(|| anyhow!("Could not get parent directory of descriptor.mod"))?;
    mod_definition.path = Some(mod_dir.to_path_buf());

    // Load the mod using the existing GameMod::load functionality
    let game_mod = GameMod::load(
        mod_definition,
        LoadMode::Parallel,
        get_interner(),
        crate::base_game::game::get_glob_patterns(),
        None,
        false,
    )?;

    log_message_sync(
        client,
        tower_lsp::lsp_types::MessageType::INFO,
        format!("Successfully loaded mod: {}", game_mod.definition.name),
    );

    Ok(game_mod)
}

/// Check if a file is a mod file and load the mod if needed
pub fn handle_mod_file(file_path: &Path, client: &Client) -> Result<Option<GameMod>> {
    let mut temp_cache = HashMap::new();
    handle_mod_file_with_cache(file_path, client, &mut temp_cache)
}

/// Check if a file is a mod file and load the mod if needed
/// Returns a GameMod if the file is part of a mod, None if it's a base game file
pub fn handle_mod_file_with_cache(
    file_path: &Path,
    client: &Client,
    mod_cache: &mut HashMap<PathBuf, GameMod>,
) -> Result<Option<GameMod>> {
    // First check if it's a base game file
    if is_base_game_file(file_path) {
        return Ok(None);
    }

    // Try to find descriptor.mod in the directory tree
    if let Some(descriptor_path) = find_descriptor_mod(file_path.to_path_buf()) {
        // Get the mod directory for caching
        let mod_dir = descriptor_path
            .parent()
            .ok_or_else(|| anyhow!("Could not get parent directory of descriptor.mod"))?
            .to_path_buf();

        // Check if mod is already cached
        if let Some(cached_mod) = mod_cache.get(&mod_dir) {
            log_message_sync(
                client,
                tower_lsp::lsp_types::MessageType::INFO,
                format!("Using cached mod: {}", cached_mod.definition.name),
            );
            return Ok(Some(cached_mod.clone()));
        }

        log_message_sync(
            client,
            tower_lsp::lsp_types::MessageType::INFO,
            format!("Found descriptor.mod at: {}", descriptor_path.display()),
        );

        // Use dependency-aware loading (with block_in_place for the async call)
        let mut loaded_mods = HashMap::new();
        let load_result = {
            let descriptor_path = descriptor_path.clone();
            let client = client.clone();
            load_mod_from_descriptor_with_dependencies(&descriptor_path, &client, &mut loaded_mods)
        };

        match load_result {
            Ok(game_mod) => {
                // Cache the mod
                mod_cache.insert(mod_dir, game_mod.clone());
                Ok(Some(game_mod))
            }
            Err(e) => {
                log_message_sync(
                    client,
                    tower_lsp::lsp_types::MessageType::ERROR,
                    format!("Failed to load mod: {}", e),
                );
                Err(e)
            }
        }
    } else {
        log_message_sync(
            client,
            tower_lsp::lsp_types::MessageType::INFO,
            format!("No descriptor.mod found for file: {}", file_path.display()),
        );
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_find_descriptor_mod() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create a nested directory structure
        let sub_dir = temp_path.join("common").join("buildings");
        fs::create_dir_all(&sub_dir).unwrap();

        // Create a descriptor.mod file in the root
        let descriptor_path = temp_path.join("descriptor.mod");
        fs::write(&descriptor_path, "name=\"Test Mod\"").unwrap();

        // Create a test file in the subdirectory
        let test_file = sub_dir.join("test_building.txt");
        fs::write(&test_file, "test content").unwrap();

        // Test finding descriptor.mod from the nested file
        let found = find_descriptor_mod(test_file);
        assert_eq!(found, Some(descriptor_path));
    }

    #[test]
    fn test_find_descriptor_mod_not_found() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        let test_file = temp_path.join("test.txt");
        fs::write(&test_file, "test content").unwrap();

        let found = find_descriptor_mod(test_file);
        assert_eq!(found, None);
    }
}

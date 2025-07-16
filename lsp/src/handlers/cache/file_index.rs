use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use cw_games::stellaris::BaseGame;
use cw_model::GameMod;

/// Cache for file existence checks in the game directory and loaded mods
#[derive(Clone)]
pub struct FileIndex {
    /// Set of all file paths relative to the game root directory
    /// Normalized to use forward slashes for consistent lookup
    files: HashSet<String>,
    /// The root game directory path
    game_root: PathBuf,
    /// Additional mod directories that have been integrated
    mod_paths: Vec<PathBuf>,
}

static FILE_INDEX_CACHE: RwLock<Option<Arc<FileIndex>>> = RwLock::new(None);

impl FileIndex {
    /// Initialize the file index cache in a background thread
    pub fn initialize_in_background() {
        std::thread::spawn(|| {
            let _ = Self::get_or_init_blocking();
        });
    }

    /// Get the global file index cache, returns None if not yet initialized
    pub fn get() -> Option<Arc<FileIndex>> {
        FILE_INDEX_CACHE.read().unwrap().clone()
    }

    /// Check if the file index has been initialized
    pub fn is_initialized() -> bool {
        FILE_INDEX_CACHE.read().unwrap().is_some()
    }

    /// Reset the file index cache, forcing re-initialization on next access
    pub fn reset() {
        eprintln!("Resetting FileIndex cache");
        let mut cache = FILE_INDEX_CACHE.write().unwrap();
        *cache = None;
    }

    /// Get or initialize the global file index cache (blocking version)
    fn get_or_init_blocking() -> Arc<FileIndex> {
        // Check if already initialized
        if let Some(index) = Self::get() {
            return index;
        }

        // Compute the result without holding the lock
        let start = Instant::now();
        eprintln!("Initializing file index cache...");

        let base_game = BaseGame::load_global_as_mod_definition(cw_model::LoadMode::Parallel);

        let game_root = if let Some(path) = &base_game.definition.path {
            path.clone()
        } else {
            eprintln!("Warning: No game root path found, file index will be empty");
            let result = Arc::new(FileIndex {
                files: HashSet::new(),
                game_root: PathBuf::new(),
                mod_paths: Vec::new(),
            });

            // Store empty result
            let mut cache = FILE_INDEX_CACHE.write().unwrap();
            if cache.is_none() {
                *cache = Some(result.clone());
            }
            return result;
        };

        let mut files = HashSet::new();

        if let Err(e) = Self::scan_directory_recursive(&game_root, &game_root, &mut files) {
            eprintln!("Warning: Failed to scan game directory: {}", e);
        }

        eprintln!(
            "Built file index cache with {} files in {:?}",
            files.len(),
            start.elapsed()
        );

        let result = Arc::new(FileIndex {
            files,
            game_root,
            mod_paths: Vec::new(),
        });

        // Now acquire the lock only to store the result
        let mut cache = FILE_INDEX_CACHE.write().unwrap();
        if cache.is_none() {
            *cache = Some(result.clone());
        }

        result
    }

    /// Recursively scan a directory and add all files to the set
    fn scan_directory_recursive(
        root_dir: &Path,
        current_dir: &Path,
        files: &mut HashSet<String>,
    ) -> Result<(), std::io::Error> {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Store relative path from game root, normalized with forward slashes
                if let Ok(relative_path) = path.strip_prefix(root_dir) {
                    let normalized_path = relative_path.to_string_lossy().replace('\\', "/");
                    files.insert(normalized_path);
                }
            } else if path.is_dir() {
                // Skip common directories that don't contain game files
                if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                    match dir_name {
                        ".git" | ".svn" | "node_modules" | "target" | "dist" | "build" => continue,
                        _ => {}
                    }
                }

                // Recursively scan subdirectory
                Self::scan_directory_recursive(root_dir, &path, files)?;
            }
        }
        Ok(())
    }

    /// Check if a file exists in the game directory
    ///
    /// # Arguments
    /// * `file_path` - Path to check, can be relative or use any slash style
    ///
    /// # Returns
    /// * `true` if the file exists in the indexed files
    /// * `false` if the file doesn't exist or index is not initialized
    pub fn file_exists(&self, file_path: &str) -> bool {
        // Normalize the path for lookup
        let normalized_path = file_path.replace('\\', "/");

        // Try exact match first
        if self.files.contains(&normalized_path) {
            return true;
        }

        // Try without leading slash
        let trimmed_path = normalized_path.trim_start_matches('/');
        if self.files.contains(trimmed_path) {
            return true;
        }

        false
    }

    /// Get all files matching a pattern (simple contains check)
    pub fn find_files_containing(&self, pattern: &str) -> Vec<String> {
        self.files
            .iter()
            .filter(|path| path.contains(pattern))
            .cloned()
            .collect()
    }

    /// Get all files with a specific extension
    pub fn find_files_with_extension(&self, extension: &str) -> Vec<String> {
        let ext_pattern = if extension.starts_with('.') {
            extension.to_string()
        } else {
            format!(".{}", extension)
        };

        self.files
            .iter()
            .filter(|path| path.ends_with(&ext_pattern))
            .cloned()
            .collect()
    }

    /// Get the total number of indexed files
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// Get the game root directory
    pub fn game_root(&self) -> &Path {
        &self.game_root
    }

    /// Add files from a mod to the index
    pub fn integrate_mod(&mut self, game_mod: &GameMod) {
        if let Some(mod_path) = &game_mod.definition.path {
            let start = Instant::now();
            let initial_count = self.files.len();

            // Add to mod paths if not already there
            if !self.mod_paths.contains(mod_path) {
                self.mod_paths.push(mod_path.clone());
            }

            // For mods, we want the files to be stored as if they were part of the game directory
            // so we pass mod_path as both the root and current directory
            if let Err(e) = Self::scan_directory_recursive(mod_path, mod_path, &mut self.files) {
                eprintln!(
                    "Warning: Failed to scan mod directory {}: {}",
                    mod_path.display(),
                    e
                );
            } else {
                let added_files = self.files.len() - initial_count;
                eprintln!(
                    "Integrated mod '{}' with {} files in {:?}",
                    game_mod.definition.name,
                    added_files,
                    start.elapsed()
                );
            }
        }
    }

    /// Add files from multiple mods to the index
    pub fn integrate_mods(&mut self, mods: &[&GameMod]) {
        for game_mod in mods {
            self.integrate_mod(game_mod);
        }
    }

    /// Update the global file index cache with mod data
    pub fn update_global_with_mod(game_mod: &GameMod) {
        if let Some(current_index) = Self::get() {
            // Create a new FileIndex with the mod integrated
            let mut new_index = FileIndex {
                files: current_index.files.clone(),
                game_root: current_index.game_root.clone(),
                mod_paths: current_index.mod_paths.clone(),
            };
            new_index.integrate_mod(game_mod);

            // Update the global cache
            let mut cache = FILE_INDEX_CACHE.write().unwrap();
            *cache = Some(Arc::new(new_index));
        }
    }

    /// Update the global file index cache with multiple mods
    pub fn update_global_with_mods(mods: &[&GameMod]) {
        if let Some(current_index) = Self::get() {
            // Create a new FileIndex with the mods integrated
            let mut new_index = FileIndex {
                files: current_index.files.clone(),
                game_root: current_index.game_root.clone(),
                mod_paths: current_index.mod_paths.clone(),
            };
            new_index.integrate_mods(mods);

            // Update the global cache
            let mut cache = FILE_INDEX_CACHE.write().unwrap();
            *cache = Some(Arc::new(new_index));
        }
    }
}

/// Convenience function to check if a file exists
pub fn file_exists(file_path: &str) -> bool {
    if let Some(index) = FileIndex::get() {
        index.file_exists(file_path)
    } else {
        false
    }
}

/// Convenience function to find files containing a pattern
pub fn find_files_containing(pattern: &str) -> Vec<String> {
    if let Some(index) = FileIndex::get() {
        index.find_files_containing(pattern)
    } else {
        Vec::new()
    }
}

/// Convenience function to find files with an extension
pub fn find_files_with_extension(extension: &str) -> Vec<String> {
    if let Some(index) = FileIndex::get() {
        index.find_files_with_extension(extension)
    } else {
        Vec::new()
    }
}

/// Convenience function to initialize the file index in the background
pub fn initialize_file_index() {
    FileIndex::initialize_in_background();
}

/// Convenience function to add a mod to the global file index
pub fn add_mod_to_index(game_mod: &GameMod) {
    FileIndex::update_global_with_mod(game_mod);
}

/// Convenience function to add multiple mods to the global file index
pub fn add_mods_to_index(mods: &[&GameMod]) {
    FileIndex::update_global_with_mods(mods);
}

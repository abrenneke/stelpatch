//! File index cache for fast file existence checks
//!
//! This module provides a high-performance file index that caches the complete file listing
//! of the game directory and loaded mods. Since games like Stellaris can contain 37,000+
//! files nested in directories, scanning them all is expensive (kernel time).
//!
//! To solve this, the cache is persisted to disk using the game executable's version/metadata
//! as a cache key. This allows near-instant loading on subsequent runs when the game hasn't
//! been updated.
//!
//! ## Cache Invalidation
//! The cache automatically invalidates when:
//! - Game executable is updated (different size, modification time)
//! - Game directory path changes
//! - Cache is manually reset via `FileIndex::reset()`
//!
//! ## Cache Location
//! - Windows: `%LOCALAPPDATA%\stelpatch\file_index\`
//! - Other platforms: Uses standard cache directories via `dirs` crate

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use std::time::Instant;

use cw_model::GameMod;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Serializable cache structure for disk storage
#[derive(Debug, Clone, Serialize, Deserialize)]
struct FileIndexCache {
    /// Set of all file paths
    files: HashSet<String>,
    /// Game version/checksum this cache was built for
    game_version_hash: String,
    /// Timestamp when cache was created
    created_at: u64,
}

impl FileIndexCache {
    /// Create a new cache from file set and game version
    fn new(files: HashSet<String>, game_version_hash: String) -> Self {
        Self {
            files,
            game_version_hash,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        }
    }

    /// Check if this cache is valid for the given game version
    fn is_valid_for_version(&self, expected_hash: &str) -> bool {
        self.game_version_hash == expected_hash
    }
}

/// Get the cache directory for file index storage
fn get_cache_directory() -> Result<PathBuf, std::io::Error> {
    let cache_dir = dirs::cache_dir()
        .or_else(|| dirs::data_local_dir())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not find cache directory",
            )
        })?
        .join("stelpatch")
        .join("file_index");

    fs::create_dir_all(&cache_dir)?;
    Ok(cache_dir)
}

/// Compute a hash for the game version based on executable metadata
fn compute_game_version_hash(game_root: &Path) -> String {
    let mut hasher = Sha256::new();

    // Get the game executable name from BaseGame
    let exe_name = crate::base_game::game::get_executable_name();
    let exe_path = game_root.join(exe_name);

    if exe_path.exists() {
        // Hash the executable path
        hasher.update(exe_path.to_string_lossy().as_bytes());

        // Hash file metadata (size, modified time)
        if let Ok(metadata) = fs::metadata(&exe_path) {
            hasher.update(metadata.len().to_be_bytes());
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    hasher.update(duration.as_secs().to_be_bytes());
                }
            }
        }
    } else {
        // Fallback: hash the game root directory path and its metadata
        hasher.update(game_root.to_string_lossy().as_bytes());
        if let Ok(metadata) = fs::metadata(game_root) {
            if let Ok(modified) = metadata.modified() {
                if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                    hasher.update(duration.as_secs().to_be_bytes());
                }
            }
        }
    }

    format!("{:x}", hasher.finalize())
}

/// Get the cache file path for a specific game version
fn get_cache_file_path(game_version_hash: &str) -> Result<PathBuf, std::io::Error> {
    let cache_dir = get_cache_directory()?;
    Ok(cache_dir.join(format!("file_index_{}.json", game_version_hash)))
}

/// Load file index cache from disk
fn load_cache_from_disk(game_version_hash: &str) -> Option<FileIndexCache> {
    let cache_path = get_cache_file_path(game_version_hash).ok()?;

    if !cache_path.exists() {
        return None;
    }

    let cache_content = fs::read_to_string(&cache_path).ok()?;
    let cache: FileIndexCache = serde_json::from_str(&cache_content).ok()?;

    // Verify the cache is for the correct version
    if cache.is_valid_for_version(game_version_hash) {
        eprintln!(
            "Loaded file index cache from disk with {} files (created {})",
            cache.files.len(),
            cache.created_at
        );
        Some(cache)
    } else {
        // Cache is for wrong version, remove it
        let _ = fs::remove_file(&cache_path);
        None
    }
}

/// Save file index cache to disk
fn save_cache_to_disk(cache: &FileIndexCache) -> Result<(), std::io::Error> {
    let cache_path = get_cache_file_path(&cache.game_version_hash)?;
    let cache_content = serde_json::to_string_pretty(cache)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // Write to temporary file first, then rename for atomic operation
    let temp_path = cache_path.with_extension("tmp");
    fs::write(&temp_path, cache_content)?;
    fs::rename(&temp_path, &cache_path)?;

    eprintln!(
        "Saved file index cache to disk: {} ({} files)",
        cache_path.display(),
        cache.files.len()
    );

    Ok(())
}

/// Clean up old cache files (keep only the most recent 5)
fn cleanup_old_cache_files() {
    if let Ok(cache_dir) = get_cache_directory() {
        if let Ok(entries) = fs::read_dir(&cache_dir) {
            let mut cache_files: Vec<_> = entries
                .filter_map(|entry| entry.ok())
                .filter(|entry| {
                    entry
                        .path()
                        .extension()
                        .and_then(|ext| ext.to_str())
                        .map(|ext| ext == "json")
                        .unwrap_or(false)
                })
                .filter_map(|entry| {
                    entry.metadata().ok().and_then(|metadata| {
                        metadata
                            .modified()
                            .ok()
                            .map(|modified| (entry.path(), modified))
                    })
                })
                .collect();

            // Sort by modification time, newest first
            cache_files.sort_by(|a, b| b.1.cmp(&a.1));

            // Remove all but the 5 most recent
            for (old_cache_path, _) in cache_files.into_iter().skip(5) {
                let _ = fs::remove_file(&old_cache_path);
            }
        }
    }
}

/// Cache for file existence checks in the game directory and loaded mods
///
/// This cache significantly improves startup performance by avoiding expensive directory
/// scanning (37,000+ files) when the game hasn't changed. The cache is stored on disk
/// using the game executable's metadata as a version key, automatically invalidating
/// when the game is updated.
///
/// Cache location: %LOCALAPPDATA%\stelpatch\file_index\ (Windows)
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

        // Also clear disk cache
        if let Ok(cache_dir) = get_cache_directory() {
            if let Ok(entries) = fs::read_dir(&cache_dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
                        let _ = fs::remove_file(&path);
                    }
                }
            }
        }
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

        let game_root = if let Some(path) = crate::base_game::game::get_install_directory_windows()
        {
            path
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

        // Compute game version hash for cache key
        let game_version_hash = compute_game_version_hash(&game_root);

        // Try to load from disk cache first
        let files = if let Some(cached_data) = load_cache_from_disk(&game_version_hash) {
            eprintln!(
                "Using cached file index with {} files (saved {:?})",
                cached_data.files.len(),
                start.elapsed()
            );
            cached_data.files
        } else {
            eprintln!("No valid cache found, scanning directory...");
            let mut files = HashSet::new();

            if let Err(e) = Self::scan_directory_recursive(&game_root, &game_root, &mut files) {
                eprintln!("Warning: Failed to scan game directory: {}", e);
            }

            eprintln!(
                "Built file index cache with {} files in {:?}",
                files.len(),
                start.elapsed()
            );

            // Save to disk cache for next time
            let cache_data = FileIndexCache::new(files.clone(), game_version_hash);
            if let Err(e) = save_cache_to_disk(&cache_data) {
                eprintln!("Warning: Failed to save file index cache: {}", e);
            }

            // Clean up old cache files in background
            std::thread::spawn(|| {
                cleanup_old_cache_files();
            });

            files
        };

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

    pub fn get_all_files(&self) -> &HashSet<String> {
        &self.files
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

use std::{collections::HashMap, path::PathBuf};

use anyhow::anyhow;
use colored::*;
use glob::glob;
use rayon::prelude::*;

use crate::{ModDefinition, Module, Namespace};

#[derive(Debug, Clone)]
pub struct GameMod {
    pub definition: ModDefinition,
    pub namespaces: HashMap<String, Namespace>,
}

pub enum LoadMode {
    Serial,
    Parallel,
}

impl GameMod {
    pub fn new() -> Self {
        Self {
            definition: ModDefinition::new(),
            namespaces: HashMap::new(),
        }
    }

    pub fn with_definition(definition: ModDefinition) -> Self {
        Self {
            definition,
            namespaces: HashMap::new(),
        }
    }

    pub fn with_module(module: Module) -> Self {
        let mut game_mod = Self::new();
        game_mod.push(module);
        game_mod
    }

    pub fn push(&mut self, module: Module) -> &mut Self {
        let namespace = self
            .namespaces
            .entry(module.namespace.clone())
            .or_insert_with(|| Namespace::new(&module.namespace, None));

        namespace.insert(module);
        self
    }

    pub fn get_namespace(&self, namespace: &str) -> Option<&Namespace> {
        self.namespaces.get(namespace)
    }

    fn parse_serial(paths: &Vec<PathBuf>) -> Vec<Result<Module, anyhow::Error>> {
        paths
            .iter()
            .map(move |path| Module::from_file(path))
            .collect()
    }

    fn parse_parallel(paths: &Vec<PathBuf>) -> Vec<Result<Module, anyhow::Error>> {
        paths
            .par_iter()
            .map(move |path| Module::from_file(path))
            .collect()
    }

    pub fn load(definition: ModDefinition, mode: LoadMode) -> Result<Self, anyhow::Error> {
        let base_path = PathBuf::from(definition.path.as_ref().unwrap());

        // Define glob patterns for different file types
        // Support both Stellaris and Victoria 3 directory structures
        let glob_patterns = vec![
            // Stellaris patterns (simple structure)
            "common/**/*.txt",
            "interface/**/*.gui",
            "interface/**/*.gfx",
            "events/**/*.txt",
            "gfx/**/*.gfx",
            "gfx/**/*.asset",
            "gfx/**/*.txt",
            "flags/**/*.txt",
            "music/**/*.txt",
            "music/**/*.asset",
            "sound/**/*.txt",
            "sound/**/*.asset",
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

        // Define ignore patterns for files to exclude (simple filename matching)
        let ignore_filenames = vec![
            "99_README.txt",
            "HOW_TO_MAKE_NEW_SHIPS.txt",
            "readme.txt",
            "Readme.txt",
            "changelog.txt",
            "CHANGELOG.txt",
            "ChangeLog.txt",
            "license.txt",
            "LICENSE.txt",
            "credits.txt",
            "CREDITS.txt",
            "TODO.txt",
            "todo.txt",
            "info.txt",
        ];

        let mut paths = vec![];

        // Collect files matching all patterns
        for pattern in glob_patterns {
            let full_pattern = base_path.join(pattern);
            let pattern_str = full_pattern.to_string_lossy();

            match glob(&pattern_str) {
                Ok(paths_iter) => {
                    for entry in paths_iter {
                        match entry {
                            Ok(path) => {
                                if path.is_file() {
                                    // Check if this file should be ignored (simple filename matching)
                                    let should_ignore = if let Some(filename) = path.file_name() {
                                        if let Some(filename_str) = filename.to_str() {
                                            ignore_filenames.contains(&filename_str)
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    };

                                    if !should_ignore {
                                        paths.push(path);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error reading path in pattern {}: {}", pattern, e);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error with glob pattern {}: {}", pattern, e);
                }
            }
        }

        let mut mod_modules = vec![];

        let modules = match mode {
            LoadMode::Serial => Self::parse_serial(&paths),
            LoadMode::Parallel => Self::parse_parallel(&paths),
        };

        for (module, path) in modules.into_iter().zip(paths.iter()) {
            let module = module
                .map_err(|e| anyhow!("Failed to load module at {}: {}", path.display(), e))?;
            mod_modules.push(module);
        }

        let mut game_mod = Self::with_definition(definition);
        for module in mod_modules {
            game_mod.push(module);
        }

        Ok(game_mod)
    }

    pub fn print_contents(&self) {
        println!("{}", "Namespaces:".bold());
        for namespace in self.namespaces.values() {
            println!("  {}", namespace.namespace);
        }
    }
}

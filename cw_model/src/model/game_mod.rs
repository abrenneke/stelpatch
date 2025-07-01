use std::{collections::HashMap, path::PathBuf};

use anyhow::anyhow;
use colored::*;
use rayon::prelude::*;
use walkdir::WalkDir;

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
        let mut dir = PathBuf::from(definition.path.as_ref().unwrap());
        dir.push("common");
        let mut paths = vec![];

        for entry in WalkDir::new(&dir).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file()
                && entry.path().extension().unwrap_or_default() == "txt"
                && entry
                    .path()
                    .parent()
                    .map(|p| p.file_name().unwrap_or_default())
                    .unwrap_or_default()
                    != "common"
            {
                paths.push(entry.path().to_path_buf());
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

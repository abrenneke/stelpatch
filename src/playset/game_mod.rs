use std::{collections::HashMap, path::PathBuf};

use crate::cw_model::{Entity, Module, Namespace, Value};

use super::mod_definition::ModDefinition;
use anyhow::anyhow;
use colored::*;
use rayon::prelude::*;
use walkdir::WalkDir;

mod tests;

#[derive(Debug, Clone)]
pub struct GameMod {
    pub definition: ModDefinition,
    pub modules: Vec<Module>,
    pub namespaces: HashMap<String, Namespace>,

    module_lookup_by_path: HashMap<String, usize>,
}

impl GameMod {
    pub fn new(definition: ModDefinition) -> Self {
        Self {
            definition,
            modules: vec![],
            namespaces: HashMap::new(),
            module_lookup_by_path: HashMap::new(),
        }
    }

    pub fn push(&mut self, module: Module) -> () {
        let index = self.modules.len();

        let namespace = self
            .namespaces
            .entry(module.namespace.clone())
            .or_insert_with(|| Namespace::new(module.namespace.clone()));

        namespace.insert(&module);

        self.module_lookup_by_path.insert(module.path(), index);

        self.modules.push(module);
    }

    pub fn get_namespace(&self, namespace: &str) -> Option<&Namespace> {
        self.namespaces.get(namespace)
    }

    pub fn load_parallel(definition: ModDefinition) -> Result<Self, anyhow::Error> {
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
                let path = entry.path().to_string_lossy().to_string();
                paths.push(path);
            }
        }

        let modules: Vec<Result<Module, anyhow::Error>> = paths
            .par_iter()
            .map(|path| Module::parse_from_file(path))
            .collect();

        let mut mod_modules = vec![];

        for (module, path) in modules.into_iter().zip(paths.iter()) {
            let module = module.map_err(|e| anyhow!("Failed to load module at {}: {}", path, e))?;
            mod_modules.push(module);
        }

        let mut game_mod = Self::new(definition);
        for module in mod_modules {
            game_mod.push(module);
        }

        Ok(game_mod)
    }

    /// Gets a module by its path (namespace + filename), or None if it doesn't exist.
    /// For example, "common/units/units"
    pub fn get_by_path(&self, path: &str) -> Option<&Module> {
        self.module_lookup_by_path
            .get(path)
            .map(|index| &self.modules[*index])
    }

    /// Returns the sole entity for a type path, or None if there are none with the name.
    pub fn get_entity(&self, namespace: &str, name: &str) -> Option<&Entity> {
        self.get_namespace(namespace)?.get_entity(name)
    }

    pub fn get_overridden_modules(&self, other: &GameMod) -> Vec<&Module> {
        let mut overridden_modules = vec![];

        for module in &self.modules {
            if other.get_by_path(&module.path()).is_some() {
                overridden_modules.push(module);
            }
        }

        overridden_modules
    }

    pub fn get_overridden_modules_by_namespace(
        &self,
        other: &GameMod,
    ) -> HashMap<String, Namespace> {
        let mut namespaces = HashMap::new();

        for module in &self.modules {
            for other_module in &other.modules {
                if module.path() == other_module.path() {
                    let namespace = namespaces
                        .entry(module.namespace.clone())
                        .or_insert_with(|| Namespace::new(module.namespace.clone()));

                    namespace.insert(module);
                }
            }
        }
        namespaces
    }

    pub fn get_overridden_entities(&self, other: &GameMod) -> Vec<(String, String, Value)> {
        let mut overridden_entities = vec![];

        for module in &self.modules {
            for (entity_name, entity) in &module.entities {
                if other.get_entity(&module.namespace, &entity_name).is_some() {
                    overridden_entities.push((
                        module.namespace.to_owned(),
                        entity_name.to_owned(),
                        entity.to_owned(),
                    ));
                }
            }
        }

        overridden_entities
    }

    pub fn print_contents(&self) {
        println!("{}", "Namespaces:".bold());
        for namespace in self.namespaces.values() {
            println!("  {}", namespace.namespace);
        }

        println!("{}", "Modules:".bold());
        for module in self.modules.iter() {
            println!("  {}/{}", module.namespace, module.filename.bold());
        }

        println!("{}", "Entities:".bold());
        for namespace in self.namespaces.values() {
            for entity_name in namespace.entities.keys() {
                println!("  {}", entity_name);
            }
        }
    }
}

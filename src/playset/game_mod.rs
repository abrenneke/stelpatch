use std::path::PathBuf;

use crate::cw_model::{Entity, Module, NamedEntity};

use super::mod_definition::ModDefinition;
use anyhow::anyhow;
use rayon::prelude::*;
use walkdir::WalkDir;

mod tests;

pub struct GameMod {
    pub definition: ModDefinition,
    pub modules: Vec<Module>,

    module_lookup_by_path: std::collections::HashMap<String, usize>,
    module_lookup_by_type_path: std::collections::HashMap<String, Vec<usize>>,
}

impl GameMod {
    pub fn new(definition: ModDefinition) -> Self {
        Self {
            definition,
            modules: vec![],
            module_lookup_by_path: std::collections::HashMap::new(),
            module_lookup_by_type_path: std::collections::HashMap::new(),
        }
    }

    pub fn push(&mut self, module: Module) -> () {
        let index = self.modules.len();

        let path = format!("{}/{}", module.type_path.clone(), module.filename.clone());

        self.module_lookup_by_path.insert(path, index);

        let type_path_lookup = self
            .module_lookup_by_type_path
            .entry(module.type_path.clone())
            .or_insert(vec![]);

        type_path_lookup.push(index);

        self.modules.push(module);
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

    /// Gets a module by its path (type_path + filename), or None if it doesn't exist.
    /// For example, "common/units/units"
    pub fn get_by_path(&self, path: &str) -> Option<&Module> {
        self.module_lookup_by_path
            .get(path)
            .map(|index| &self.modules[*index])
    }

    /// Returns the sole entity for a type path, or None if there are none with the name.
    pub fn get_entity(&self, type_path: &str, name: &str) -> Option<&Entity> {
        let modules = self.module_lookup_by_type_path.get(type_path)?;

        for module_index in modules {
            let module = &self.modules[*module_index];

            if let Some(entity) = module.get_entity(name) {
                return Some(entity.entity());
            }
        }

        None
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

    pub fn get_overridden_entities(&self, other: &GameMod) -> Vec<NamedEntity> {
        let mut overridden_entities = vec![];

        for module in &self.modules {
            for entity in module.entities() {
                if other.get_entity(&module.type_path, &entity.1).is_some() {
                    overridden_entities.push(entity);
                }
            }
        }

        overridden_entities
    }
}

use std::{collections::HashMap, path::PathBuf, sync::Arc};

use crate::cw_model::{Module, Namespace};

use super::mod_definition::ModDefinition;
use anyhow::anyhow;
use colored::*;
use lasso::{Spur, ThreadedRodeo};
use rayon::prelude::*;
use walkdir::WalkDir;

mod tests;

#[derive(Debug, Clone)]
pub struct GameMod {
    pub definition: ModDefinition,
    pub modules: Vec<Module>,
    pub namespaces: HashMap<Spur, Namespace>,

    module_lookup_by_path: HashMap<Spur, usize>,
}

pub enum LoadMode {
    Serial,
    Parallel,
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

    pub fn push(&mut self, module: Module, interner: &ThreadedRodeo) -> () {
        let index = self.modules.len();

        let namespace = self
            .namespaces
            .entry(module.namespace.clone())
            .or_insert_with(|| {
                let ns_spur = interner.resolve(&module.namespace).to_owned();
                Namespace::new(&ns_spur, None, interner)
            });

        self.module_lookup_by_path
            .insert(interner.get_or_intern(module.path(interner)), index);
        namespace.insert(module);

        // self.modules.push(module);
    }

    pub fn get_namespace(&self, namespace: &Spur) -> Option<&Namespace> {
        self.namespaces.get(namespace)
    }

    fn parse_serial(
        paths: &Vec<PathBuf>,
        interner: Arc<ThreadedRodeo>,
    ) -> Vec<Result<Module, anyhow::Error>> {
        paths
            .iter()
            .map(move |path| Module::parse_from_file(path, &interner))
            .collect()
    }

    fn parse_parallel(
        paths: &Vec<PathBuf>,
        interner: Arc<ThreadedRodeo>,
    ) -> Vec<Result<Module, anyhow::Error>> {
        paths
            .par_iter()
            .map(move |path| Module::parse_from_file(path, &interner))
            .collect()
    }

    pub fn load(
        definition: ModDefinition,
        mode: LoadMode,
        interner: Arc<ThreadedRodeo>,
    ) -> Result<Self, anyhow::Error> {
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
            LoadMode::Serial => Self::parse_serial(&paths, interner.clone()),
            LoadMode::Parallel => Self::parse_parallel(&paths, interner.clone()),
        };

        for (module, path) in modules.into_iter().zip(paths.iter()) {
            let module = module
                .map_err(|e| anyhow!("Failed to load module at {}: {}", path.display(), e))?;
            mod_modules.push(module);
        }

        let mut game_mod = Self::new(definition);
        for module in mod_modules {
            game_mod.push(module, &interner.clone());
        }

        Ok(game_mod)
    }

    /// Gets a module by its path (namespace + filename), or None if it doesn't exist.
    /// For example, "common/units/units"
    pub fn get_by_path(&self, path: &Spur) -> Option<&Module> {
        self.module_lookup_by_path
            .get(path)
            .map(|index| &self.modules[*index])
    }

    /// Returns the sole entity for a type path, or None if there are none with the name.
    // pub fn get_entity(&self, namespace: &str, name: &str) -> Option<&Entity> {
    // self.get_namespace(namespace)?.get_entity(name)
    // }

    pub fn get_overridden_modules(
        &self,
        other: &GameMod,
        interner: &ThreadedRodeo,
    ) -> Vec<&Module> {
        let mut overridden_modules = vec![];

        for module in &self.modules {
            if other
                .get_by_path(&interner.get_or_intern(module.path(interner)))
                .is_some()
            {
                overridden_modules.push(module);
            }
        }

        overridden_modules
    }

    /// Gets the modules that are completely overridden (same file name) by another mod. Groups by namespace.
    // pub fn get_overridden_modules_by_namespace(
    //     &self,
    //     other: &GameMod,
    // ) -> HashMap<String, Namespace> {
    //     let mut namespaces = HashMap::new();

    //     for module in &self.modules {
    //         for other_module in &other.modules {
    //             if module.path() == other_module.path() {
    //                 let namespace = namespaces
    //                     .entry(module.namespace.clone())
    //                     .or_insert_with(|| Namespace::new(&module.namespace, None));

    //                 namespace.insert(module);
    //             }
    //         }
    //     }
    //     namespaces
    // }

    // pub fn get_overridden_entities(&self, other: &GameMod) -> Vec<(String, String, Value)> {
    //     let mut overridden_entities = vec![];

    //     for module in &self.modules {
    //         for (entity_name, entity) in &module.entities {
    //             if other.get_entity(&module.namespace, &entity_name).is_some() {
    //                 overridden_entities.push((
    //                     module.namespace.to_owned(),
    //                     entity_name.to_owned(),
    //                     entity.to_owned(),
    //                 ));
    //             }
    //         }
    //     }

    //     overridden_entities
    // }

    pub fn print_contents(&self, resolver: &ThreadedRodeo) {
        println!("{}", "Namespaces:".bold());
        for namespace in self.namespaces.values() {
            println!("  {}", resolver.resolve(&namespace.namespace));
        }

        println!("{}", "Modules:".bold());
        for module in self.modules.iter() {
            println!(
                "  {}/{}",
                resolver.resolve(&module.namespace),
                resolver.resolve(&module.filename).bold()
            );
        }

        // println!("{}", "Entities:".bold());
        // for namespace in self.namespaces.values() {
        //     for entity_name in namespace.entities.keys() {
        //         println!("  {}", entity_name);
        //     }
        // }
    }
}

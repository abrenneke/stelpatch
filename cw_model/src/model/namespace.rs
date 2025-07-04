use std::collections::HashMap;

use crate::{EntityMergeMode, Module, Properties, Value};

/// A Namespace is the path to the folder containing module files in the `common` directory. Maybe other directories too.
/// E.g. common/armies is the namespace, and contains modules with unique names. All modules in a namespace are combined together following
/// the rules above in Module.
#[derive(Debug, PartialEq, Clone)]
pub struct Namespace {
    pub namespace: String,
    pub properties: Properties,
    pub values: Vec<Value>,
    pub modules: HashMap<String, Module>,
    pub merge_mode: EntityMergeMode,
}

impl Namespace {
    pub fn new(namespace: &str, merge_mode: Option<EntityMergeMode>) -> Self {
        let ns = Self {
            namespace: namespace.to_string(),
            properties: Properties::new_module(),
            values: Vec::new(),
            modules: HashMap::new(),
            merge_mode: merge_mode.unwrap_or(EntityMergeMode::Unknown),
        };

        ns
    }

    pub fn insert(&mut self, module: Module) -> &Self {
        // TODO: properties should follow the merge mode, technically, but it's unlikely a single
        // mod will define the same property twice in the same namespace, so for now we can treat it like
        // EntityMergeMode::LIOS
        self.properties.kv.extend(module.properties.kv.clone());
        self.values.extend(module.values.clone());

        self.modules.insert(module.filename.clone(), module);

        self
    }

    pub fn get_module(&self, module_name: &str) -> Option<&Module> {
        self.modules.get(module_name)
    }

    pub fn get_only(&self, key: &str) -> Option<&Value> {
        if let Some(value) = self.properties.kv.get(key) {
            if value.0.len() == 1 {
                return Some(&value.0[0].value);
            }
        }
        None
    }

    // pub fn get_entity(&self, entity_name: &str) -> Option<&Entity> {
    // self.entities.get(entity_name).map(|v| v.entity())
    // }
}

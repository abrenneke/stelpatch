use crate::handlers::cache::EntityRestructurer;
use cw_model::types::CwtAnalyzer;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Shared utilities for resolver modules
pub struct ResolverUtils {
    cwt_analyzer: Arc<CwtAnalyzer>,
    namespace_cache: Arc<RwLock<HashMap<String, Option<Arc<HashSet<String>>>>>>,
}

impl ResolverUtils {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        Self {
            cwt_analyzer,
            namespace_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get namespace keys for a TypeRef alias name
    pub fn get_namespace_keys_for_type_ref(&self, type_name: &str) -> Option<Arc<HashSet<String>>> {
        if let Some(cached_result) = self.namespace_cache.read().unwrap().get(type_name) {
            return cached_result.clone();
        }

        if let Some(type_def) = self.cwt_analyzer.get_type(type_name) {
            if let Some(path) = type_def.path.as_ref() {
                let path = path.trim_start_matches("game/");
                if let Some(namespace_keys) =
                    EntityRestructurer::get_namespace_entity_keys_set(&path)
                {
                    let result = Some(namespace_keys);
                    self.namespace_cache
                        .write()
                        .unwrap()
                        .insert(type_name.to_string(), result.clone());
                    return result;
                }
            }
        }
        let result = None;
        self.namespace_cache
            .write()
            .unwrap()
            .insert(type_name.to_string(), result.clone());
        result
    }

    /// Get access to the CWT analyzer
    pub fn cwt_analyzer(&self) -> &CwtAnalyzer {
        &self.cwt_analyzer
    }
}

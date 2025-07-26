use crate::handlers::cache::EntityRestructurer;
use crate::interner::get_interner;
use cw_model::types::CwtAnalyzer;
use lasso::Spur;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, RwLock};

/// Shared utilities for resolver modules
pub struct ResolverUtils {
    cwt_analyzer: Arc<CwtAnalyzer>,
    namespace_cache: Arc<RwLock<HashMap<Spur, Option<Arc<HashSet<Spur>>>>>>,
}

impl ResolverUtils {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>) -> Self {
        Self {
            cwt_analyzer,
            namespace_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Get namespace keys for a TypeRef alias name
    pub fn get_namespace_keys_for_type_ref(&self, type_name: Spur) -> Option<Arc<HashSet<Spur>>> {
        if let Some(cached_result) = self.namespace_cache.read().unwrap().get(&type_name) {
            return cached_result.clone();
        }

        if let Some(type_def) = self.cwt_analyzer.get_type(type_name) {
            if let Some(path) = type_def.path.as_ref() {
                let interner = get_interner();
                let path = get_interner().resolve(path);
                let path = path.trim_start_matches("game/");
                let path = interner.get_or_intern(path);
                if let Some(namespace_keys) =
                    EntityRestructurer::get_namespace_entity_keys_set(path)
                {
                    let result = Some(namespace_keys);
                    self.namespace_cache
                        .write()
                        .unwrap()
                        .insert(type_name, result.clone());
                    return result;
                }
            }
        }
        let result = None;
        self.namespace_cache
            .write()
            .unwrap()
            .insert(type_name, result.clone());
        result
    }

    /// Get access to the CWT analyzer
    pub fn cwt_analyzer(&self) -> &CwtAnalyzer {
        &self.cwt_analyzer
    }
}

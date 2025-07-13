use cw_model::{AliasDefinition, CwtType};
use std::collections::HashMap;
use std::sync::Arc;

pub struct TypeResolverCache {
    pub resolved_references: HashMap<String, Arc<CwtType>>,
    pub alias_match_left: HashMap<String, (CwtType, Option<AliasDefinition>)>,
    pub pattern_type_matches: HashMap<String, bool>,
}

impl TypeResolverCache {
    pub fn new() -> Self {
        Self {
            resolved_references: HashMap::new(),
            alias_match_left: HashMap::new(),
            pattern_type_matches: HashMap::new(),
        }
    }
}

impl Default for TypeResolverCache {
    fn default() -> Self {
        Self::new()
    }
}

use cw_model::CwtType;
use std::collections::HashMap;
use std::sync::Arc;

pub struct TypeResolverCache {
    pub resolved_references: HashMap<String, Arc<CwtType>>,
}

impl TypeResolverCache {
    pub fn new() -> Self {
        Self {
            resolved_references: HashMap::new(),
        }
    }
}

impl Default for TypeResolverCache {
    fn default() -> Self {
        Self::new()
    }
}

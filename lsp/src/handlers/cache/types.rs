use std::sync::Arc;

use crate::handlers::scoped_type::ScopedType;

/// Information about a property's type
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub property_path: String,
    pub scoped_type: Option<Arc<ScopedType>>,
    pub documentation: Option<String>,
    pub source_info: Option<String>,
}

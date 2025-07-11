use crate::handlers::scoped_type::ScopedType;

/// Information about a property's type
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub property_path: String,
    pub type_description: String,
    pub scoped_type: Option<ScopedType>,
    pub documentation: Option<String>,
    pub source_info: Option<String>,
}

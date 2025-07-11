use crate::handlers::scoped_type::CwtTypeOrSpecial;

/// Information about a property's type
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub property_path: String,
    pub type_description: String,
    pub cwt_type: Option<CwtTypeOrSpecial>,
    pub documentation: Option<String>,
    pub source_info: Option<String>,
}

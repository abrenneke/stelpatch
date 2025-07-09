use cw_model::CwtType;

/// Information about a property's type
#[derive(Debug, Clone)]
pub struct TypeInfo {
    pub property_path: String,
    pub type_description: String,
    pub cwt_type: Option<CwtType>,
    pub documentation: Option<String>,
    pub source_info: Option<String>,
}

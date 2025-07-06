/// Configuration for type inference behavior
#[derive(Debug, Clone)]
pub struct TypeInferenceConfig {
    /// Maximum number of literal values before converting to a string type
    pub max_literals: usize,

    /// Whether to infer boolean types from yes/no values
    pub infer_booleans: bool,

    /// Whether to merge similar object types
    pub merge_objects: bool,

    /// Whether to prefer arrays over single values when multiple values are present
    pub prefer_arrays: bool,
}

impl Default for TypeInferenceConfig {
    fn default() -> Self {
        Self {
            max_literals: 10,
            infer_booleans: true,
            merge_objects: true,
            prefer_arrays: false,
        }
    }
}

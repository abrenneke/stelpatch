use super::ResolverUtils;
use cw_model::types::{CwtAnalyzer, PatternProperty, PatternType};
use cw_model::{AliasName, BlockType};
use std::sync::Arc;

pub struct PatternMatcher {
    pub cwt_analyzer: Arc<CwtAnalyzer>,
    pub utils: Arc<ResolverUtils>,
}

impl PatternMatcher {
    pub fn new(cwt_analyzer: Arc<CwtAnalyzer>, utils: Arc<ResolverUtils>) -> Self {
        Self {
            cwt_analyzer,
            utils,
        }
    }

    /// Check if a key matches any pattern property in a block
    pub fn key_matches_pattern<'b>(
        &self,
        key: &str,
        block_type: &'b BlockType,
    ) -> Option<&'b PatternProperty> {
        for pattern_property in &block_type.pattern_properties {
            if self.key_matches_pattern_type(key, &pattern_property.pattern_type) {
                return Some(pattern_property);
            }
        }
        None
    }

    /// Check if a key matches a specific pattern type
    pub fn key_matches_pattern_type(&self, key: &str, pattern_type: &PatternType) -> bool {
        match pattern_type {
            PatternType::AliasName { category } => {
                // Check if the key matches any alias name from this category
                if let Some(aliases_in_category) =
                    self.cwt_analyzer.get_aliases_for_category(category)
                {
                    for alias_pattern in aliases_in_category {
                        match &alias_pattern.name {
                            AliasName::Static(name) => {
                                if name == key {
                                    return true;
                                }
                            }
                            AliasName::TypeRef(type_name) => {
                                // Check if key matches any type from this namespace
                                if let Some(namespace_keys) =
                                    self.utils.get_namespace_keys_for_type_ref(type_name)
                                {
                                    if namespace_keys.contains(&key.to_string()) {
                                        return true;
                                    }
                                }
                            }
                            AliasName::Enum(enum_name) => {
                                // Check if key matches any enum value
                                if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                    if enum_def.values.contains(key) {
                                        return true;
                                    }
                                }
                            }
                        }
                    }
                }
                false
            }
            PatternType::Enum { key: enum_key } => {
                // Check if the key matches any enum value
                if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_key) {
                    enum_def.values.contains(key)
                } else {
                    false
                }
            }
        }
    }

    /// Get all possible completions for a pattern type
    pub fn get_pattern_completions(&self, pattern_type: &PatternType) -> Vec<String> {
        match pattern_type {
            PatternType::AliasName { category } => {
                let mut completions = Vec::new();
                if let Some(aliases_in_category) =
                    self.cwt_analyzer.get_aliases_for_category(category)
                {
                    for alias_pattern in aliases_in_category {
                        match &alias_pattern.name {
                            AliasName::Static(name) => {
                                completions.push(name.clone());
                            }
                            AliasName::TypeRef(type_name) => {
                                if let Some(namespace_keys) =
                                    self.utils.get_namespace_keys_for_type_ref(type_name)
                                {
                                    completions.extend(namespace_keys.iter().cloned());
                                }
                            }
                            AliasName::Enum(enum_name) => {
                                if let Some(enum_def) = self.cwt_analyzer.get_enum(enum_name) {
                                    completions.extend(enum_def.values.iter().cloned());
                                }
                            }
                        }
                    }
                }
                completions
            }
            PatternType::Enum { key } => {
                if let Some(enum_def) = self.cwt_analyzer.get_enum(key) {
                    enum_def.values.iter().cloned().collect()
                } else {
                    Vec::new()
                }
            }
        }
    }
}

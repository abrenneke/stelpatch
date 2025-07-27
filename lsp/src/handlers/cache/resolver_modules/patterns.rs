use crate::interner::get_interner;

use super::{ResolverUtils, SubtypeHandler};
use cw_model::types::{CwtAnalyzer, PatternProperty, PatternType};
use cw_model::{AliasName, BlockType};
use lasso::Spur;
use std::sync::Arc;

pub struct PatternMatcher {
    pub cwt_analyzer: Arc<CwtAnalyzer>,
    pub utils: Arc<ResolverUtils>,
    pub subtype_handler: Arc<SubtypeHandler>,
}

impl PatternMatcher {
    pub fn new(
        cwt_analyzer: Arc<CwtAnalyzer>,
        utils: Arc<ResolverUtils>,
        subtype_handler: Arc<SubtypeHandler>,
    ) -> Self {
        Self {
            cwt_analyzer,
            utils,
            subtype_handler,
        }
    }

    /// Check if a key matches any pattern property in a block
    pub fn key_matches_pattern<'b>(
        &self,
        key: Spur,
        block_type: &'b BlockType,
    ) -> Option<&'b PatternProperty> {
        for pattern_property in &block_type.pattern_properties {
            if self.key_matches_pattern_type(key, &pattern_property.pattern_type) {
                return Some(pattern_property);
            }
        }
        None
    }

    /// Check if a key matches any pattern properties in a block and return ALL matches
    pub fn key_matches_all_patterns<'b>(
        &self,
        key: Spur,
        block_type: &'b BlockType,
    ) -> Vec<&'b PatternProperty> {
        let mut matches = Vec::new();
        for pattern_property in &block_type.pattern_properties {
            if self.key_matches_pattern_type(key, &pattern_property.pattern_type) {
                matches.push(pattern_property);
            }
        }
        matches
    }

    /// Check if a key matches a specific pattern type
    pub fn key_matches_pattern_type(&self, key: Spur, pattern_type: &PatternType) -> bool {
        let interner = get_interner();
        match pattern_type {
            PatternType::AliasName { category } => {
                // Check if the key matches any alias name from this category
                if let Some(aliases_in_category) =
                    self.cwt_analyzer.get_aliases_for_category(*category)
                {
                    for alias_pattern in aliases_in_category {
                        match &alias_pattern.name {
                            AliasName::Static(name) => {
                                if *name == key {
                                    return true;
                                }
                            }
                            AliasName::TypeRef(type_name) => {
                                // Check if key matches any type from this namespace
                                if let Some(namespace_keys) =
                                    self.utils.get_namespace_keys_for_type_ref(*type_name)
                                {
                                    if namespace_keys.contains(&key) {
                                        return true;
                                    }
                                }
                            }
                            AliasName::Enum(enum_name) => {
                                // Check if key matches any enum value
                                if let Some(enum_def) = self.cwt_analyzer.get_enum(*enum_name) {
                                    if enum_def.values.contains(&key) {
                                        return true;
                                    }
                                }
                            }
                            AliasName::TypeRefWithPrefixSuffix(name, prefix, suffix) => {
                                // Check if key matches pattern with prefix/suffix
                                let key = interner.resolve(&key);
                                let mut stripped_key = key;

                                // Remove prefix if present
                                if let Some(prefix_str) = prefix {
                                    if let Some(without_prefix) =
                                        stripped_key.strip_prefix(interner.resolve(&prefix_str))
                                    {
                                        stripped_key = without_prefix;
                                    } else {
                                        return false; // Key doesn't start with required prefix
                                    }
                                }

                                // Remove suffix if present
                                if let Some(suffix_str) = suffix {
                                    if let Some(without_suffix) =
                                        stripped_key.strip_suffix(interner.resolve(&suffix_str))
                                    {
                                        stripped_key = without_suffix;
                                    } else {
                                        return false; // Key doesn't end with required suffix
                                    }
                                }

                                // Check if the remaining key matches any type from this namespace
                                if let Some(namespace_keys) =
                                    self.utils.get_namespace_keys_for_type_ref(*name)
                                {
                                    if namespace_keys
                                        .contains(&interner.get_or_intern(stripped_key))
                                    {
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
                if let Some(enum_def) = self.cwt_analyzer.get_enum(*enum_key) {
                    enum_def.values.contains(&key)
                } else {
                    false
                }
            }
            PatternType::Type { key: type_key } => {
                // Check if this is a subtype reference (contains a dot)
                let type_key_str = interner.resolve(&type_key);
                if let Some(dot_pos) = type_key_str.find('.') {
                    let (base_type, subtype) = type_key_str.split_at(dot_pos);
                    let subtype = &subtype[1..]; // Remove the leading dot

                    let base_type = interner.get_or_intern(base_type);

                    // Get the base type definition
                    let type_def = self.cwt_analyzer.get_type(base_type);

                    if let Some(type_def) = type_def {
                        if let Some(path) = type_def.path.as_ref() {
                            // CWT paths are prefixed with "game/"
                            let path = interner.resolve(&path).trim_start_matches("game/");

                            // Get the CWT type for this namespace
                            if let Some(cwt_type) = self.cwt_analyzer.get_type(base_type) {
                                // Use subtype handler to filter entities by subtype
                                let filtered_keys = self
                                    .subtype_handler
                                    .get_entity_keys_in_namespace_for_subtype(
                                        interner.get_or_intern(path),
                                        &cwt_type.rules,
                                        interner.get_or_intern(subtype),
                                    );

                                return filtered_keys.contains(&key);
                            }
                        }
                    }

                    // If subtype filtering failed, fall back to false
                    return false;
                }

                // Handle regular type references (no subtype)
                if let Some(namespace_keys) = self.utils.get_namespace_keys_for_type_ref(*type_key)
                {
                    namespace_keys.contains(&key)
                } else {
                    false
                }
            }
        }
    }

    /// Get all possible completions for a pattern type
    pub fn get_pattern_completions(&self, pattern_type: &PatternType) -> Vec<String> {
        let interner = get_interner();
        match pattern_type {
            PatternType::AliasName { category } => {
                let mut completions = Vec::new();
                if let Some(aliases_in_category) =
                    self.cwt_analyzer.get_aliases_for_category(*category)
                {
                    for alias_pattern in aliases_in_category {
                        match &alias_pattern.name {
                            AliasName::Static(name) => {
                                completions.push(interner.resolve(name).to_string());
                            }
                            AliasName::TypeRef(type_name) => {
                                if let Some(namespace_keys) =
                                    self.utils.get_namespace_keys_for_type_ref(*type_name)
                                {
                                    completions.extend(
                                        namespace_keys
                                            .iter()
                                            .cloned()
                                            .map(|k| interner.resolve(&k).to_string()),
                                    );
                                }
                            }
                            AliasName::Enum(enum_name) => {
                                if let Some(enum_def) = self.cwt_analyzer.get_enum(*enum_name) {
                                    completions.extend(
                                        enum_def
                                            .values
                                            .iter()
                                            .cloned()
                                            .map(|v| interner.resolve(&v).to_string()),
                                    );
                                }
                            }
                            AliasName::TypeRefWithPrefixSuffix(type_name, prefix, suffix) => {
                                if let Some(namespace_keys) =
                                    self.utils.get_namespace_keys_for_type_ref(*type_name)
                                {
                                    for key in namespace_keys.iter() {
                                        let completion = match (prefix, suffix) {
                                            (Some(p), Some(s)) => format!(
                                                "{}{}{}",
                                                interner.resolve(p),
                                                interner.resolve(key),
                                                interner.resolve(s)
                                            ),
                                            (Some(p), None) => format!(
                                                "{}{}",
                                                interner.resolve(p),
                                                interner.resolve(key)
                                            ),
                                            (None, Some(s)) => format!(
                                                "{}{}",
                                                interner.resolve(key),
                                                interner.resolve(s)
                                            ),
                                            (None, None) => interner.resolve(key).to_string(),
                                        };
                                        completions.push(completion);
                                    }
                                }
                            }
                        }
                    }
                }
                completions
            }
            PatternType::Enum { key } => {
                if let Some(enum_def) = self.cwt_analyzer.get_enum(*key) {
                    enum_def
                        .values
                        .iter()
                        .cloned()
                        .map(|v| interner.resolve(&v).to_string())
                        .collect()
                } else {
                    Vec::new()
                }
            }
            PatternType::Type { key } => {
                // Check if this is a subtype reference (contains a dot)
                let key_str = interner.resolve(&key);
                if let Some(dot_pos) = key_str.find('.') {
                    let (base_type, subtype) = key_str.split_at(dot_pos);
                    let subtype = &subtype[1..]; // Remove the leading dot

                    let base_type = interner.get_or_intern(base_type);

                    // Get the base type definition
                    let type_def = self.cwt_analyzer.get_type(base_type);

                    if let Some(type_def) = type_def {
                        if let Some(path) = type_def.path.as_ref() {
                            let path = interner.resolve(path);
                            // CWT paths are prefixed with "game/"
                            let path = path.trim_start_matches("game/");
                            let path = interner.get_or_intern(path);

                            // Get the CWT type for this namespace
                            if let Some(cwt_type) = self.cwt_analyzer.get_type(base_type) {
                                // Use subtype handler to filter entities by subtype
                                let filtered_keys = self
                                    .subtype_handler
                                    .get_entity_keys_in_namespace_for_subtype(
                                        path,
                                        &cwt_type.rules,
                                        interner.get_or_intern(subtype),
                                    );

                                return filtered_keys
                                    .iter()
                                    .map(|k| interner.resolve(k).to_string())
                                    .collect();
                            }
                        }
                    }

                    // If subtype filtering failed, return empty
                    return Vec::new();
                }

                // Handle regular type references (no subtype)
                if let Some(namespace_keys) = self.utils.get_namespace_keys_for_type_ref(*key) {
                    namespace_keys
                        .iter()
                        .cloned()
                        .map(|k| interner.resolve(&k).to_string())
                        .collect()
                } else {
                    Vec::new()
                }
            }
        }
    }
}

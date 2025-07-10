use cw_model::CwtType;
use cw_model::TypeFingerprint;
use cw_parser::{AstEntityItem, AstModule, AstNode, AstValue};
use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::lsp_types::*;

use crate::handlers::cache::GameDataCache;

use super::cache::TypeCache;
use super::utils::extract_namespace_from_uri;

/// Provider for generating diagnostics with shared state
pub struct DiagnosticsProvider<'client> {
    client: &'client Client,
    documents: Arc<RwLock<HashMap<String, String>>>,
}

impl<'client> DiagnosticsProvider<'client> {
    /// Create a new diagnostics provider
    pub fn new(client: &'client Client, documents: Arc<RwLock<HashMap<String, String>>>) -> Self {
        Self { client, documents }
    }

    /// Generate diagnostics for a document by attempting to parse it and type-check it
    pub async fn generate_diagnostics(&self, uri: &str) {
        let start_time = Instant::now();
        eprintln!("DEBUG: Starting diagnostics generation for {}", uri);

        let documents_guard = self.documents.read().await;
        if let Some(content) = documents_guard.get(uri) {
            let mut diagnostics = Vec::new();

            // First, try to parse the content
            let parse_start = Instant::now();
            let mut module = AstModule::new();
            match module.parse_input(content) {
                Ok(()) => {
                    let parse_duration = parse_start.elapsed();
                    eprintln!(
                        "DEBUG: Parsing completed in {:?} for {}",
                        parse_duration, uri
                    );

                    // If parsing succeeds, do type checking
                    let type_check_start = Instant::now();
                    let type_diagnostics =
                        self.generate_type_diagnostics(&module, uri, content).await;
                    let type_check_duration = type_check_start.elapsed();
                    eprintln!(
                        "DEBUG: Type checking completed in {:?} for {}",
                        type_check_duration, uri
                    );

                    diagnostics.extend(type_diagnostics);
                }
                Err(error) => {
                    let parse_duration = parse_start.elapsed();
                    eprintln!("DEBUG: Parsing failed in {:?} for {}", parse_duration, uri);

                    // If parsing fails, add parsing error
                    let diagnostic = Self::create_diagnostic_from_parse_error(&error, content);
                    diagnostics.push(diagnostic);
                }
            }

            // Publish diagnostics to the client
            self.client
                .publish_diagnostics(Url::parse(uri).unwrap(), diagnostics, None)
                .await;

            let total_duration = start_time.elapsed();
            eprintln!(
                "DEBUG: Total diagnostics generation completed in {:?} for {}",
                total_duration, uri
            );
        } else {
            eprintln!("DEBUG: No content found for {}", uri);
        }
    }

    /// Generate type-checking diagnostics for a successfully parsed document
    async fn generate_type_diagnostics(
        &self,
        module: &AstModule<'_>,
        uri: &str,
        content: &str,
    ) -> Vec<Diagnostic> {
        let type_check_start = Instant::now();
        let mut diagnostics = Vec::new();

        // Check if type cache is initialized
        if !TypeCache::is_initialized() {
            eprintln!(
                "DEBUG: TypeCache not initialized, skipping diagnostics for {}",
                uri
            );
            return diagnostics;
        }

        if !GameDataCache::is_initialized() {
            eprintln!(
                "DEBUG: GameDataCache not initialized, skipping diagnostics for {}",
                uri
            );
            return diagnostics;
        }

        // Extract namespace from URI
        let namespace_start = Instant::now();
        let namespace = match extract_namespace_from_uri(uri) {
            Some(ns) => {
                eprintln!("DEBUG: Extracted namespace '{}' from URI {}", ns, uri);
                ns
            }
            None => {
                eprintln!(
                    "DEBUG: Could not extract namespace from URI {}, skipping diagnostics",
                    uri
                );
                return diagnostics;
            }
        };
        let namespace_duration = namespace_start.elapsed();
        eprintln!(
            "DEBUG: Namespace extraction took {:?} for {}",
            namespace_duration, uri
        );

        // Get type information for this namespace
        let type_info_start = Instant::now();
        let type_info = match super::cache::get_namespace_entity_type(&namespace).await {
            Some(info) => {
                eprintln!("DEBUG: Retrieved type info for namespace '{}'", namespace);
                info
            }
            None => {
                eprintln!(
                    "DEBUG: No type info available for namespace '{}'",
                    namespace
                );
                return diagnostics;
            }
        };
        let type_info_duration = type_info_start.elapsed();
        eprintln!(
            "DEBUG: Type info retrieval took {:?} for namespace '{}'",
            type_info_duration, namespace
        );

        let namespace_type = match &type_info.cwt_type {
            Some(t) => {
                eprintln!("DEBUG: Found concrete type for namespace '{}'", namespace);
                t
            }
            None => {
                eprintln!(
                    "DEBUG: No concrete type available for namespace '{}'",
                    namespace
                );
                return diagnostics;
            }
        };

        eprintln!(
            "DEBUG: Starting validation for {} items in namespace '{}'",
            module.items.len(),
            namespace
        );

        // Validate each entity in the module
        let validation_start = Instant::now();
        for item in &module.items {
            if let AstEntityItem::Expression(expr) = item {
                let entity_name = expr.key.raw_value();

                eprintln!(
                    "DEBUG: Validating entity '{}' in namespace '{}'",
                    entity_name, namespace
                );

                // Top-level keys are entity names - they can be anything, so don't validate them
                // Instead, validate their VALUES against the namespace structure
                let entity_diagnostics =
                    self.validate_entity_value(&expr.value, namespace_type, content, &namespace, 0);
                diagnostics.extend(entity_diagnostics);
            }
        }
        let validation_duration = validation_start.elapsed();
        eprintln!(
            "DEBUG: Entity validation took {:?} for namespace '{}'",
            validation_duration, namespace
        );

        let total_type_check_duration = type_check_start.elapsed();
        eprintln!(
            "DEBUG: Generated {} diagnostics in {:?} for namespace '{}'",
            diagnostics.len(),
            total_type_check_duration,
            namespace
        );
        diagnostics
    }

    /// Validate an entity value against the expected type structure
    fn validate_entity_value(
        &self,
        value: &AstValue<'_>,
        expected_type: &CwtType,
        content: &str,
        namespace: &str,
        depth: usize,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Prevent infinite recursion
        if depth > 10 {
            eprintln!("DEBUG: Max recursion depth reached at depth {}", depth);
            return diagnostics;
        }

        match value {
            AstValue::Entity(entity) => {
                // Validate each property in the entity
                for item in &entity.items {
                    if let AstEntityItem::Expression(expr) = item {
                        let key_name = expr.key.raw_value();

                        eprintln!("DEBUG: Validating key '{}'", key_name);

                        // Check if this key is valid for the expected type
                        if !Self::is_key_valid(expected_type, key_name) {
                            let diagnostic = Self::create_unexpected_key_diagnostic(
                                expr.key.span_range(),
                                key_name,
                                namespace,
                                content,
                            );
                            diagnostics.push(diagnostic);
                        } else {
                            // Get the expected type for this key
                            let property_type =
                                Self::get_property_type_from_expected_type(expected_type, key_name);

                            // Validate the value against the property type
                            let value_diagnostics = self.validate_value_against_type(
                                &expr.value,
                                &property_type,
                                content,
                                namespace,
                                depth + 1,
                            );
                            diagnostics.extend(value_diagnostics);
                        }
                    }
                }
            }
            _ => {
                // For non-entity values, validate the value directly against the expected type
                let value_diagnostics = self.validate_value_against_type(
                    value,
                    expected_type,
                    content,
                    namespace,
                    depth + 1,
                );
                diagnostics.extend(value_diagnostics);
            }
        }

        diagnostics
    }

    /// Validate a value against the expected CWT type
    fn validate_value_against_type(
        &self,
        value: &AstValue<'_>,
        expected_type: &CwtType,
        content: &str,
        namespace: &str,
        depth: usize,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Prevent infinite recursion
        if depth > 10 {
            eprintln!("DEBUG: Max recursion depth reached at depth {}", depth);
            return diagnostics;
        }

        if !TypeCache::is_initialized() {
            return diagnostics;
        }

        let cache = TypeCache::get().unwrap();
        let resolved_type = cache.resolve_type(expected_type);

        match (&resolved_type, value) {
            // Block type validation
            (CwtType::Block(_), AstValue::Entity(_)) => {
                // For block types, validate the entity structure recursively
                let entity_diagnostics =
                    self.validate_entity_value(value, &resolved_type, content, namespace, depth);
                diagnostics.extend(entity_diagnostics);
            }
            (CwtType::Block(_), _) => {
                // Expected a block but got something else
                let diagnostic = Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected a block/entity",
                    content,
                );
                diagnostics.push(diagnostic);
            }

            // Literal value validation
            (CwtType::Literal(literal_value), AstValue::String(string_value)) => {
                if string_value.raw_value() != literal_value {
                    let diagnostic = Self::create_value_mismatch_diagnostic(
                        value.span_range(),
                        &format!(
                            "Expected '{}' but got '{}'",
                            literal_value,
                            string_value.raw_value()
                        ),
                        content,
                    );
                    diagnostics.push(diagnostic);
                }
            }
            (CwtType::Literal(literal_value), _) => {
                // Expected a literal string but got something else
                let diagnostic = Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    &format!("Expected string literal '{}'", literal_value),
                    content,
                );
                diagnostics.push(diagnostic);
            }

            // Literal set validation
            (CwtType::LiteralSet(valid_values), AstValue::String(string_value)) => {
                if !valid_values.contains(string_value.raw_value()) {
                    let valid_list: Vec<_> = valid_values.iter().collect();
                    let diagnostic = Self::create_value_mismatch_diagnostic(
                        value.span_range(),
                        &format!(
                            "Expected one of {:?} but got '{}'",
                            valid_list,
                            string_value.raw_value()
                        ),
                        content,
                    );
                    diagnostics.push(diagnostic);
                }
            }
            (CwtType::LiteralSet(_), _) => {
                // Expected a string from literal set but got something else
                let diagnostic = Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected a string value",
                    content,
                );
                diagnostics.push(diagnostic);
            }

            // Simple type validation
            (CwtType::Simple(simple_type), _) => {
                if let Some(diagnostic) =
                    Self::is_value_compatible_with_simple_type(value, simple_type, content)
                {
                    diagnostics.push(diagnostic);
                }
            }

            // Array type validation
            (CwtType::Array(array_type), AstValue::Entity(entity)) => {
                // Arrays in CW are represented as entities with numbered keys
                // For now, we'll just validate that it's an entity - more complex validation would require
                // checking that all keys are valid indices and values match the element type
                let _element_type = &array_type.element_type;
                // TODO: Implement array element validation
            }
            (CwtType::Array(_), _) => {
                let diagnostic = Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Expected an array (entity with indexed elements)",
                    content,
                );
                diagnostics.push(diagnostic);
            }

            // Union type validation
            (CwtType::Union(types), _) => {
                // Check if the value is structurally compatible with any of the union members
                let mut compatible_type = None;

                for union_type in types {
                    if Self::is_value_structurally_compatible(value, union_type) {
                        compatible_type = Some(union_type.clone());
                        break;
                    }
                }

                if let Some(matching_type) = compatible_type {
                    // Value is structurally compatible with this union member,
                    // now validate the content according to this type
                    let content_diagnostics = self.validate_value_against_type(
                        value,
                        &matching_type,
                        content,
                        namespace,
                        depth + 1,
                    );
                    diagnostics.extend(content_diagnostics);
                } else {
                    // Value is not structurally compatible with any union member
                    let type_names: Vec<String> =
                        types.iter().map(|t| Self::get_type_name(t)).collect();

                    let diagnostic = Self::create_type_mismatch_diagnostic(
                        value.span_range(),
                        &format!(
                            "Value is not compatible with any of the expected types: {}",
                            type_names.join(", ")
                        ),
                        content,
                    );
                    diagnostics.push(diagnostic);
                }
            }

            // Comparable type validation
            (CwtType::Comparable(base_type), _) => {
                // For comparable types, validate against the base type
                let base_diagnostics = self.validate_value_against_type(
                    value,
                    base_type,
                    content,
                    namespace,
                    depth + 1,
                );
                diagnostics.extend(base_diagnostics);
            }

            // Reference type validation
            (CwtType::Reference(ref_type), _) => {
                // For reference types, we need to resolve them through the cache
                // For now, we'll skip validation of reference types as they require complex resolution
                eprintln!(
                    "DEBUG: Skipping validation of reference type {:?}",
                    ref_type
                );
            }

            // Unknown type - don't validate
            (CwtType::Unknown, _) => {
                // Don't validate unknown types
            }
        }

        diagnostics
    }

    /// Check if a value is compatible with a simple type, returning a diagnostic if incompatible
    fn is_value_compatible_with_simple_type(
        value: &AstValue<'_>,
        simple_type: &cw_model::SimpleType,
        content: &str,
    ) -> Option<Diagnostic> {
        use cw_model::SimpleType;

        match (value, simple_type) {
            (AstValue::String(_), SimpleType::Localisation) => {
                // TODO: Implement proper localisation validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Localisation validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::LocalisationSynced) => {
                // TODO: Implement proper localisation validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Localisation synced validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::LocalisationInline) => {
                // TODO: Implement proper localisation validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Inline localisation validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::Filepath) => {
                // TODO: Implement proper filepath validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Filepath validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::Icon) => {
                // TODO: Implement proper icon validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Icon validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::VariableField) => {
                // TODO: Implement proper variable field validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Variable field validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::ScopeField) => {
                // TODO: Implement proper scope field validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Scope field validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::DateField) => {
                // TODO: Implement proper date field validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Date field validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::Scalar) => None, // Valid
            (AstValue::String(_), SimpleType::IntVariableField) => {
                // TODO: Implement proper int variable field validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Int variable field validation not yet implemented",
                    content,
                ))
            }
            (AstValue::String(_), SimpleType::IntValueField) => {
                // TODO: Implement proper int value field validation
                Some(Self::create_type_mismatch_diagnostic(
                    value.span_range(),
                    "Int value field validation not yet implemented",
                    content,
                ))
            }

            (AstValue::Number(_), SimpleType::ValueField) => None, // Valid
            (AstValue::Number(n), SimpleType::Int) => {
                if n.value.value.find('.').is_none() {
                    None // Valid integer
                } else {
                    Some(Self::create_type_mismatch_diagnostic(
                        value.span_range(),
                        "Expected integer but got decimal number",
                        content,
                    ))
                }
            }
            (AstValue::Number(_), SimpleType::Float) => None, // Valid
            (AstValue::Number(n), SimpleType::PercentageField) => {
                if n.value.value.ends_with("%") {
                    None // Valid percentage
                } else {
                    Some(Self::create_type_mismatch_diagnostic(
                        value.span_range(),
                        "Expected percentage value (ending with %)",
                        content,
                    ))
                }
            }

            (AstValue::String(s), SimpleType::Bool) => {
                let val = s.raw_value();
                if val == "yes" || val == "no" {
                    None // Valid boolean
                } else {
                    Some(Self::create_type_mismatch_diagnostic(
                        value.span_range(),
                        "Expected boolean value ('yes' or 'no')",
                        content,
                    ))
                }
            }

            (AstValue::Color(_), SimpleType::Color) => None, // Valid
            (AstValue::Maths(_), SimpleType::Maths) => None, // Valid

            // Type mismatches
            (_, simple_type) => Some(Self::create_type_mismatch_diagnostic(
                value.span_range(),
                &format!(
                    "Expected {:?} but got {}",
                    simple_type,
                    Self::get_value_type_name(value)
                ),
                content,
            )),
        }
    }

    /// Get a human-readable name for a value type
    fn get_value_type_name(value: &AstValue<'_>) -> &'static str {
        match value {
            AstValue::String(_) => "string",
            AstValue::Number(_) => "number",
            AstValue::Entity(_) => "entity/block",
            AstValue::Color(_) => "color",
            AstValue::Maths(_) => "math expression",
        }
    }

    /// Get a human-readable name for a CWT type
    fn get_type_name(cwt_type: &CwtType) -> String {
        match cwt_type {
            CwtType::Simple(simple_type) => format!("{:?}", simple_type),
            CwtType::Block(_) => "block".to_string(),
            CwtType::Literal(value) => format!("'{}'", value),
            CwtType::LiteralSet(values) => {
                let value_list: Vec<_> = values.iter().take(3).collect();
                if values.len() > 3 {
                    format!("one of {:?}...", value_list)
                } else {
                    format!("one of {:?}", value_list)
                }
            }
            CwtType::Array(_) => "array".to_string(),
            CwtType::Union(_) => "union".to_string(),
            CwtType::Comparable(base_type) => {
                format!("comparable {}", Self::get_type_name(base_type))
            }
            CwtType::Reference(ref_type) => match ref_type {
                cw_model::ReferenceType::Type { key } => format!("<{}>", key),
                cw_model::ReferenceType::Enum { key } => format!("enum {}", key),
                cw_model::ReferenceType::ComplexEnum { key } => format!("complex_enum {}", key),
                cw_model::ReferenceType::ValueSet { key } => format!("value_set {}", key),
                cw_model::ReferenceType::Value { key } => format!("value {}", key),
                cw_model::ReferenceType::Scope { key } => format!("scope {}", key),
                cw_model::ReferenceType::ScopeGroup { key } => format!("scope_group {}", key),
                cw_model::ReferenceType::Alias { key } => format!("alias {}", key),
                cw_model::ReferenceType::AliasName { key } => format!("alias_name {}", key),
                cw_model::ReferenceType::AliasMatchLeft { key } => {
                    format!("alias_match_left {}", key)
                }
                cw_model::ReferenceType::SingleAlias { key } => format!("single_alias {}", key),
                cw_model::ReferenceType::AliasKeysField { key } => {
                    format!("alias_keys_field {}", key)
                }
                cw_model::ReferenceType::Colour { format } => format!("colour ({})", format),
                cw_model::ReferenceType::Icon { path } => format!("icon ({})", path),
                cw_model::ReferenceType::Filepath { path } => format!("filepath ({})", path),
                cw_model::ReferenceType::Subtype { name } => format!("subtype {}", name),
                cw_model::ReferenceType::StellarisNameFormat { key } => {
                    format!("name_format {}", key)
                }
                _ => format!("reference {:?}", ref_type),
            },
            CwtType::Unknown => "unknown".to_string(),
        }
    }

    /// Check if a value is structurally compatible with a type (without content validation)
    fn is_value_structurally_compatible(value: &AstValue<'_>, expected_type: &CwtType) -> bool {
        Self::is_value_structurally_compatible_with_depth(value, expected_type, 0)
    }

    /// Check if a value is structurally compatible with a type with recursion depth limit
    fn is_value_structurally_compatible_with_depth(
        value: &AstValue<'_>,
        expected_type: &CwtType,
        depth: usize,
    ) -> bool {
        // Prevent infinite recursion
        if depth > 10 {
            return false;
        }

        if !TypeCache::is_initialized() {
            return true; // Default to compatible if cache not available
        }

        let cache = TypeCache::get().unwrap();
        let resolved_type = cache.resolve_type(expected_type);

        match (&resolved_type, value) {
            // Block types are compatible with entities
            (CwtType::Block(_), AstValue::Entity(_)) => true,

            // Literal types are compatible with strings
            (CwtType::Literal(_), AstValue::String(_)) => true,

            // Literal sets are compatible with strings
            (CwtType::LiteralSet(_), AstValue::String(_)) => true,

            // Simple types - check basic compatibility
            (CwtType::Simple(simple_type), _) => {
                Self::is_value_compatible_with_simple_type_structurally(value, simple_type)
            }

            // Array types are compatible with entities
            (CwtType::Array(_), AstValue::Entity(_)) => true,

            // Union types - check if compatible with any member
            (CwtType::Union(types), _) => types.iter().any(|union_type| {
                Self::is_value_structurally_compatible_with_depth(value, union_type, depth + 1)
            }),

            // Comparable types - check compatibility with base type
            (CwtType::Comparable(base_type), _) => {
                Self::is_value_structurally_compatible_with_depth(value, base_type, depth + 1)
            }

            // Reference types - resolve and check
            (CwtType::Reference(_), _) => {
                // For now, assume references are compatible
                // TODO: Implement proper reference resolution
                true
            }

            // Unknown types are always compatible
            (CwtType::Unknown, _) => true,

            // Everything else is incompatible
            _ => false,
        }
    }

    /// Check if a value is structurally compatible with a simple type
    fn is_value_compatible_with_simple_type_structurally(
        value: &AstValue<'_>,
        simple_type: &cw_model::SimpleType,
    ) -> bool {
        use cw_model::SimpleType;

        match (value, simple_type) {
            // String-based types
            (AstValue::String(_), SimpleType::Localisation) => true,
            (AstValue::String(_), SimpleType::LocalisationSynced) => true,
            (AstValue::String(_), SimpleType::LocalisationInline) => true,
            (AstValue::String(_), SimpleType::Filepath) => true,
            (AstValue::String(_), SimpleType::Icon) => true,
            (AstValue::String(_), SimpleType::VariableField) => true,
            (AstValue::String(_), SimpleType::ScopeField) => true,
            (AstValue::String(_), SimpleType::DateField) => true,
            (AstValue::String(_), SimpleType::Scalar) => true,
            (AstValue::String(_), SimpleType::IntVariableField) => true,
            (AstValue::String(_), SimpleType::IntValueField) => true,
            (AstValue::String(_), SimpleType::Bool) => true,

            // Number-based types
            (AstValue::Number(_), SimpleType::ValueField) => true,
            (AstValue::Number(_), SimpleType::Int) => true,
            (AstValue::Number(_), SimpleType::Float) => true,
            (AstValue::Number(_), SimpleType::PercentageField) => true,

            // Specialized types
            (AstValue::Color(_), SimpleType::Color) => true,
            (AstValue::Maths(_), SimpleType::Maths) => true,

            // Everything else is incompatible
            _ => false,
        }
    }

    /// Create a diagnostic for type mismatches
    fn create_type_mismatch_diagnostic(
        span: Range<usize>,
        message: &str,
        content: &str,
    ) -> Diagnostic {
        let range = span_to_lsp_range(span, content);

        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("type-mismatch".to_string())),
            code_description: None,
            source: Some("cw-type-checker".to_string()),
            message: message.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Create a diagnostic for value mismatches
    fn create_value_mismatch_diagnostic(
        span: Range<usize>,
        message: &str,
        content: &str,
    ) -> Diagnostic {
        let range = span_to_lsp_range(span, content);

        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String("value-mismatch".to_string())),
            code_description: None,
            source: Some("cw-type-checker".to_string()),
            message: message.to_string(),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Get the type of a property from the expected type structure
    fn get_property_type_from_expected_type(
        expected_type: &CwtType,
        property_name: &str,
    ) -> CwtType {
        Self::get_property_type_from_expected_type_with_depth(expected_type, property_name, 0)
    }

    /// Get the type of a property from the expected type structure with recursion depth limit
    fn get_property_type_from_expected_type_with_depth(
        expected_type: &CwtType,
        property_name: &str,
        depth: usize,
    ) -> CwtType {
        // Prevent infinite recursion by limiting depth
        if depth > 10 {
            eprintln!(
                "DEBUG: Max recursion depth reached for property {}, returning Unknown",
                property_name
            );
            return CwtType::Unknown;
        }

        if !TypeCache::is_initialized() {
            return CwtType::Unknown;
        }

        let cache = TypeCache::get().unwrap();
        let resolved_type = cache.resolve_type(expected_type);

        match resolved_type {
            CwtType::Block(obj) => {
                if let Some(property_def) = obj.properties.get(property_name) {
                    // Return the resolved type of this property
                    cache.resolve_type(&property_def.property_type)
                } else {
                    // Check if the property matches any pattern property
                    if let Some(pattern_property) = cache
                        .get_resolver()
                        .key_matches_pattern(property_name, &obj)
                    {
                        cache.resolve_type(&pattern_property.value_type)
                    } else {
                        CwtType::Unknown
                    }
                }
            }
            CwtType::Union(types) => {
                // For union types, try to find the property in any of the union members
                for union_type in types {
                    let property_type = Self::get_property_type_from_expected_type_with_depth(
                        &union_type,
                        property_name,
                        depth + 1,
                    );
                    if !matches!(property_type, CwtType::Unknown) {
                        return property_type;
                    }
                }
                CwtType::Unknown
            }
            CwtType::Reference(_) => {
                // For references, resolve and try again
                let resolved_ref = cache.resolve_type(&resolved_type);
                if !matches!(resolved_ref, CwtType::Reference(_)) {
                    Self::get_property_type_from_expected_type_with_depth(
                        &resolved_ref,
                        property_name,
                        depth + 1,
                    )
                } else {
                    CwtType::Unknown
                }
            }
            _ => CwtType::Unknown,
        }
    }

    /// Check if a key is valid for the given type
    fn is_key_valid(cwt_type: &CwtType, key_name: &str) -> bool {
        Self::is_key_valid_with_depth(cwt_type, key_name, 0)
    }

    /// Check if a key is valid for the given type with recursion depth limit
    fn is_key_valid_with_depth(cwt_type: &CwtType, key_name: &str, depth: usize) -> bool {
        // Prevent infinite recursion by limiting depth
        if depth > 10 {
            return false;
        }

        if !TypeCache::is_initialized() {
            return true;
        }

        let cache = TypeCache::get().unwrap();
        let cwt_type = cache.resolve_type(cwt_type);

        let result = match &cwt_type {
            CwtType::Block(obj) => {
                // Check if the key is in the known properties
                if obj.properties.contains_key(key_name) {
                    return true;
                }

                // Check if the key matches any pattern property
                if cache
                    .get_resolver()
                    .key_matches_pattern(key_name, obj)
                    .is_some()
                {
                    return true;
                }

                false
            }
            CwtType::Union(types) => {
                // For union types, key is valid if it's valid in any of the union members
                types
                    .iter()
                    .any(|t| Self::is_key_valid_with_depth(t, key_name, depth + 1))
            }
            CwtType::Reference(_) => {
                // For references, try to resolve them but don't get stuck in infinite loops
                Self::is_key_valid_with_depth(&cwt_type, key_name, depth + 1)
            }
            CwtType::Unknown => false,
            CwtType::Simple(_) => false,
            CwtType::Array(_) => false,
            CwtType::Literal(_) => false,
            CwtType::LiteralSet(_) => false,
            CwtType::Comparable(_) => false,
        };

        result
    }

    /// Create a diagnostic for an unexpected key
    fn create_unexpected_key_diagnostic(
        span: Range<usize>,
        key_name: &str,
        namespace: &str,
        content: &str,
    ) -> Diagnostic {
        let range = span_to_lsp_range(span, content);

        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String("unexpected-key".to_string())),
            code_description: None,
            source: Some("cw-type-checker".to_string()),
            message: format!("Unexpected key '{}' in {} entity", key_name, namespace),
            related_information: None,
            tags: None,
            data: None,
        }
    }

    /// Create an LSP diagnostic from a parsing error
    fn create_diagnostic_from_parse_error(
        error: &cw_parser::CwParseError,
        _content: &str,
    ) -> Diagnostic {
        // Extract position information from the structured error
        let (range, message) = match error {
            cw_parser::CwParseError::Parse(parse_error) => {
                // Use the position information from the structured error
                let start_line = parse_error.line as u32;
                let start_character = parse_error.column as u32;

                // Calculate end position based on the span
                let span_length = (parse_error.span.end - parse_error.span.start) as u32;
                let end_character = start_character + span_length.max(1);

                let range = tower_lsp::lsp_types::Range {
                    start: Position {
                        line: start_line,
                        character: start_character,
                    },
                    end: Position {
                        line: start_line,
                        character: end_character,
                    },
                };

                (range, parse_error.message.clone())
            }
            cw_parser::CwParseError::Other(msg) => {
                // For non-parse errors, default to start of document
                let range = tower_lsp::lsp_types::Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 1,
                    },
                };

                (range, msg.clone())
            }
        };

        Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::ERROR),
            code: None,
            code_description: None,
            source: Some("cw-parser".to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
        }
    }
}

/// Generate diagnostics for a document (convenience function)
pub async fn generate_diagnostics(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    uri: &str,
) {
    let provider = DiagnosticsProvider::new(client, documents.clone());
    provider.generate_diagnostics(uri).await;
}

/// Convert a byte span to an LSP range
fn span_to_lsp_range(span: Range<usize>, content: &str) -> tower_lsp::lsp_types::Range {
    let start_position = offset_to_position(content, span.start);
    let end_position = offset_to_position(content, span.end);

    tower_lsp::lsp_types::Range {
        start: start_position,
        end: end_position,
    }
}

/// Convert byte offset to LSP position
fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut line = 0;
    let mut character = 0;

    for (i, ch) in content.char_indices() {
        if i >= offset {
            break;
        }

        if ch == '\n' {
            line += 1;
            character = 0;
        } else {
            character += 1;
        }
    }

    Position {
        line: line as u32,
        character: character as u32,
    }
}

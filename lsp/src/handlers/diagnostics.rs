use cw_model::CwtType;
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
                            // For nested entities, we need to get the actual type of this property
                            // before validating its contents
                            if let AstValue::Entity(_) = &expr.value {
                                // Get the type of this specific property
                                let property_type = Self::get_property_type_from_expected_type(
                                    expected_type,
                                    key_name,
                                );

                                let nested_diagnostics = self.validate_entity_value(
                                    &expr.value,
                                    &property_type,
                                    content,
                                    namespace,
                                    depth + 1,
                                );
                                diagnostics.extend(nested_diagnostics);
                            }
                        }
                    }
                }
            }
            _ => {
                eprintln!("DEBUG: Non-entity value at depth {}", depth);
                // For non-entity values (strings, numbers, etc.), no validation needed
            }
        }

        diagnostics
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
                    CwtType::Unknown
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
                obj.properties.contains_key(key_name)
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

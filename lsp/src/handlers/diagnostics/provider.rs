use cw_model::{CwtType, SimpleType, entity_from_module_ast};
use cw_parser::{AstEntityItem, AstModule, AstNode, AstValue};
use tower_lsp::lsp_types::*;

use crate::handlers::cache::{EntityRestructurer, GameDataCache, TypeCache};
use crate::handlers::diagnostics::diagnostic::{
    create_diagnostic_from_parse_error, create_unexpected_key_diagnostic,
};
use crate::handlers::diagnostics::type_validation::validate_entity_value;
use crate::handlers::scoped_type::{CwtTypeOrSpecialRef, PropertyNavigationResult, ScopedType};

use super::super::utils::extract_namespace_from_uri;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;

/// Provider for generating diagnostics with shared state
pub struct DiagnosticsProvider {
    documents: Arc<RwLock<HashMap<String, String>>>,
    log: bool,
}

impl DiagnosticsProvider {
    /// Create a new diagnostics provider
    pub fn new(documents: Arc<RwLock<HashMap<String, String>>>, log: bool) -> Self {
        Self { documents, log }
    }

    /// Generate diagnostics for a document by attempting to parse it and type-check it
    pub fn generate_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        let start_time = Instant::now();

        if self.log {
            eprintln!("🔍 Starting diagnostics generation for: {}", uri);
        }

        let documents_guard = self.documents.read().unwrap();
        if let Some(content) = documents_guard.get(uri) {
            let mut diagnostics = Vec::new();

            // First, try to parse the content
            let mut module = AstModule::new();
            match module.parse_input(content) {
                Ok(()) => {
                    if self.log {
                        eprintln!("✅ Parsing successful for: {}", uri);
                    }
                    // If parsing succeeds, do type checking
                    let type_diagnostics = self.generate_type_diagnostics(&module, uri, content);

                    diagnostics.extend(type_diagnostics);
                }
                Err(error) => {
                    if self.log {
                        eprintln!("❌ Parsing failed for: {} - {}", uri, error);
                    }
                    // If parsing fails, add parsing error
                    let diagnostic = create_diagnostic_from_parse_error(&error, content);
                    diagnostics.push(diagnostic);
                }
            }

            let elapsed = start_time.elapsed();
            if self.log {
                eprintln!(
                    "🏁 Finished diagnostics for: {} | {} diagnostics generated | took {:?}",
                    uri,
                    diagnostics.len(),
                    elapsed
                );
            }

            diagnostics
        } else {
            let elapsed = start_time.elapsed();
            if self.log {
                eprintln!("❓ Document not found: {} | took {:?}", uri, elapsed);
            }
            Vec::new()
        }
    }

    /// Generate type-checking diagnostics for a successfully parsed document
    fn generate_type_diagnostics(
        &self,
        module: &AstModule<'_>,
        uri: &str,
        content: &str,
    ) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Check if type cache is initialized
        if !TypeCache::is_initialized() {
            return diagnostics;
        }

        if !GameDataCache::is_initialized() {
            return diagnostics;
        }

        // Ensure EntityRestructurer is initialized for correct subtype narrowing
        if !EntityRestructurer::is_initialized() {
            return diagnostics;
        }

        let type_cache = TypeCache::get().unwrap();

        // Extract namespace from URI
        let namespace = match extract_namespace_from_uri(uri) {
            Some(ns) => ns,
            None => return diagnostics,
        };

        if namespace.starts_with("common/inline_scripts") {
            // These are special, they don't have a type
            return diagnostics;
        }

        // Get type information for this namespace
        let namespace_type = match type_cache.get_namespace_type(&namespace, Some(uri)) {
            Some(info) => info,
            None => return diagnostics,
        };

        if let CwtTypeOrSpecialRef::Unknown = namespace_type.cwt_type_for_matching() {
            panic!("Namespace type is unknown");
        }

        let type_def = type_cache
            .get_cwt_analyzer()
            .get_type(&namespace_type.get_type_name());

        // Check if we should treat the entire file as a single entity
        if let Some(type_def) = type_def {
            if type_def.options.type_per_file {
                let entity = entity_from_module_ast(module);

                // Perform subtype narrowing at the file level
                let validation_type = if let Some(type_cache) = TypeCache::get() {
                    let matching_subtypes = type_cache
                        .get_resolver()
                        .determine_matching_subtypes(namespace_type.clone(), &entity);

                    if !matching_subtypes.is_empty() {
                        Arc::new(namespace_type.with_subtypes(matching_subtypes))
                    } else {
                        namespace_type.clone()
                    }
                } else {
                    namespace_type.clone()
                };

                // Validate each top-level property in the module as if it were an entity property
                for item in &module.items {
                    if let AstEntityItem::Expression(expr) = item {
                        let key_name = expr.key.raw_value();

                        // Filter union types before property navigation
                        let filtered_validation_type = type_cache
                            .filter_union_types_by_properties(validation_type.clone(), &entity);

                        if let PropertyNavigationResult::Success(property_type) = type_cache
                            .get_resolver()
                            .navigate_to_property(filtered_validation_type.clone(), key_name)
                        {
                            // Validate the value against the property type
                            let value_diagnostics = validate_entity_value(
                                &expr.value,
                                property_type,
                                content,
                                &namespace,
                                1,
                            );
                            diagnostics.extend(value_diagnostics);
                        } else {
                            // Create diagnostic for unexpected property
                            let diagnostic = create_unexpected_key_diagnostic(
                                expr.key.span_range(),
                                key_name,
                                &namespace_type.type_name_for_display(),
                                content,
                            );
                            diagnostics.push(diagnostic);
                        }
                    }
                }

                return diagnostics;
            }
        }

        // Standard behavior: validate each entity in the module separately
        for item in &module.items {
            if let AstEntityItem::Expression(expr) = item {
                if expr.key.raw_value().starts_with("@") {
                    // we're setting a variable
                    let expected_type = Arc::new(CwtType::Union(vec![
                        Arc::new(CwtType::Simple(SimpleType::Int)),
                        Arc::new(CwtType::Simple(SimpleType::Float)),
                        Arc::new(CwtType::Simple(SimpleType::Scalar)),
                        Arc::new(CwtType::Simple(SimpleType::Bool)),
                    ]));

                    let entity_diagnostics = validate_entity_value(
                        &expr.value,
                        Arc::new(ScopedType::new_cwt(
                            expected_type,
                            namespace_type.scope_stack().clone(),
                            namespace_type.in_scripted_effect_block().cloned(),
                        )),
                        content,
                        &namespace,
                        0,
                    );

                    diagnostics.extend(entity_diagnostics);
                    continue;
                }

                // Top-level keys are entity names - they can be anything, so don't validate them
                // Instead, validate their VALUES against the namespace structure

                // Check if this entity needs restructuring for correct subtype narrowing
                if let AstValue::Entity(ast_entity) = &expr.value {
                    let container_key = expr.key.raw_value();

                    // Check if the container key matches skip_root_key for any union type BEFORE filtering
                    let mut is_skip_root_key_container = false;
                    if let CwtTypeOrSpecialRef::Union(union_types) =
                        namespace_type.cwt_type_for_matching()
                    {
                        for union_type in union_types {
                            let type_name = union_type.get_type_name();
                            if !type_name.is_empty() {
                                if let Some(type_def) =
                                    type_cache.get_cwt_analyzer().get_type(&type_name)
                                {
                                    if let Some(skip_root_key) = &type_def.skip_root_key {
                                        let should_skip = match skip_root_key {
                                            cw_model::SkipRootKey::Specific(skip_key) => {
                                                container_key.to_lowercase()
                                                    == skip_key.to_lowercase()
                                            }
                                            cw_model::SkipRootKey::Any => true,
                                            cw_model::SkipRootKey::Except(exceptions) => {
                                                !exceptions.iter().any(|exception| {
                                                    exception.to_lowercase()
                                                        == container_key.to_lowercase()
                                                })
                                            }
                                            cw_model::SkipRootKey::Multiple(keys) => {
                                                keys.iter().any(|k| {
                                                    k.to_lowercase() == container_key.to_lowercase()
                                                })
                                            }
                                        };

                                        if should_skip {
                                            is_skip_root_key_container = true;
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if is_skip_root_key_container {
                        // This is a skip_root_key container - validate each nested entity individually
                        for nested_item in &ast_entity.items {
                            if let AstEntityItem::Expression(nested_expr) = nested_item {
                                if let AstValue::Entity(nested_ast_entity) = &nested_expr.value {
                                    let nested_entity_key = nested_expr.key.raw_value();

                                    // For nested entities in skip_root_key containers, filter by the nested entity key
                                    let nested_filtered_namespace_type = type_cache
                                        .filter_union_types_by_key(
                                            namespace_type.clone(),
                                            nested_entity_key,
                                        );

                                    let (_effective_key, effective_entity) =
                                        EntityRestructurer::get_effective_entity_for_subtype_narrowing(
                                            &namespace,
                                            container_key,
                                            nested_entity_key,
                                            nested_ast_entity,
                                        );

                                    // Perform subtype narrowing with the effective entity data
                                    let nested_validation_type = if let Some(type_cache) =
                                        TypeCache::get()
                                    {
                                        let matching_subtypes =
                                            type_cache.get_resolver().determine_matching_subtypes(
                                                nested_filtered_namespace_type.clone(),
                                                &effective_entity,
                                            );

                                        if !matching_subtypes.is_empty() {
                                            Arc::new(
                                                nested_filtered_namespace_type
                                                    .with_subtypes(matching_subtypes),
                                            )
                                        } else {
                                            nested_filtered_namespace_type.clone()
                                        }
                                    } else {
                                        nested_filtered_namespace_type.clone()
                                    };

                                    // The type is already filtered, no need to filter again
                                    let filtered_nested_validation_type = nested_validation_type;

                                    // Validate the nested entity using the AST value for proper diagnostics
                                    let nested_entity_diagnostics = validate_entity_value(
                                        &nested_expr.value,
                                        filtered_nested_validation_type,
                                        content,
                                        &namespace,
                                        0,
                                    );

                                    diagnostics.extend(nested_entity_diagnostics);
                                }
                            }
                        }

                        // Skip the normal validation for the container since we handled the nested entities
                        continue;
                    }

                    let entity_key = container_key; // For normal entities, these are the same

                    // For normal entities, filter by the container key
                    let filtered_namespace_type = type_cache
                        .filter_union_types_by_key(namespace_type.clone(), &container_key);

                    let (_effective_key, effective_entity) =
                        EntityRestructurer::get_effective_entity_for_subtype_narrowing(
                            &namespace,
                            container_key,
                            entity_key,
                            ast_entity,
                        );

                    // Perform subtype narrowing with the effective entity data
                    let validation_type = if let Some(type_cache) = TypeCache::get() {
                        let matching_subtypes =
                            type_cache.get_resolver().determine_matching_subtypes(
                                filtered_namespace_type.clone(),
                                &effective_entity,
                            );

                        if !matching_subtypes.is_empty() {
                            Arc::new(filtered_namespace_type.with_subtypes(matching_subtypes))
                        } else {
                            filtered_namespace_type.clone()
                        }
                    } else {
                        filtered_namespace_type.clone()
                    };

                    // The type is already filtered, no need to filter again
                    let filtered_validation_type = validation_type;

                    let entity_diagnostics = validate_entity_value(
                        &expr.value,
                        filtered_validation_type,
                        content,
                        &namespace,
                        0,
                    );
                    diagnostics.extend(entity_diagnostics);
                } else {
                    // For non-entity values, use the namespace type without filtering
                    let entity_diagnostics = validate_entity_value(
                        &expr.value,
                        namespace_type.clone(),
                        content,
                        &namespace,
                        0,
                    );
                    diagnostics.extend(entity_diagnostics);
                };
            }
        }
        diagnostics
    }

    /// Generate diagnostics for content directly (synchronous version for parallel processing)
    pub fn generate_diagnostics_for_content(&self, uri: &str, content: &str) -> Vec<Diagnostic> {
        let start_time = Instant::now();

        if self.log {
            eprintln!("🔍 Starting diagnostics generation for content: {}", uri);
        }

        let mut diagnostics = Vec::new();

        // First, try to parse the content
        let mut module = AstModule::new();
        match module.parse_input(content) {
            Ok(()) => {
                if self.log {
                    eprintln!("✅ Parsing successful for content: {}", uri);
                }
                // If parsing succeeds, do type checking
                let type_diagnostics = self.generate_type_diagnostics(&module, uri, content);
                diagnostics.extend(type_diagnostics);
            }
            Err(error) => {
                if self.log {
                    eprintln!("❌ Parsing failed for content: {} - {}", uri, error);
                }
                // If parsing fails, add parsing error
                let diagnostic = create_diagnostic_from_parse_error(&error, content);
                diagnostics.push(diagnostic);
            }
        }

        let elapsed = start_time.elapsed();
        if self.log {
            eprintln!(
                "🏁 Finished diagnostics for content: {} | {} diagnostics generated | took {:?}",
                uri,
                diagnostics.len(),
                elapsed
            );
        }

        diagnostics
    }
}

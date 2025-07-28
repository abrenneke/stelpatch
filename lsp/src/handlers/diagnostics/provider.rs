use cw_model::entity_from_module_ast;
use cw_parser::{AstEntityItem, AstModule, AstNode, AstValue};

use crate::handlers::cache::TypeCache;
use crate::handlers::common_validation::{
    NamespaceValidationResult, apply_file_level_subtype_narrowing, create_variable_assignment_type,
    detect_skip_root_key_container, filter_and_narrow_entity_type, is_type_per_file_namespace,
    validate_namespace_and_caches,
};
use crate::handlers::diagnostics::diagnostic::{
    UnresolvedDiagnostic, create_diagnostic_from_parse_error, create_unexpected_key_diagnostic,
};
use crate::handlers::diagnostics::type_validation::validate_entity_value;
use crate::handlers::scoped_type::{CwtTypeOrSpecialRef, PropertyNavigationResult, ScopedType};
use crate::interner::get_interner;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use url::Url;

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
    pub fn generate_diagnostics(&self, uri: &str) -> Vec<tower_lsp::lsp_types::Diagnostic> {
        let start_time = Instant::now();

        if self.log {
            eprintln!("🔍 Starting diagnostics generation for: {}", uri);
        }

        let base_dir = Url::parse(uri)
            .ok()
            .and_then(|url| url.to_file_path().ok())
            .and_then(|path| crate::base_game::game::detect_base_directory(&path));

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
                    if let Some(ref base_dir) = base_dir {
                        let type_diagnostics =
                            self.generate_type_diagnostics(&module, uri, content, base_dir);

                        diagnostics.extend(type_diagnostics);
                    }
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

            diagnostics.into_iter().map(|d| d.into()).collect()
        } else {
            let elapsed = start_time.elapsed();
            if self.log {
                eprintln!("❓ Document not found: {} | took {:?}", uri, elapsed);
            }
            Vec::new()
        }
    }

    /// Generate type-checking diagnostics for a successfully parsed document
    fn generate_type_diagnostics<'a>(
        &self,
        module: &AstModule<'_>,
        uri: &str,
        content: &'a str,
        root_dir: &Path,
    ) -> Vec<UnresolvedDiagnostic<'a>> {
        let interner = get_interner();
        let mut diagnostics = Vec::new();

        // Validate namespace and caches using common validation
        let validation_context = match validate_namespace_and_caches(uri, root_dir) {
            NamespaceValidationResult::Valid(context) => context,
            other => {
                eprintln!("Namespace not found: {} - {:?}", uri, other);
                return diagnostics;
            }
        };

        let namespace = validation_context.namespace;
        let namespace_type = validation_context.namespace_type;
        let type_cache = TypeCache::get().unwrap();

        if let CwtTypeOrSpecialRef::Unknown = namespace_type.cwt_type_for_matching() {
            panic!("Namespace type is unknown");
        }

        // Check if we should treat the entire file as a single entity using common validation
        if is_type_per_file_namespace(&namespace_type) {
            let entity = entity_from_module_ast(module, get_interner());

            // Perform subtype narrowing at the file level using common function
            let validation_type =
                apply_file_level_subtype_narrowing(namespace_type.clone(), &entity);

            // Validate each top-level property in the module as if it were an entity property
            for item in &module.items {
                if let AstEntityItem::Expression(expr) = item {
                    let key_name = interner.get_or_intern(expr.key.raw_value());

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
                            namespace,
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

        // Standard behavior: validate each entity in the module separately
        for item in &module.items {
            if let AstEntityItem::Expression(expr) = item {
                if expr.key.raw_value().starts_with("@") {
                    // we're setting a variable - use common validation function
                    let variable_type = create_variable_assignment_type(&namespace_type);

                    let entity_diagnostics =
                        validate_entity_value(&expr.value, variable_type, content, namespace, 0);

                    diagnostics.extend(entity_diagnostics);
                    continue;
                }

                // Top-level keys are entity names - they can be anything, so don't validate them
                // Instead, validate their VALUES against the namespace structure

                // Check if this entity needs restructuring for correct subtype narrowing
                if let AstValue::Entity(ast_entity) = &expr.value {
                    let container_key = interner.get_or_intern(expr.key.raw_value());

                    // Check if the container key matches skip_root_key using common validation
                    let skip_root_key_result =
                        detect_skip_root_key_container(&namespace_type, container_key);
                    let is_skip_root_key_container =
                        skip_root_key_result.is_skip_root_key_container;

                    if is_skip_root_key_container {
                        // This is a skip_root_key container - validate each nested entity individually
                        for nested_item in &ast_entity.items {
                            if let AstEntityItem::Expression(nested_expr) = nested_item {
                                if let AstValue::Entity(nested_ast_entity) = &nested_expr.value {
                                    let nested_entity_key =
                                        interner.get_or_intern(nested_expr.key.raw_value());

                                    // For nested entities in skip_root_key containers, use the matching type
                                    let base_validation_type = if let Some(matching_type_name) =
                                        skip_root_key_result.matching_type_name
                                    {
                                        // Use only the specific type that matched the skip_root_key pattern
                                        let type_cache = TypeCache::get().unwrap();
                                        if let Some(type_def) = type_cache
                                            .get_cwt_analyzer()
                                            .get_type(matching_type_name)
                                        {
                                            Arc::new(ScopedType::new_cwt(
                                                type_def.rules.clone(),
                                                namespace_type.scope_stack().clone(),
                                                None,
                                            ))
                                        } else {
                                            namespace_type.clone()
                                        }
                                    } else {
                                        namespace_type.clone()
                                    };

                                    let filtered_nested_validation_type =
                                        filter_and_narrow_entity_type(
                                            base_validation_type,
                                            namespace,
                                            container_key,
                                            nested_entity_key,
                                            nested_ast_entity,
                                        );

                                    // Validate the nested entity using the AST value for proper diagnostics
                                    let nested_entity_diagnostics = validate_entity_value(
                                        &nested_expr.value,
                                        filtered_nested_validation_type,
                                        content,
                                        namespace,
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

                    // For normal entities, use common validation function
                    let filtered_validation_type = filter_and_narrow_entity_type(
                        namespace_type.clone(),
                        namespace,
                        container_key,
                        entity_key,
                        ast_entity,
                    );

                    let entity_diagnostics = validate_entity_value(
                        &expr.value,
                        filtered_validation_type,
                        content,
                        namespace,
                        0,
                    );
                    diagnostics.extend(entity_diagnostics);
                } else {
                    // For non-entity values, use the namespace type without filtering
                    let entity_diagnostics = validate_entity_value(
                        &expr.value,
                        namespace_type.clone(),
                        content,
                        namespace,
                        0,
                    );
                    diagnostics.extend(entity_diagnostics);
                };
            }
        }
        diagnostics
    }

    /// Generate diagnostics for content directly (synchronous version for parallel processing)
    pub fn generate_diagnostics_for_content<'a>(
        &self,
        uri: &str,
        content: &'a str,
        root_dir: &Path,
    ) -> Vec<UnresolvedDiagnostic<'a>> {
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
                let type_diagnostics =
                    self.generate_type_diagnostics(&module, uri, content, root_dir);
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

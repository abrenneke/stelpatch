use cw_model::{CwtType, SimpleType};
use cw_parser::{AstEntityItem, AstModule, AstValue};
use tower_lsp::lsp_types::*;

use crate::handlers::cache::{
    EntityRestructurer, GameDataCache, TypeCache, get_namespace_entity_type,
};
use crate::handlers::diagnostics::diagnostic::create_diagnostic_from_parse_error;
use crate::handlers::diagnostics::type_validation::validate_entity_value;
use crate::handlers::scoped_type::{CwtTypeOrSpecial, ScopedType};

use super::super::utils::extract_namespace_from_uri;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use tower_lsp::Client;

pub struct ClientDiagnosticsProvider<'client> {
    client: &'client Client,
    provider: DiagnosticsProvider,
}

impl<'client> ClientDiagnosticsProvider<'client> {
    pub fn new(client: &'client Client, provider: DiagnosticsProvider) -> Self {
        Self { client, provider }
    }

    pub async fn generate_diagnostics(&self, uri: &str) {
        let diagnostics = self.provider.generate_diagnostics(uri);

        // Publish diagnostics to the client
        self.client
            .publish_diagnostics(Url::parse(uri).unwrap(), diagnostics, None)
            .await;
    }
}

/// Provider for generating diagnostics with shared state
pub struct DiagnosticsProvider {
    documents: Arc<RwLock<HashMap<String, String>>>,
}

impl DiagnosticsProvider {
    /// Create a new diagnostics provider
    pub fn new(documents: Arc<RwLock<HashMap<String, String>>>) -> Self {
        Self { documents }
    }

    /// Generate diagnostics for a document by attempting to parse it and type-check it
    pub fn generate_diagnostics(&self, uri: &str) -> Vec<Diagnostic> {
        let start_time = Instant::now();
        eprintln!("DEBUG: Starting diagnostics generation for {}", uri);

        let documents_guard = self.documents.read().unwrap();
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
                    let type_diagnostics = self.generate_type_diagnostics(&module, uri, content);
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
                    let diagnostic = create_diagnostic_from_parse_error(&error, content);
                    diagnostics.push(diagnostic);
                }
            }

            let total_duration = start_time.elapsed();
            eprintln!(
                "DEBUG: Total diagnostics generation completed in {:?} for {}",
                total_duration, uri
            );

            diagnostics
        } else {
            eprintln!("DEBUG: No content found for {}", uri);
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
        let type_check_start = Instant::now();
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
            None => {
                return diagnostics;
            }
        };

        // Get type information for this namespace
        let namespace_type = match type_cache.get_namespace_type(&namespace, Some(uri)) {
            Some(info) => info,
            None => {
                return diagnostics;
            }
        };

        if let CwtTypeOrSpecial::CwtType(CwtType::Unknown) = namespace_type.cwt_type() {
            dbg!(&namespace);
            panic!("Namespace type is unknown");
        }

        // Validate each entity in the module
        let validation_start = Instant::now();
        for item in &module.items {
            if let AstEntityItem::Expression(expr) = item {
                if expr.key.raw_value().starts_with("@") {
                    // we're setting a variable
                    let expected_type = CwtType::Union(vec![
                        CwtType::Simple(SimpleType::Int),
                        CwtType::Simple(SimpleType::Float),
                        CwtType::Simple(SimpleType::Scalar),
                        CwtType::Simple(SimpleType::Bool),
                    ]);

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
                let validation_type = if let AstValue::Entity(ast_entity) = &expr.value {
                    // Get the effective entity for subtype narrowing while preserving AST for diagnostics
                    let container_key = expr.key.raw_value();
                    let entity_key = container_key; // For top-level entities, these are the same

                    let (effective_key, effective_entity) =
                        EntityRestructurer::get_effective_entity_for_subtype_narrowing(
                            &namespace,
                            container_key,
                            entity_key,
                            ast_entity,
                        );

                    // If the effective key is different, we need to perform subtype narrowing
                    if effective_key != entity_key {
                        // Extract property data from the effective entity for subtype narrowing
                        let mut property_data = HashMap::new();
                        for (key, property_list) in &effective_entity.properties.kv {
                            if let Some(first_property) = property_list.0.first() {
                                property_data.insert(key.clone(), first_property.value.to_string());
                            }
                        }

                        // Perform subtype narrowing with the effective entity data
                        if let Some(type_cache) = TypeCache::get() {
                            let matching_subtypes =
                                type_cache.get_resolver().determine_matching_subtypes(
                                    namespace_type.clone(),
                                    &property_data,
                                );

                            if !matching_subtypes.is_empty() {
                                Arc::new(namespace_type.with_subtypes(matching_subtypes))
                            } else {
                                namespace_type.clone()
                            }
                        } else {
                            namespace_type.clone()
                        }
                    } else {
                        namespace_type.clone()
                    }
                } else {
                    namespace_type.clone()
                };

                let entity_diagnostics =
                    validate_entity_value(&expr.value, validation_type, content, &namespace, 0);
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

    /// Generate diagnostics for content directly (synchronous version for parallel processing)
    pub fn generate_diagnostics_for_content(&self, uri: &str, content: &str) -> Vec<Diagnostic> {
        let start_time = Instant::now();
        eprintln!("DEBUG: Starting diagnostics generation for {}", uri);

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
                let type_diagnostics = self.generate_type_diagnostics(&module, uri, content);
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
                let diagnostic = create_diagnostic_from_parse_error(&error, content);
                diagnostics.push(diagnostic);
            }
        }

        let total_duration = start_time.elapsed();
        eprintln!(
            "DEBUG: Total diagnostics generation completed in {:?} for {}",
            total_duration, uri
        );

        diagnostics
    }
}

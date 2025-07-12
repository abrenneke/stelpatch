use cw_parser::{AstEntityItem, AstModule};
use tower_lsp::lsp_types::*;

use crate::handlers::cache::{GameDataCache, get_namespace_entity_type};
use crate::handlers::diagnostics::diagnostic::create_diagnostic_from_parse_error;
use crate::handlers::diagnostics::type_validation::validate_entity_value;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tower_lsp::Client;

use super::super::cache::TypeCache;
use super::super::utils::extract_namespace_from_uri;

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
                    let diagnostic = create_diagnostic_from_parse_error(&error, content);
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
            return diagnostics;
        }

        if !GameDataCache::is_initialized() {
            return diagnostics;
        }

        // Extract namespace from URI
        let namespace = match extract_namespace_from_uri(uri) {
            Some(ns) => ns,
            None => {
                return diagnostics;
            }
        };

        // Get type information for this namespace
        let type_info = match get_namespace_entity_type(&namespace) {
            Some(info) => info,
            None => {
                return diagnostics;
            }
        };

        let namespace_type = match &type_info.scoped_type {
            Some(t) => t.clone(),
            None => {
                return diagnostics;
            }
        };

        // Validate each entity in the module
        let validation_start = Instant::now();
        for item in &module.items {
            if let AstEntityItem::Expression(expr) = item {
                // Top-level keys are entity names - they can be anything, so don't validate them
                // Instead, validate their VALUES against the namespace structure
                let entity_diagnostics = validate_entity_value(
                    &expr.value,
                    namespace_type.clone(),
                    content,
                    &namespace,
                    0,
                );
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
}

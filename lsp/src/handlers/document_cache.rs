use crate::semantic_token_collector::{SemanticTokenCollector, generate_semantic_tokens};
use cw_parser::{AstModule, AstModuleCell, AstVisitor};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::lsp_types::*;

#[derive(Debug)]
pub struct CachedDocument {
    document: AstModuleCell,
    semantic_tokens: Vec<SemanticToken>,
    version: Option<i32>,
}

impl CachedDocument {
    /// Create a new cached document by parsing the content
    pub fn new(content: String, version: Option<i32>) -> Option<Self> {
        let document = AstModuleCell::from_input(content);

        let content_arc: Arc<str> = document.borrow_owner().as_str().into();

        let mut collector = SemanticTokenCollector::new(content_arc);

        if let Ok(ast) = document.borrow_dependent().as_ref() {
            collector.visit_module(ast);
        } else {
            return None;
        }

        let semantic_tokens = collector.build_tokens();

        Some(CachedDocument {
            document,
            semantic_tokens,
            version,
        })
    }

    pub fn borrow_ast<'a>(&'a self) -> Result<&'a AstModule<'a>, &'a cw_parser::CwParseError> {
        self.document.borrow_dependent().as_ref()
    }

    pub fn borrow_input(&self) -> &str {
        self.document.borrow_owner()
    }

    /// Check if this cache entry is valid for the given version
    pub fn is_valid_for_version(&self, version: Option<i32>) -> bool {
        self.version == version
    }
}

/// Document cache that stores parsed ASTs and derived information
pub struct DocumentCache {
    cache: RwLock<HashMap<String, Arc<CachedDocument>>>,
}

impl DocumentCache {
    pub fn new() -> Self {
        Self {
            cache: RwLock::new(HashMap::new()),
        }
    }

    pub async fn get(&self, uri: &str) -> Option<Arc<CachedDocument>> {
        let cache = self.cache.read().await;
        cache.get(uri).cloned()
    }

    /// Update or create a cached document
    pub async fn update_document(&self, uri: String, content: String, version: Option<i32>) {
        if let Some(cached_doc) = CachedDocument::new(content, version) {
            let mut cache = self.cache.write().await;
            cache.insert(uri, Arc::new(cached_doc));
        }
    }

    /// Remove a document from cache
    pub async fn remove_document(&self, uri: &str) {
        let mut cache = self.cache.write().await;
        cache.remove(uri);
    }

    /// Get semantic tokens for a document, using cache if available
    pub async fn get_semantic_tokens(
        &self,
        uri: &str,
        content: &str,
        version: Option<i32>,
    ) -> Vec<SemanticToken> {
        let cache = self.cache.read().await;
        let cached = cache.get(uri);

        if let Some(cached) = cached {
            return cached.semantic_tokens.clone();
        }

        // If not in cache, update cache and return tokens
        self.update_document(uri.to_string(), content.to_string(), version)
            .await;

        if let Some(cached) = cache.get(uri) {
            cached.semantic_tokens.clone()
        } else {
            // Fallback to direct generation if caching fails
            generate_semantic_tokens(content).await
        }
    }
}

impl Default for DocumentCache {
    fn default() -> Self {
        Self::new()
    }
}

use crate::semantic_token_collector::{SemanticTokenCollector, generate_semantic_tokens};
use cw_parser::{AstModule, AstModuleCell, AstVisitor};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tower_lsp::lsp_types::*;
use url::Url;

#[derive(Debug)]
pub struct CachedDocument {
    document: AstModuleCell,
    semantic_tokens: Vec<SemanticToken>,
    pub root_dir: PathBuf,

    #[allow(dead_code)]
    version: Option<i32>,
}

impl CachedDocument {
    /// Create a new cached document by parsing the content
    pub fn new(uri: &str, content: String, version: Option<i32>) -> Option<Self> {
        let document = AstModuleCell::from_input(content);

        let content_arc: Arc<str> = document.borrow_owner().as_str().into();

        let mut collector = SemanticTokenCollector::new(content_arc);

        if let Ok(ast) = document.borrow_dependent().as_ref() {
            collector.visit_module(ast);
        } else {
            return None;
        }

        let semantic_tokens = collector.build_tokens();

        let root_dir = match Url::parse(uri)
            .ok()
            .and_then(|url| url.to_file_path().ok())
            .and_then(|path| crate::base_game::game::detect_base_directory(&path))
        {
            Some(dir) => dir,
            None => return None,
        };

        Some(CachedDocument {
            document,
            semantic_tokens,
            version,
            root_dir,
        })
    }

    pub fn borrow_ast<'a>(&'a self) -> Result<&'a AstModule<'a>, &'a cw_parser::CwParseError> {
        self.document.borrow_dependent().as_ref()
    }

    pub fn borrow_input(&self) -> &str {
        self.document.borrow_owner()
    }

    /// Check if this cache entry is valid for the given version
    #[allow(dead_code)]
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

    pub fn get(&self, uri: &str) -> Option<Arc<CachedDocument>> {
        let cache = self.cache.read().expect("Failed to read cache");
        cache.get(uri).cloned()
    }

    /// Update or create a cached document
    pub fn update_document(&self, uri: String, content: String, version: Option<i32>) {
        if let Some(cached_doc) = CachedDocument::new(&uri, content, version) {
            let mut cache = self.cache.write().expect("Failed to write cache");
            cache.insert(uri, Arc::new(cached_doc));
        }
    }

    /// Remove a document from cache
    #[allow(dead_code)]
    pub fn remove_document(&self, uri: &str) {
        let mut cache = self.cache.write().expect("Failed to write cache");
        cache.remove(uri);
    }

    /// Get semantic tokens for a document, using cache if available
    pub fn get_semantic_tokens(
        &self,
        uri: &str,
        content: &str,
        version: Option<i32>,
        range: Option<Range>,
    ) -> Vec<SemanticToken> {
        let cache = self.cache.read().expect("Failed to read cache");
        let cached = cache.get(uri);

        let tokens = if let Some(cached) = cached {
            cached.semantic_tokens.clone()
        } else {
            // If not in cache, update cache and return tokens
            drop(cache); // Release read lock before acquiring write lock
            self.update_document(uri.to_string(), content.to_string(), version);

            let cache = self.cache.read().expect("Failed to read cache");
            if let Some(cached) = cache.get(uri) {
                cached.semantic_tokens.clone()
            } else {
                // Fallback to direct generation if caching fails
                generate_semantic_tokens(content)
            }
        };

        // Filter tokens by range if specified
        if let Some(range) = range {
            self.filter_tokens_by_range(tokens, range)
        } else {
            tokens
        }
    }

    /// Filter semantic tokens to only include those within the specified range
    fn filter_tokens_by_range(
        &self,
        tokens: Vec<SemanticToken>,
        range: Range,
    ) -> Vec<SemanticToken> {
        let start_line = range.start.line;
        let end_line = range.end.line;

        // Convert from relative positions back to absolute positions for filtering
        let mut filtered = Vec::new();
        let mut current_line = 0;
        let mut current_start = 0;

        for token in tokens {
            current_line += token.delta_line;
            if token.delta_line == 0 {
                current_start += token.delta_start;
            } else {
                current_start = token.delta_start;
            }

            let token_end = current_start + token.length;

            // Check if token falls within the requested range
            let include_token = if current_line < start_line || current_line > end_line {
                false
            } else if current_line == start_line && current_line == end_line {
                // Token is on both start and end line - check both boundaries
                token_end > range.start.character && current_start < range.end.character
            } else if current_line == start_line {
                // Token is on start line - check start boundary
                token_end > range.start.character
            } else if current_line == end_line {
                // Token is on end line - check end boundary
                current_start < range.end.character
            } else {
                // Token is between start and end lines - include it
                true
            };

            if include_token {
                filtered.push(token);
            }

            // Early exit if we've passed the end line
            if current_line > end_line {
                break;
            }
        }

        // Convert back to relative positions
        self.convert_to_relative_positions(filtered)
    }

    /// Convert absolute positioned tokens back to relative positions for LSP
    fn convert_to_relative_positions(&self, tokens: Vec<SemanticToken>) -> Vec<SemanticToken> {
        let mut result = Vec::new();
        let mut last_line = 0;
        let mut last_start = 0;

        for token in tokens {
            // Get absolute position from the token
            let abs_line = last_line + token.delta_line;
            let abs_start = if token.delta_line == 0 {
                last_start + token.delta_start
            } else {
                token.delta_start
            };

            // Calculate relative position
            let delta_line = abs_line - last_line;
            let delta_start = if delta_line == 0 {
                abs_start - last_start
            } else {
                abs_start
            };

            result.push(SemanticToken {
                delta_line,
                delta_start,
                length: token.length,
                token_type: token.token_type,
                token_modifiers_bitset: token.token_modifiers_bitset,
            });

            last_line = abs_line;
            last_start = abs_start;
        }

        result
    }
}

impl Default for DocumentCache {
    fn default() -> Self {
        Self::new()
    }
}

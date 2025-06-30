use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use super::document_cache::DocumentCache;

pub async fn semantic_tokens_full(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    document_cache: &DocumentCache,
    params: SemanticTokensParams,
) -> Result<Option<SemanticTokensResult>> {
    let uri = params.text_document.uri.to_string();

    client
        .log_message(
            MessageType::INFO,
            format!("Semantic tokens (full) requested for: {}", uri),
        )
        .await;

    let documents_guard = documents.read().await;
    if let Some(content) = documents_guard.get(&uri) {
        // Use document cache for efficient token generation
        let token_data = document_cache
            .get_semantic_tokens(&uri, content, None)
            .await;
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: token_data,
        })))
    } else {
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        })))
    }
}

pub async fn semantic_tokens_range(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    document_cache: &DocumentCache,
    params: SemanticTokensRangeParams,
) -> Result<Option<SemanticTokensRangeResult>> {
    let uri = params.text_document.uri.to_string();

    client
        .log_message(
            MessageType::INFO,
            format!(
                "Semantic tokens (range) requested for: {} at {:?}",
                uri, params.range
            ),
        )
        .await;

    let documents_guard = documents.read().await;
    if let Some(content) = documents_guard.get(&uri) {
        // Use document cache for efficient token generation
        // Note: For range requests, we're still generating full tokens
        // A more advanced implementation could optimize this
        let token_data = document_cache
            .get_semantic_tokens(&uri, content, None)
            .await;
        Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: token_data,
        })))
    } else {
        Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
            result_id: None,
            data: vec![],
        })))
    }
}

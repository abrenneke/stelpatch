use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::semantic_token_collector::generate_semantic_tokens;

pub async fn semantic_tokens_full(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
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
        let token_data = generate_semantic_tokens(content).await;
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
        let token_data = generate_semantic_tokens(content).await;
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

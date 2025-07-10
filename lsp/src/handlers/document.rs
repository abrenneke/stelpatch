use super::diagnostics;
use super::document_cache::DocumentCache;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::lsp_types::*;

pub async fn did_open(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    document_cache: &DocumentCache,
    params: DidOpenTextDocumentParams,
) {
    let uri = params.text_document.uri.to_string();
    let content = params.text_document.text.clone();
    let version = Some(params.text_document.version);

    // Store the document content
    documents.write().await.insert(uri.clone(), content.clone());

    // Update the document cache
    document_cache
        .update_document(uri.clone(), content, version)
        .await;

    client
        .log_message(MessageType::INFO, format!("Document opened: {}", uri))
        .await;

    // Generate diagnostics for the opened document
    diagnostics::generate_diagnostics(client, documents, &uri).await;
}

pub async fn did_change(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    document_cache: &DocumentCache,
    params: DidChangeTextDocumentParams,
) {
    let uri = params.text_document.uri.to_string();
    let version = Some(params.text_document.version);

    if let Some(change) = params.content_changes.into_iter().next() {
        let content = change.text.clone();

        // Update the stored document content
        documents.write().await.insert(uri.clone(), content.clone());

        // Update the document cache
        document_cache
            .update_document(uri.clone(), content, version)
            .await;

        client
            .log_message(MessageType::INFO, format!("Document changed: {}", uri))
            .await;

        // Generate diagnostics for the changed document
        diagnostics::generate_diagnostics(client, documents, &uri).await;
    }
}

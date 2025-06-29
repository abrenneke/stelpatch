use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::lsp_types::*;

pub async fn did_open(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    params: DidOpenTextDocumentParams,
) {
    let uri = params.text_document.uri.to_string();
    let content = params.text_document.text;

    // Store the document content
    documents.write().await.insert(uri.clone(), content);

    client
        .log_message(MessageType::INFO, format!("Document opened: {}", uri))
        .await;
}

pub async fn did_change(
    client: &Client,
    documents: &Arc<RwLock<HashMap<String, String>>>,
    params: DidChangeTextDocumentParams,
) {
    let uri = params.text_document.uri.to_string();

    if let Some(change) = params.content_changes.into_iter().next() {
        // Update the stored document content
        documents.write().await.insert(uri.clone(), change.text);

        client
            .log_message(MessageType::INFO, format!("Document changed: {}", uri))
            .await;
    }
}

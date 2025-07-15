use super::diagnostics;
use super::mod_detection;
use crate::CwLspServer;
use tower_lsp::lsp_types::*;

pub async fn did_open(server: &CwLspServer, params: DidOpenTextDocumentParams) {
    let uri = params.text_document.uri.to_string();
    let content = params.text_document.text.clone();
    let version = Some(params.text_document.version);

    // Store the document content
    server
        .documents
        .write()
        .await
        .insert(uri.clone(), content.clone());

    // Update the document cache
    server
        .document_cache
        .update_document(uri.clone(), content, version)
        .await;

    server
        .client
        .log_message(MessageType::INFO, format!("Document opened: {}", uri))
        .await;

    // Check if this is a mod file and load the mod if needed
    if let Ok(file_path) = uri.parse::<url::Url>() {
        if let Ok(file_path) = file_path.to_file_path() {
            // Get a mutable reference to the mod cache
            let mut mod_cache = server.mod_cache.write().await;

            match mod_detection::handle_mod_file_with_cache(
                &file_path,
                &server.client,
                &mut mod_cache,
            )
            .await
            {
                Ok(Some(game_mod)) => {
                    server
                        .client
                        .log_message(
                            MessageType::INFO,
                            format!("Successfully loaded mod: {}", game_mod.definition.name),
                        )
                        .await;

                    // Merge mod data into the game data cache
                    server.merge_mod_data(&game_mod).await;

                    server
                        .client
                        .log_message(
                            MessageType::INFO,
                            format!("Merged mod data into cache: {}", game_mod.definition.name),
                        )
                        .await;
                }
                Ok(None) => {
                    // Base game file or no descriptor.mod found
                }
                Err(e) => {
                    server
                        .client
                        .log_message(
                            MessageType::ERROR,
                            format!("Error handling mod file: {}", e),
                        )
                        .await;
                }
            }
        }
    }

    // Generate diagnostics for the opened document
    diagnostics::generate_diagnostics(&server.client, &server.documents, &uri).await;
}

pub async fn did_change(server: &CwLspServer, params: DidChangeTextDocumentParams) {
    let uri = params.text_document.uri.to_string();
    let version = Some(params.text_document.version);

    if let Some(change) = params.content_changes.into_iter().next() {
        let content = change.text.clone();

        // Update the stored document content
        server
            .documents
            .write()
            .await
            .insert(uri.clone(), content.clone());

        // Update the document cache
        server
            .document_cache
            .update_document(uri.clone(), content, version)
            .await;

        server
            .client
            .log_message(MessageType::INFO, format!("Document changed: {}", uri))
            .await;

        // Generate diagnostics for the changed document
        diagnostics::generate_diagnostics(&server.client, &server.documents, &uri).await;
    }
}

use std::collections::HashMap;
use std::sync::Arc;

use crate::handlers::cache::{GameDataCache, TypeCache};
use crate::handlers::diagnostics::DiagnosticsProvider;
use crate::semantic_token_collector::CwSemanticTokenType;
use tokio::sync::RwLock;
use tower_lsp::Client;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

pub async fn initialize(_params: InitializeParams) -> Result<InitializeResult> {
    Ok(InitializeResult {
        capabilities: ServerCapabilities {
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            completion_provider: Some(CompletionOptions {
                resolve_provider: Some(false),
                trigger_characters: Some(vec!["\"".to_string(), " ".to_string()]),
                all_commit_characters: None,
                work_done_progress_options: WorkDoneProgressOptions::default(),
                completion_item: None,
            }),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                    legend: SemanticTokensLegend {
                        token_types: CwSemanticTokenType::all_types(),
                        token_modifiers: vec![],
                    },
                    range: Some(true),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                }),
            ),
            document_formatting_provider: Some(OneOf::Left(true)),
            document_range_formatting_provider: Some(OneOf::Left(true)),
            ..Default::default()
        },
        server_info: Some(ServerInfo {
            name: "cw-lsp".to_string(),
            version: Some("0.1.0".to_string()),
        }),
    })
}

pub async fn initialized(
    client: &Client,
    documents: Arc<RwLock<HashMap<String, String>>>,
    _params: InitializedParams,
) {
    TypeCache::initialize_in_background();
    GameDataCache::initialize_in_background();

    let client_clone = client.clone();

    tokio::task::spawn(async move {
        while !TypeCache::is_initialized() || !GameDataCache::is_initialized() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        client_clone
            .log_message(
                MessageType::INFO,
                "Caches are ready, generating diagnostics",
            )
            .await;

        let provider = DiagnosticsProvider::new(&client_clone, documents.clone());

        let documents_guard = documents.read().await;
        for uri in documents_guard.keys() {
            provider.generate_diagnostics(uri).await;
        }
    });
}

pub async fn shutdown() -> Result<()> {
    Ok(())
}

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::handlers::cache::{
    EntityRestructurer, FileIndex, FullAnalysis, GameDataCache, TypeCache,
};
use crate::handlers::diagnostics::generate_diagnostics;
use crate::handlers::utils::log_message_sync;
use crate::semantic_token_collector::CwSemanticTokenType;
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
    FileIndex::initialize_in_background();

    let client_clone = client.clone();

    let documents = documents.clone();

    std::thread::spawn(move || {
        while !TypeCache::is_initialized()
            || !GameDataCache::is_initialized()
            || !FileIndex::is_initialized()
        {
            std::thread::sleep(std::time::Duration::from_millis(100));
        }

        log_message_sync(
            &client_clone,
            MessageType::INFO,
            "Caches are ready, restructuring entities".to_string(),
        );

        let entity_restructurer =
            EntityRestructurer::new(GameDataCache::get().unwrap(), TypeCache::get().unwrap());
        entity_restructurer.load();

        log_message_sync(
            &client_clone,
            MessageType::INFO,
            "Entity restructuring complete, loading full analysis".to_string(),
        );

        let full_analysis = FullAnalysis::new(TypeCache::get().unwrap());
        full_analysis.load();

        let documents_guard = documents.read().unwrap();
        for uri in documents_guard.keys() {
            let client_clone = client_clone.clone();
            let documents = documents.clone();
            let uri = uri.clone();

            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(async move {
                    generate_diagnostics(&client_clone, &documents, &uri).await;
                });
        }
    });
}

pub async fn shutdown() -> Result<()> {
    Ok(())
}

use crate::handlers::cache::{GameDataCache, TypeCache};
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

pub async fn initialized(client: &Client, _params: InitializedParams) {
    client
        .log_message(MessageType::INFO, "CW LSP Server initialized!")
        .await;

    // Initialize caches in the background
    TypeCache::initialize_in_background();
    GameDataCache::initialize_in_background();

    client
        .log_message(MessageType::INFO, "Starting cache initialization...")
        .await;
}

pub async fn shutdown() -> Result<()> {
    Ok(())
}

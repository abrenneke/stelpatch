use tower_lsp::LanguageServer;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::CwLspServer;

mod document;
pub mod document_cache;
mod hover;
mod semantic_tokens;
mod server_lifecycle;
pub mod utils;

#[tower_lsp::async_trait]
impl LanguageServer for CwLspServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        server_lifecycle::initialize(params).await
    }

    async fn initialized(&self, params: InitializedParams) {
        server_lifecycle::initialized(&self.client, params).await;
    }

    async fn shutdown(&self) -> Result<()> {
        server_lifecycle::shutdown().await
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        document::did_open(&self.client, &self.documents, &self.document_cache, params).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        document::did_change(&self.client, &self.documents, &self.document_cache, params).await;
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        semantic_tokens::semantic_tokens_full(
            &self.client,
            &self.documents,
            &self.document_cache,
            params,
        )
        .await
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        semantic_tokens::semantic_tokens_range(
            &self.client,
            &self.documents,
            &self.document_cache,
            params,
        )
        .await
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        hover::hover(&self.client, &self.documents, &self.document_cache, params).await
    }
}

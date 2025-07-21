use tower_lsp::LanguageServer;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;

use crate::CwLspServer;

pub mod cache;
pub mod common_validation;
pub mod diagnostics;
mod document;
pub mod document_cache;
mod formatting;
mod hover;
pub mod mod_detection;
mod modifiers;
mod scope;
mod scoped_type;
mod semantic_tokens;
mod server_lifecycle;
mod settings;
pub mod utils;

#[tower_lsp::async_trait]
impl LanguageServer for CwLspServer {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        server_lifecycle::initialize(params).await
    }

    async fn initialized(&self, params: InitializedParams) {
        server_lifecycle::initialized(&self.client, self.documents.clone(), params).await;
    }

    async fn shutdown(&self) -> Result<()> {
        server_lifecycle::shutdown().await
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        document::did_open(self, params);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        document::did_change(self, params);
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
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        hover::hover(&self.client, &self.documents, &self.document_cache, params)
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        formatting::document_formatting(&self.client, &self.documents, &self.document_cache, params)
    }

    async fn range_formatting(
        &self,
        params: DocumentRangeFormattingParams,
    ) -> Result<Option<Vec<TextEdit>>> {
        formatting::document_range_formatting(
            &self.client,
            &self.documents,
            &self.document_cache,
            params,
        )
    }
}

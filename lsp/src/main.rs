use tower_lsp::{LspService, Server};

use cw_lsp::CwLspServer;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| CwLspServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}

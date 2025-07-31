use cw_lsp::{CwLspServer, handlers::settings::Settings};
use tower_lsp::{LspService, Server};

#[tokio::main]
async fn main() {
    // Initialize global settings from command line arguments
    // This will parse --game argument to determine which game to support
    Settings::init_global_from_args();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| CwLspServer::new(client));
    Server::new(stdin, stdout, socket).serve(service).await;
}

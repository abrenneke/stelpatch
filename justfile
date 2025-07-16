set shell := ["bash", "-c"]

bench:
  cd cw_games && cargo bench

profile:
  #!/bin/bash
  EXEC_PATH=$(cd cw_games && cargo bench --no-run --message-format=json-render-diagnostics \
    | jq -js '[.[] | select(.reason=="compiler-artifact") | select(.executable != null) | select(.target.kind | map(.=="bench") | any)] | last | .executable')
  samply record "$EXEC_PATH" --bench --profile-time 10

load-stellaris:
  cd cw_games && cargo run --release --bin load_stellaris

# Package the VS Code extension into a VSIX file
package-extension:
  #!/bin/bash
  set -e
  echo "📦 Packaging VS Code extension..."
  
  # Clear any existing VSIX files
  echo "🧹 Clearing existing VSIX files..."
  rm -f vscode-extension/*.vsix
  
  # Stop any running LSP server processes to free up the executable
  echo "🛑 Stopping any running LSP server processes..."
  pkill -f "cw_lsp" || true
  sleep 1
  
  echo "🦀 Building LSP server..."
  cd lsp
  cargo build --release
  cd ..
  
  echo "📂 Copying LSP server to extension..."
  mkdir -p "vscode-extension/server"
  if [[ "$OSTYPE" == "cygwin" || "$OSTYPE" == "msys" || "$OS" == "Windows_NT" ]]; then
    cp "target/release/cw_lsp.exe" "vscode-extension/server/"
  else
    cp "target/release/cw_lsp" "vscode-extension/server/"
  fi
  
  echo "📋 Copying CWT configuration files..."
  mkdir -p "vscode-extension/config"
  cp -r "D:\dev\github\cwtools-stellaris-config/config/"* "vscode-extension/config/"
  
  cd vscode-extension
  echo "🔧 Installing dependencies..."
  npm install
  
  echo "🏗️ Compiling TypeScript..."
  npm run compile
  
  echo "📦 Creating VSIX package..."
  npm run package
  
  echo "✅ Extension packaged successfully!"
  ls -1 *.vsix | while read -r file; do
    echo "📄 Created: $file"
  done
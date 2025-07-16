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

# Update version in both package.json and Cargo.toml
# Usage: just upversion --patch | --minor | --major
upversion *ARGS:
  #!/bin/bash
  set -e
  
  # Parse arguments
  BUMP_TYPE=""
  for arg in "$@"; do
    case $arg in
      --major)
        BUMP_TYPE="major"
        ;;
      --minor) 
        BUMP_TYPE="minor"
        ;;
      --patch)
        BUMP_TYPE="patch"
        ;;
      *)
        echo "❌ Unknown argument: $arg"
        echo "Usage: just upversion [--major|--minor|--patch]"
        exit 1
        ;;
    esac
  done
  
  if [ -z "$BUMP_TYPE" ]; then
    echo "❌ Please specify version bump type: --major, --minor, or --patch"
    echo "Usage: just upversion [--major|--minor|--patch]"
    exit 1
  fi
  
  # Get current version from LSP Cargo.toml
  CURRENT_VERSION=$(grep '^version = ' lsp/Cargo.toml | sed 's/version = "\(.*\)"/\1/')
  echo "📋 Current version: $CURRENT_VERSION"
  
  # Parse version components
  IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
  
  # Calculate new version
  case $BUMP_TYPE in
    major)
      NEW_VERSION="$((MAJOR + 1)).0.0"
      ;;
    minor)
      NEW_VERSION="$MAJOR.$((MINOR + 1)).0"
      ;;
    patch)
      NEW_VERSION="$MAJOR.$MINOR.$((PATCH + 1))"
      ;;
  esac
  
  echo "🚀 Bumping $BUMP_TYPE version: $CURRENT_VERSION → $NEW_VERSION"
  
  # Update LSP Cargo.toml
  echo "📝 Updating lsp/Cargo.toml..."
  sed -i.bak "s/^version = \".*\"/version = \"$NEW_VERSION\"/" lsp/Cargo.toml
  
  # Update VS Code extension package.json
  echo "📝 Updating vscode-extension/package.json..."
  if command -v jq >/dev/null 2>&1; then
    # Use jq if available (more reliable)
    jq ".version = \"$NEW_VERSION\"" vscode-extension/package.json > vscode-extension/package.json.tmp
    mv vscode-extension/package.json.tmp vscode-extension/package.json
  else
    # Fallback to sed
    sed -i.bak "s/\"version\": \".*\"/\"version\": \"$NEW_VERSION\"/" vscode-extension/package.json
  fi
  
  # Clean up backup files
  rm -f lsp/Cargo.toml.bak vscode-extension/package.json.bak 2>/dev/null || true
  
  echo "✅ Successfully updated both versions to $NEW_VERSION"
  echo "📋 Updated files:"
  echo "   - lsp/Cargo.toml"
  echo "   - vscode-extension/package.json"

# Package the VS Code extension into a VSIX file
package-extension:
  #!/bin/bash
  set -e
  echo "📦 Packaging VS Code extension..."
  
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
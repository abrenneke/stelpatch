# CW LSP VSCode Extension

A VSCode extension for testing the CW Language Server Protocol implementation.

## Features

- Syntax highlighting for Stellaris mod files
- Language server integration with basic functionality
- Support for `.txt`, `.mod`, `.gui`, and `.gfx` files

## Setup

1. **Install dependencies:**
   ```bash
   cd vscode-extension
   npm install
   ```

2. **Compile the extension:**
   ```bash
   npm run compile
   ```

3. **Open in VSCode:**
   - Press `F5` to open a new Extension Development Host window
   - Or use "Run and Debug" in VSCode

## Usage

1. Open any `.txt`, `.mod`, `.gui`, or `.gfx` file
2. The language server will automatically start and connect
3. Check the "Output" panel → "CW Language Server" for logs

## Development

- `npm run compile` - Compile TypeScript to JavaScript
- `npm run watch` - Watch for changes and auto-compile

## Testing

Create a test file with Stellaris syntax:

```stellaris
# Test building
test_building = {
    name = "Test Building"
    cost = {
        minerals = 100
        energy = 50
    }
    
    category = manufacturing
    
    potential = {
        always = yes
    }
}
```

The LSP server should provide basic functionality and log messages to the Output panel. 
# CW LSP - Stellaris Mod Language Support

A Visual Studio Code extension providing language server support for Stellaris game files with syntax highlighting, diagnostics, and IntelliSense.

## Features

✨ **Language Support**
- Syntax highlighting for Stellaris mod files (`.txt`, `.mod`, `.gui`, `.gfx`)
- Automatic language detection for files in `common/` directories
- IntelliSense and code completion
- Real-time diagnostics and error checking

🚀 **Developer Experience**
- Language server integration with fast, incremental parsing
- Automatic server restart command
- Cross-platform support (Windows, macOS, Linux)
- Bundled LSP server - no external dependencies required

📁 **File Support**
- **`.txt`** - Game data files in `common/` directories
- **`.mod`** - Mod descriptor files
- **`.gui`** - User interface definition files
- **`.gfx`** - Graphics definition files

## Installation

### From VSIX Package

1. Download the latest `.vsix` file from the [releases page](https://github.com/abrenneke/stelpatch/releases)
2. Open VS Code
3. Press `Ctrl+Shift+P` (or `Cmd+Shift+P` on macOS) to open the command palette
4. Type `Extensions: Install from VSIX...`
5. Select the downloaded `.vsix` file

### From Command Line

```bash
code --install-extension cw-lsp-client-0.1.0.vsix
```

## Usage

### Automatic Language Detection

The extension automatically detects Stellaris files:
- Any `.txt` file in a `common/` directory
- `.mod`, `.gui`, and `.gfx` files anywhere

### Manual Language Selection

If automatic detection doesn't work:
1. Open a file
2. Click the language indicator in the status bar (bottom right)
3. Select "Stellaris" from the list

### Commands

- **Restart CW Language Server**: `Ctrl+Shift+P` → `Restart CW Language Server`
- Use this if the language server becomes unresponsive

### Viewing Logs

Check the Output panel for detailed logs:
1. Go to View → Output
2. Select "CW LSP Extension" from the dropdown

## Supported File Types

### Game Data Files (`.txt`)
```stellaris
# Buildings
building_example = {
    category = manufacturing
    cost = {
        minerals = 100
        energy = 50
    }
    
    potential = {
        always = yes
    }
}
```

### Mod Files (`.mod`)
```stellaris
name = "My Stellaris Mod"
version = "1.0"
supported_version = "3.6.*"
```

### GUI Files (`.gui`)
```stellaris
guiTypes = {
    containerWindowType = {
        name = "example_window"
        size = { width = 400 height = 300 }
    }
}
```

## Requirements

- Visual Studio Code 1.75.0 or higher
- No additional dependencies required (LSP server is bundled)

## Troubleshooting

### Extension Not Working

1. Check if the file is in a `common/` directory or has the right extension
2. Manually set the language to "Stellaris"
3. Restart the language server with `Ctrl+Shift+P` → `Restart CW Language Server`
4. Check the Output panel for error messages

### Performance Issues

- The language server compiles on first startup, which may take a moment
- Subsequent starts are faster as the server is cached
- Large mod files may take longer to parse

## Development

### Building from Source

```bash
# Clone the repository
git clone https://github.com/abrenneke/stelpatch.git
cd stelpatch

# Build the extension
just package-extension
```

### Development Mode

```bash
cd vscode-extension
npm install
npm run compile
```

Then press `F5` in VS Code to open the Extension Development Host.

## Contributing

Contributions are welcome! Please see the [main repository](https://github.com/abrenneke/stelpatch) for contribution guidelines.

## License

MIT License - see the [LICENSE](LICENSE) file for details.

## Links

- [GitHub Repository](https://github.com/abrenneke/stelpatch)
- [Issues & Bug Reports](https://github.com/abrenneke/stelpatch/issues)
- [Releases](https://github.com/abrenneke/stelpatch/releases) 
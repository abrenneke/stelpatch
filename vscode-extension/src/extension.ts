import * as path from 'path';
import * as fs from 'fs';
import { workspace, ExtensionContext, window, OutputChannel, commands, TextDocument, languages } from 'vscode';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind,
	State
} from 'vscode-languageclient/node';

// Debug: Check if extension module is loaded at all
console.log('🔍 CW LSP Extension: Module loaded!');

let client: LanguageClient;
let outputChannel: OutputChannel;

/**
 * Check if a file path contains a 'common' directory, indicating it's a Stellaris config file
 */
function isStellarisFile(filePath: string): boolean {
	const normalizedPath = path.normalize(filePath).replace(/\\/g, '/');
	return normalizedPath.includes('/common/') && filePath.endsWith('.txt');
}

/**
 * Automatically set the language for .txt files in common directories
 */
function setLanguageForDocument(document: TextDocument) {
	if (document.languageId === 'plaintext' && isStellarisFile(document.uri.fsPath)) {
		// TODO causes recursion error or infinite loop or something :(
		// log(`🎯 Auto-detecting Stellaris file: ${document.uri.fsPath}`);
		// languages.setTextDocumentLanguage(document, 'stellaris');
	}
}

export function activate(context: ExtensionContext) {
	// Debug: Check if activate function is called
	console.log('🚀 CW LSP Extension: ACTIVATE FUNCTION CALLED!');
	
	// Create output channel for logging
	outputChannel = window.createOutputChannel('CW LSP Extension');
	outputChannel.show(true);
	
	log('🚀 CW LSP Extension activating...');
	log(`Extension path: ${context.extensionPath}`);
	// Set up automatic language detection for currently open documents
	// Seems buggy, so disabled for now
	// if (workspace.textDocuments) {
	// 	for (const document of workspace.textDocuments) {
	// 		setLanguageForDocument(document);
	// 	}
	// }

	// Listen for when documents are opened
	const onDidOpenTextDocument = workspace.onDidOpenTextDocument((document) => {
		setLanguageForDocument(document);
	});

	// Listen for when documents are saved (in case they were renamed)
	const onDidSaveTextDocument = workspace.onDidSaveTextDocument((document) => {
		setLanguageForDocument(document);
	});

	// Add event listeners to context subscriptions
	context.subscriptions.push(onDidOpenTextDocument);
	context.subscriptions.push(onDidSaveTextDocument);
	
	// Try bundled executable first, fall back to cargo for development
	const executableName = process.platform === 'win32' ? 'cw_lsp.exe' : 'cw_lsp';
	const serverExecutable = path.join(context.extensionPath, 'server', executableName);
	log(`Checking for bundled executable: ${serverExecutable}`);
	
	let serverOptions: ServerOptions;
	
	if (fs.existsSync(serverExecutable)) {
		// Production mode: use bundled executable
		log('✅ Using bundled LSP server executable');
		serverOptions = {
			run: { 
				command: serverExecutable
			},
			debug: {
				command: serverExecutable
			}
		};
		log('📋 Server options configured (bundled):');
		log(`  Command: ${serverExecutable}`);
	} else {
		// Development mode: compile with cargo
		log('🔄 Bundled executable not found, falling back to cargo (development mode)');
		
		const serverCommand = 'cargo';
		const serverArgs = ['run', '--release', '--bin', 'cw_lsp'];
		const serverWorkingDirectory = path.join(context.extensionPath, '..', 'lsp');
		
		log(`Server working directory: ${serverWorkingDirectory}`);
		
		// Check if the LSP server directory exists
		if (!fs.existsSync(serverWorkingDirectory)) {
			log(`❌ ERROR: LSP server directory does not exist: ${serverWorkingDirectory}`);
			window.showErrorMessage(`CW LSP: Server directory not found at ${serverWorkingDirectory}`);
			return;
		}
		
		// Check if Cargo.toml exists in the server directory
		const cargoTomlPath = path.join(serverWorkingDirectory, 'Cargo.toml');
		if (!fs.existsSync(cargoTomlPath)) {
			log(`❌ ERROR: Cargo.toml not found at: ${cargoTomlPath}`);
			window.showErrorMessage(`CW LSP: Cargo.toml not found in server directory`);
			return;
		}
		
		log('✅ Server directory and Cargo.toml found');
		
		serverOptions = {
			run: { 
				command: serverCommand, 
				args: serverArgs,
				options: {
					cwd: serverWorkingDirectory
				}
			},
			debug: {
				command: serverCommand,
				args: serverArgs,
				options: {
					cwd: serverWorkingDirectory
				}
			}
		};
		
		log('📋 Server options configured (cargo):');
		log(`  Command: ${serverCommand}`);
		log(`  Args: ${JSON.stringify(serverArgs)}`);
		log(`  Working directory: ${serverWorkingDirectory}`);
	}

	// Options to control the language client
	const clientOptions: LanguageClientOptions = {
		// Register the server for Stellaris documents
		documentSelector: [
			{ scheme: 'file', language: 'stellaris' },
			{ scheme: 'file', language: 'plaintext', pattern: '**/common/**/*.txt' }
		],
		synchronize: {
			// Notify the server about file changes to files contained in the workspace
			fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
		},
		outputChannel: outputChannel,
		revealOutputChannelOn: 4 // Show on error
	};

	log('📋 Client options configured');
	log(`  Document selector: ${JSON.stringify(clientOptions.documentSelector)}`);

	// Create the language client and start the client.
	client = new LanguageClient(
		'cwLanguageServer',
		'CW Language Server',
		serverOptions,
		clientOptions
	);

	log('🔧 Language client created');

	// Add event listeners for debugging
	client.onDidChangeState((event) => {
		log(`🔄 Client state changed: ${State[event.oldState]} -> ${State[event.newState]}`);
		
		if (event.newState === State.Running) {
			log('✅ LSP server is now running!');
		} else if (event.newState === State.Stopped) {
			log('🛑 LSP server stopped');
			if (event.oldState === State.Starting) {
				log('❌ Server failed to start');
				window.showErrorMessage('CW LSP: Server failed to start. Check the output for details.');
			}
		}
	});

	// Note: We'll rely on state changes to detect when the server is ready

	// Start the client. This will also launch the server
	log('🚀 Starting language client...');
	try {
		client.start();
		log('✅ Client start() called successfully');
	} catch (error) {
		log(`❌ Error starting client: ${error}`);
		window.showErrorMessage(`CW LSP: Error starting client: ${error}`);
	}
	
	// Register restart command
	const restartCommand = commands.registerCommand('cwlsp.restartServer', async () => {
		log('🔄 Manual server restart requested');
		await restartServer();
	});

	// Add the client and command to the context so they can be disposed
	context.subscriptions.push(client);
	context.subscriptions.push(restartCommand);
	log('📝 Extension activation completed');
}

async function restartServer() {
	if (client) {
		log('🛑 Stopping current server...');
		try {
			await client.stop();
			log('✅ Server stopped successfully');
		} catch (error) {
			log(`❌ Error stopping server: ${error}`);
		}
		
		log('🚀 Starting server again...');
		try {
			await client.start();
			log('✅ Server restarted successfully');
			window.showInformationMessage('CW LSP: Server restarted successfully');
		} catch (error) {
			log(`❌ Error restarting server: ${error}`);
			window.showErrorMessage(`CW LSP: Error restarting server: ${error}`);
		}
	} else {
		log('❌ No client to restart');
		window.showWarningMessage('CW LSP: No server to restart');
	}
}

function log(message: string) {
	const timestamp = new Date().toISOString();
	const logMessage = `[${timestamp}] ${message}`;
	
	if (outputChannel) {
		outputChannel.appendLine(logMessage);
	}
	
	// Also log to console for development
	console.log(`[CW LSP Extension] ${logMessage}`);
}

export function deactivate(): Thenable<void> | undefined {
	console.log('🛑 CW LSP Extension: DEACTIVATE FUNCTION CALLED!');
	log('🛑 Extension deactivating...');
	
	if (!client) {
		return undefined;
	}
	return client.stop();
} 
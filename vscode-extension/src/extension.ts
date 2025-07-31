import * as path from 'path';
import * as fs from 'fs';
import { workspace, ExtensionContext, window, OutputChannel, commands, TextDocument, languages, WorkspaceConfiguration, StatusBarItem, StatusBarAlignment } from 'vscode';

import {
	LanguageClient,
	LanguageClientOptions,
	ServerOptions,
	TransportKind,
	State
} from 'vscode-languageclient/node';

// Debug: Check if extension module is loaded at all
console.log('🔍 CW LSP Extension: Module loaded!');

// Game type definitions
enum GameType {
	Stellaris = 'stellaris',
	Victoria3 = 'victoria3'
}

interface GameConfig {
	languageId: string;
	displayName: string;
	filePatterns: string[];
}

// Game configurations
const GAME_CONFIGS: Record<GameType, GameConfig> = {
	[GameType.Stellaris]: {
		languageId: 'clauswitz',
		displayName: 'Stellaris',
		filePatterns: ['**/common/**/*.txt', '**/*.mod', '**/*.gui', '**/*.gfx']
	},
	[GameType.Victoria3]: {
		languageId: 'clauswitz',
		displayName: 'Victoria 3',
		filePatterns: ['**/common/**/*.txt', '**/*.mod', '**/*.gui', '**/*.gfx']
	}
};

// Global state - only one client active at a time
let activeClient: LanguageClient | null = null;
let activeGameType: GameType | null = null;
let outputChannel: OutputChannel;
let statusBarItem: StatusBarItem;
let processedDocuments = new Set<string>();



/**
 * Update the status bar item to show the current game
 */
function updateStatusBar() {
	if (activeGameType) {
		const config = GAME_CONFIGS[activeGameType];
		statusBarItem.text = `$(game) ${config.displayName}`;
		statusBarItem.tooltip = `Current game: ${config.displayName}. Click to switch games.`;
		statusBarItem.show();
	} else {
		statusBarItem.text = `$(game) No Game`;
		statusBarItem.tooltip = 'No language server active. Click to start one.';
		statusBarItem.show();
	}
}

/**
 * Set language to Clauswitz if it looks like a game file
 */
function setLanguageForDocument(document: TextDocument) {
	const filePath = document.uri.fsPath;
	
	if (document.languageId === 'plaintext') {
		const documentKey = `${filePath}:${document.languageId}`;
		
		// Prevent processing the same document multiple times
		if (processedDocuments.has(documentKey)) {
			return;
		}
		
		const normalizedPath = path.normalize(filePath).replace(/\\/g, '/');
		
		// Simple heuristic: if it's in common/ and .txt, or is .mod/.gui/.gfx, set to clauswitz
		if ((normalizedPath.includes('/common/') && filePath.endsWith('.txt')) ||
			filePath.endsWith('.mod') || filePath.endsWith('.gui') || filePath.endsWith('.gfx')) {
			log(`🎯 Setting Clauswitz language for: ${filePath}`);
			processedDocuments.add(documentKey);
			languages.setTextDocumentLanguage(document, 'clauswitz');
		}
	}
}

/**
 * Create and configure a language client for a specific game
 */
async function createClientForGame(gameType: GameType, context: ExtensionContext): Promise<LanguageClient> {
	const config = GAME_CONFIGS[gameType];
	log(`🔧 Creating ${config.displayName} language client...`);
	
	// Try bundled executable first, fall back to cargo for development
	const executableName = process.platform === 'win32' ? 'cw_lsp.exe' : 'cw_lsp';
	const serverExecutable = path.join(context.extensionPath, 'server', executableName);
	log(`Checking for bundled LSP executable: ${serverExecutable}`);
	
	let serverOptions: ServerOptions;
	
	if (fs.existsSync(serverExecutable)) {
		// Production mode: use bundled executable
		log(`✅ Using bundled LSP server executable for ${config.displayName}`);
		serverOptions = {
			run: { 
				command: serverExecutable,
				args: ['--game', gameType]
			},
			debug: {
				command: serverExecutable,
				args: ['--game', gameType]
			}
		};
	} else {
		// Development mode: compile with cargo
		log(`🔄 Bundled executable not found, falling back to cargo for ${config.displayName} (development mode)`);
		
		const serverCommand = 'cargo';
		const serverArgs = ['run', '--release', '--bin', 'cw_lsp', '--', '--game', gameType];
		const serverWorkingDirectory = path.join(context.extensionPath, '..', 'lsp');
		
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
	}

	// Client options for this specific game (using shared Clauswitz language)
	const clientOptions: LanguageClientOptions = {
		documentSelector: [
			{ scheme: 'file', language: 'clauswitz' },
			...config.filePatterns.map(pattern => ({ scheme: 'file', language: 'plaintext', pattern }))
		],
		synchronize: {
			fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
		},
		outputChannel: outputChannel,
		revealOutputChannelOn: 4 // Show on error
	};

	// Create the language client
	const client = new LanguageClient(
		`cw${gameType}LanguageServer`,
		`CW ${config.displayName} Language Server`,
		serverOptions,
		clientOptions
	);

	// Add event listeners for debugging
	client.onDidChangeState((event) => {
		log(`🔄 ${config.displayName} client state changed: ${State[event.oldState]} -> ${State[event.newState]}`);
		
		if (event.newState === State.Running) {
			log(`✅ ${config.displayName} LSP server is now running!`);
		} else if (event.newState === State.Stopped) {
			log(`🛑 ${config.displayName} LSP server stopped`);
			if (event.oldState === State.Starting) {
				log(`❌ ${config.displayName} server failed to start`);
				window.showErrorMessage(`CW LSP: ${config.displayName} server faiAled to start. Check the output for details.`);
			}
		}
	});

	return client;
}

/**
 * Get or create a language client for a specific game (only one active at a time)
 */
async function getOrCreateClient(gameType: GameType, context: ExtensionContext): Promise<LanguageClient> {
	// If we already have the right client active, return it
	if (activeClient && activeGameType === gameType) {
		return activeClient;
	}
	
	// Stop the current client if it's for a different game
	if (activeClient && activeGameType !== gameType) {
		log(`🔄 Switching from ${GAME_CONFIGS[activeGameType!].displayName} to ${GAME_CONFIGS[gameType].displayName}`);
		try {
			await activeClient.stop();
			log(`✅ ${GAME_CONFIGS[activeGameType!].displayName} client stopped`);
		} catch (error) {
			log(`❌ Error stopping ${GAME_CONFIGS[activeGameType!].displayName} client: ${error}`);
		}
		activeClient = null;
		activeGameType = null;
		updateStatusBar();
	}
	
	// Create and start the new client
	const client = await createClientForGame(gameType, context);
	
	try {
		await client.start();
		activeClient = client;
		activeGameType = gameType;
		
		// Add client to context subscriptions for proper cleanup
		context.subscriptions.push(client);
		
		// Update status bar to show current game
		updateStatusBar();
		
		log(`✅ ${GAME_CONFIGS[gameType].displayName} client started successfully`);
	} catch (error) {
		log(`❌ Error starting ${GAME_CONFIGS[gameType].displayName} client: ${error}`);
		throw error;
	}
	
	return client;
}

export function activate(context: ExtensionContext) {
	// Debug: Check if activate function is called
	console.log('🚀 CW LSP Extension: ACTIVATE FUNCTION CALLED!');
	
	// Create output channel for logging
	outputChannel = window.createOutputChannel('CW LSP Extension');
	outputChannel.show(true);
	
	// Create status bar item
	statusBarItem = window.createStatusBarItem(StatusBarAlignment.Left, 100);
	statusBarItem.command = 'cwlsp.switchGame';
	context.subscriptions.push(statusBarItem);
	updateStatusBar();
	
	log('🚀 CW LSP Extension activating...');
	log(`Extension path: ${context.extensionPath}`);

	// Language servers will start on-demand when manually switched
	log(`💡 Language servers will start on-demand when manually switched via status bar or command`);

	// Set up document event listeners
	const onDidOpenTextDocument = workspace.onDidOpenTextDocument(async (document) => {
		setLanguageForDocument(document);
		// Note: No automatic game server starting - users must manually switch via status bar
	});

	const onDidSaveTextDocument = workspace.onDidSaveTextDocument(async (document) => {
		setLanguageForDocument(document);
	});

	// Listen for when someone manually changes language to 'clauswitz'
	const onDidChangeActiveTextEditor = window.onDidChangeActiveTextEditor(async (editor) => {
		if (editor && editor.document && editor.document.languageId === 'clauswitz') {
			// If no server is running, show a message to guide user to switch manually
			if (!activeClient) {
				log(`📝 Clauswitz file opened but no language server is running. Use status bar to start one.`);
			}
		}
	});

	// Add event listeners to context subscriptions
	context.subscriptions.push(onDidOpenTextDocument);
	context.subscriptions.push(onDidSaveTextDocument);
	context.subscriptions.push(onDidChangeActiveTextEditor);

	// Register commands
	const restartServerCommand = commands.registerCommand('cwlsp.restartServer', async () => {
		log('🔄 Manual server restart requested');
		if (activeClient && activeGameType) {
			await restartServer(activeGameType);
		} else {
			window.showWarningMessage('CW LSP: No server is currently running');
		}
	});

	const restartAllServersCommand = commands.registerCommand('cwlsp.restartAllServers', async () => {
		log('🔄 Manual restart requested (same as single restart in single-client mode)');
		if (activeClient && activeGameType) {
			await restartServer(activeGameType);
		} else {
			window.showWarningMessage('CW LSP: No server is currently running');
		}
	});

	const switchGameCommand = commands.registerCommand('cwlsp.switchGame', async () => {
		log('🔄 Manual game switch requested');
		
		const gameOptions = Object.entries(GAME_CONFIGS).map(([gameType, config]) => ({
			label: config.displayName,
			detail: `Switch to ${config.displayName} language server`,
			gameType: gameType as GameType
		}));

		const selected = await window.showQuickPick(gameOptions, {
			placeHolder: 'Select game to switch to'
		});

		if (selected) {
			try {
				await getOrCreateClient(selected.gameType, context);
				window.showInformationMessage(`CW LSP: Switched to ${selected.label} server`);
			} catch (error) {
				log(`❌ Failed to switch to ${selected.label}: ${error}`);
				window.showErrorMessage(`CW LSP: Failed to switch to ${selected.label}: ${error}`);
			}
		}
	});

	// Add commands to context subscriptions
	context.subscriptions.push(restartServerCommand);
	context.subscriptions.push(restartAllServersCommand);
	context.subscriptions.push(switchGameCommand);

	// Active client will be added to subscriptions when created

	log('📝 Extension activation completed');
}

/**
 * Restart the active language server
 */
async function restartServer(gameType: GameType) {
	const config = GAME_CONFIGS[gameType];
	
	if (activeClient && activeGameType === gameType) {
		log(`🛑 Stopping ${config.displayName} server...`);
		try {
			await activeClient.stop();
			log(`✅ ${config.displayName} server stopped successfully`);
		} catch (error) {
			log(`❌ Error stopping ${config.displayName} server: ${error}`);
		}
		
		log(`🚀 Starting ${config.displayName} server again...`);
		try {
			await activeClient.start();
			log(`✅ ${config.displayName} server restarted successfully`);
			window.showInformationMessage(`CW LSP: ${config.displayName} server restarted successfully`);
		} catch (error) {
			log(`❌ Error restarting ${config.displayName} server: ${error}`);
			window.showErrorMessage(`CW LSP: Error restarting ${config.displayName} server: ${error}`);
			// Clear the failed client
			activeClient = null;
			activeGameType = null;
			updateStatusBar();
		}
	} else {
		log(`❌ No ${config.displayName} client to restart`);
		window.showWarningMessage(`CW LSP: No ${config.displayName} server to restart`);
	}
}

/**
 * Restart the active language server (same as restartServer in single-client mode)
 */
async function restartAllServers() {
	if (activeClient && activeGameType) {
		await restartServer(activeGameType);
	} else {
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
	
	if (!activeClient) {
		return undefined;
	}

	// Stop the active client
	return activeClient.stop().then(() => {
		activeClient = null;
		activeGameType = null;
		updateStatusBar();
		log('✅ Language server stopped');
	});
} 
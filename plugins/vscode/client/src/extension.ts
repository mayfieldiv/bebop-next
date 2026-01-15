import { exec, execSync, spawn } from "child_process";
import * as fs from "fs";
import * as path from "path";
import {
	commands,
	type ExtensionContext,
	type OutputChannel,
	type StatusBarItem,
	StatusBarAlignment,
	Task,
	TaskScope,
	type TaskProvider,
	ShellExecution,
	Uri,
	window,
	workspace,
	env,
} from "vscode";

import {
	type Executable,
	LanguageClient,
	type LanguageClientOptions,
	type ServerOptions,
} from "vscode-languageclient/node";

const MIN_VERSION = { major: 4, minor: 0, patch: 0 };
const DOCS_URL = "https://bebop.sh/docs";

let client: LanguageClient | undefined;
let outputChannel: OutputChannel;
let statusBarItem: StatusBarItem;
let bebopcPath: string | null = null;
let bebopcVersion: BebopcVersion | null = null;
let configPath: string | null = null;
let watchProcess: ReturnType<typeof spawn> | null = null;

interface BebopcVersion {
	major: number;
	minor: number;
	patch: number;
	prerelease?: string;
	isValid: boolean;
	raw: string;
}

function log(msg: string) {
	const timestamp = new Date().toISOString();
	outputChannel.appendLine(`[${timestamp}] ${msg}`);
}

function parseVersion(versionOutput: string): BebopcVersion {
	const trimmed = versionOutput.trim();
	const match = trimmed.match(/^bebopc\s+(\d+)\.(\d+)\.(\d+)(?:-(.+))?$/);
	if (match) {
		return {
			major: parseInt(match[1], 10),
			minor: parseInt(match[2], 10),
			patch: parseInt(match[3], 10),
			prerelease: match[4],
			isValid: true,
			raw: trimmed,
		};
	}
	return { major: 0, minor: 0, patch: 0, isValid: false, raw: trimmed };
}

function isVersionCompatible(version: BebopcVersion): boolean {
	if (!version.isValid) return false;
	if (version.major === 0 && version.minor === 0 && version.patch === 0 && version.prerelease) {
		return true;
	}
	if (version.major > MIN_VERSION.major) return true;
	if (version.major < MIN_VERSION.major) return false;
	if (version.minor > MIN_VERSION.minor) return true;
	if (version.minor < MIN_VERSION.minor) return false;
	return version.patch >= MIN_VERSION.patch;
}

function getBebopcVersion(bebopcPath: string): BebopcVersion | null {
	try {
		const output = execSync(`"${bebopcPath}" --version`, {
			encoding: "utf-8",
			timeout: 5000,
		});
		return parseVersion(output);
	} catch {
		return null;
	}
}

function hasLspCommand(bebopcPath: string): boolean {
	try {
		// Try running lsp with --help to check if the command exists
		execSync(`"${bebopcPath}" lsp --help`, {
			encoding: "utf-8",
			timeout: 5000,
			stdio: ["ignore", "pipe", "pipe"],
		});
		return true;
	} catch {
		return false;
	}
}

function findBebopc(): string | null {
	const config = workspace.getConfiguration("bebop");
	const configuredPath = config.get<string>("compiler.path");

	if (configuredPath && configuredPath.trim() !== "") {
		if (fs.existsSync(configuredPath)) {
			return configuredPath;
		}
		log(`Configured path does not exist: ${configuredPath}`);
		return null;
	}

	try {
		const which = process.platform === "win32" ? "where" : "which";
		const result = execSync(`${which} bebopc`, { encoding: "utf-8", timeout: 5000 });
		const foundPath = result.trim().split("\n")[0];
		if (foundPath && fs.existsSync(foundPath)) {
			return foundPath;
		}
	} catch {
		// Not on PATH
	}

	return null;
}

async function findConfigFile(): Promise<string | null> {
	const config = workspace.getConfiguration("bebop");
	const configuredPath = config.get<string>("compiler.configPath");

	if (configuredPath && configuredPath.trim() !== "") {
		if (fs.existsSync(configuredPath)) {
			return configuredPath;
		}
	}

	const workspaceFolders = workspace.workspaceFolders;
	if (!workspaceFolders) return null;

	for (const folder of workspaceFolders) {
		for (const name of ["bebop.yml", "bebop.yaml"]) {
			const p = path.join(folder.uri.fsPath, name);
			if (fs.existsSync(p)) {
				return p;
			}
		}
	}

	return null;
}

let hasLsp = false;

function updateStatusBar() {
	if (!bebopcPath) {
		statusBarItem.text = "$(warning) Bebop: Not Found";
		statusBarItem.tooltip = "bebopc not found. Click to configure.";
		statusBarItem.command = "workbench.action.openSettings";
		statusBarItem.show();
		return;
	}

	if (bebopcVersion && !isVersionCompatible(bebopcVersion)) {
		statusBarItem.text = `$(warning) Bebop: ${bebopcVersion.raw}`;
		statusBarItem.tooltip = `Version ${bebopcVersion.raw} is not compatible. Requires >= ${MIN_VERSION.major}.${MIN_VERSION.minor}.${MIN_VERSION.patch}`;
		statusBarItem.command = "bebop.openDocs";
		statusBarItem.show();
		return;
	}

	if (!hasLsp) {
		statusBarItem.text = "$(warning) Bebop: No LSP";
		statusBarItem.tooltip = "bebopc does not support language server. Update to >= 4.0.0";
		statusBarItem.command = "workbench.action.openSettings";
		statusBarItem.show();
		return;
	}

	const versionStr = bebopcVersion ? bebopcVersion.raw.replace("bebopc ", "") : "unknown";
	if (watchProcess) {
		statusBarItem.text = `$(eye) Bebop: ${versionStr}`;
		statusBarItem.tooltip = "Bebop watch mode active. Click to stop.";
		statusBarItem.command = "bebop.stopWatch";
	} else {
		statusBarItem.text = `$(check) Bebop: ${versionStr}`;
		statusBarItem.tooltip = `Bebop ${versionStr}\nClick to build`;
		statusBarItem.command = "bebop.build";
	}
	statusBarItem.show();
}

async function runBuild() {
	if (!bebopcPath) {
		window.showErrorMessage("bebopc not found. Please configure bebop.compiler.path");
		return;
	}

	const cfg = await findConfigFile();
	if (!cfg) {
		window.showErrorMessage("No bebop.yml found. Run 'Bebop: Initialize Project' first.");
		return;
	}

	const terminal = window.createTerminal("Bebop Build");
	terminal.show();
	terminal.sendText(`"${bebopcPath}" build --config "${cfg}"`);
}

async function runWatch() {
	if (!bebopcPath) {
		window.showErrorMessage("bebopc not found. Please configure bebop.compiler.path");
		return;
	}

	const cfg = await findConfigFile();
	if (!cfg) {
		window.showErrorMessage("No bebop.yml found. Run 'Bebop: Initialize Project' first.");
		return;
	}

	if (watchProcess) {
		window.showInformationMessage("Watch mode is already running.");
		return;
	}

	log("Starting watch mode...");
	watchProcess = spawn(bebopcPath, ["watch", "--config", cfg], {
		stdio: ["ignore", "pipe", "pipe"],
	});

	watchProcess.stdout?.on("data", (data) => {
		log(`[watch] ${data.toString().trim()}`);
	});

	watchProcess.stderr?.on("data", (data) => {
		log(`[watch] ${data.toString().trim()}`);
	});

	watchProcess.on("close", (code) => {
		log(`Watch process exited with code ${code}`);
		watchProcess = null;
		updateStatusBar();
	});

	updateStatusBar();
	window.showInformationMessage("Bebop watch mode started.");
}

function stopWatch() {
	if (watchProcess) {
		watchProcess.kill();
		watchProcess = null;
		updateStatusBar();
		window.showInformationMessage("Bebop watch mode stopped.");
	}
}

async function initProject() {
	const workspaceFolders = workspace.workspaceFolders;
	if (!workspaceFolders || workspaceFolders.length === 0) {
		window.showErrorMessage("No workspace folder open.");
		return;
	}

	const folder = workspaceFolders.length === 1
		? workspaceFolders[0]
		: await window.showWorkspaceFolderPick({ placeHolder: "Select folder for bebop.yml" });

	if (!folder) return;

	const configPath = path.join(folder.uri.fsPath, "bebop.yml");
	if (fs.existsSync(configPath)) {
		window.showWarningMessage("bebop.yml already exists in this folder.");
		return;
	}

	const template = `# Bebop Configuration
# See https://bebop.sh/docs/configuration for full options

edition: "2026"

generators:
  # TypeScript
  # - type: typescript
  #   output: ./src/generated/bebop.ts
  #   options:
  #     emitNotice: true

  # C#
  # - type: csharp
  #   output: ./Generated/Bebop.cs
  #   options:
  #     namespace: MyApp.Models

includes:
  - "**/*.bop"

excludes:
  - "**/node_modules/**"
`;

	fs.writeFileSync(configPath, template);
	const doc = await workspace.openTextDocument(configPath);
	await window.showTextDocument(doc);
	window.showInformationMessage("Created bebop.yml. Configure your generators to get started.");

	commands.executeCommand("setContext", "bebop.hasConfig", true);
}

function restartServer() {
	if (client) {
		log("Restarting language server...");
		client.stop().then(() => {
			startLanguageClient();
		});
	} else {
		startLanguageClient();
	}
}

function showOutput() {
	outputChannel.show();
}

function openDocs() {
	env.openExternal(Uri.parse(DOCS_URL));
}

function startLanguageClient() {
	if (!bebopcPath || !bebopcVersion || !isVersionCompatible(bebopcVersion)) {
		return;
	}

	const config = workspace.getConfiguration("bebop");
	const includePaths = config.get<string[]>("compiler.includePaths") || [];
	const cfgPath = config.get<string>("compiler.configPath") || "";

	const args = ["lsp"];
	for (const inc of includePaths) {
		args.push("-I", inc);
	}
	if (cfgPath) {
		args.push("--config", cfgPath);
	}
	log(`Server args: ${args.join(" ")}`);

	const run: Executable = {
		command: bebopcPath,
		args: args,
		options: { env: { ...process.env } },
	};

	const serverOptions: ServerOptions = { run, debug: run };

	const clientOptions: LanguageClientOptions = {
		documentSelector: [
			{ scheme: "file", language: "bebop" },
			{ scheme: "untitled", language: "bebop" },
		],
		synchronize: {
			fileEvents: workspace.createFileSystemWatcher("**/*.bop"),
		},
		outputChannel: outputChannel,
	};

	client = new LanguageClient("bebop", "Bebop Language Server", serverOptions, clientOptions);

	log("Starting language client...");
	client.start().then(() => {
		log("Language client started successfully");
	}).catch((err) => {
		log(`Failed to start language client: ${err}`);
		window.showErrorMessage(`Bebop: Failed to start language server: ${err.message}`);
	});
}

class BebopTaskProvider implements TaskProvider {
	provideTasks(): Task[] {
		const tasks: Task[] = [];

		if (bebopcPath && configPath) {
			tasks.push(new Task(
				{ type: "bebop", command: "build" },
				TaskScope.Workspace,
				"Build",
				"bebop",
				new ShellExecution(`"${bebopcPath}" build --config "${configPath}"`),
				"$bebopc"
			));

			tasks.push(new Task(
				{ type: "bebop", command: "watch" },
				TaskScope.Workspace,
				"Watch",
				"bebop",
				new ShellExecution(`"${bebopcPath}" watch --config "${configPath}"`),
				"$bebopc"
			));
		}

		return tasks;
	}

	resolveTask(task: Task): Task | undefined {
		const command = task.definition.command as string;
		const cfg = task.definition.config || configPath;

		if (bebopcPath && cfg) {
			return new Task(
				task.definition,
				TaskScope.Workspace,
				task.name,
				"bebop",
				new ShellExecution(`"${bebopcPath}" ${command} --config "${cfg}"`),
				"$bebopc"
			);
		}

		return undefined;
	}
}


export async function activate(context: ExtensionContext) {
	outputChannel = window.createOutputChannel("Bebop");
	context.subscriptions.push(outputChannel);

	statusBarItem = window.createStatusBarItem(StatusBarAlignment.Left, 100);
	context.subscriptions.push(statusBarItem);

	log("Bebop extension activating...");

	// Find bebopc
	bebopcPath = findBebopc();
	if (bebopcPath) {
		log(`Found bebopc: ${bebopcPath}`);
		bebopcVersion = getBebopcVersion(bebopcPath);
		if (bebopcVersion) {
			log(`bebopc version: ${bebopcVersion.raw}`);
		}
		hasLsp = hasLspCommand(bebopcPath);
		log(`LSP support: ${hasLsp}`);
	} else {
		log("bebopc not found");
	}

	// Find config file
	configPath = await findConfigFile();
	commands.executeCommand("setContext", "bebop.hasConfig", configPath !== null);
	if (configPath) {
		log(`Found config: ${configPath}`);
	}

	// Update status bar
	updateStatusBar();

	// Register commands
	context.subscriptions.push(
		commands.registerCommand("bebop.build", runBuild),
		commands.registerCommand("bebop.watch", runWatch),
		commands.registerCommand("bebop.stopWatch", stopWatch),
		commands.registerCommand("bebop.init", initProject),
		commands.registerCommand("bebop.restartServer", restartServer),
		commands.registerCommand("bebop.showOutput", showOutput),
		commands.registerCommand("bebop.openDocs", openDocs),
	);

	// Register task provider
	context.subscriptions.push(
		workspace.registerTaskProvider("bebop", new BebopTaskProvider())
	);

	// Watch for config changes
	context.subscriptions.push(
		workspace.onDidChangeConfiguration((e) => {
			if (e.affectsConfiguration("bebop")) {
				const newPath = findBebopc();
				if (newPath !== bebopcPath) {
					bebopcPath = newPath;
					bebopcVersion = bebopcPath ? getBebopcVersion(bebopcPath) : null;
					updateStatusBar();
					restartServer();
				}
			}
		})
	);

	// Watch for bebop.yml changes
	const configWatcher = workspace.createFileSystemWatcher("**/bebop.{yml,yaml}");
	context.subscriptions.push(configWatcher);
	configWatcher.onDidCreate(async () => {
		configPath = await findConfigFile();
		commands.executeCommand("setContext", "bebop.hasConfig", configPath !== null);
	});
	configWatcher.onDidDelete(async () => {
		configPath = await findConfigFile();
		commands.executeCommand("setContext", "bebop.hasConfig", configPath !== null);
	});

	// Show error if bebopc not found or incompatible
	if (!bebopcPath) {
		const selection = await window.showErrorMessage(
			"Bebop compiler (bebopc) not found. Please install bebopc or configure the path in settings.",
			"Open Settings",
			"Learn More"
		);
		if (selection === "Open Settings") {
			commands.executeCommand("workbench.action.openSettings", "bebop.compiler.path");
		} else if (selection === "Learn More") {
			openDocs();
		}
	} else if (bebopcVersion && !isVersionCompatible(bebopcVersion)) {
		const selection = await window.showErrorMessage(
			`bebopc version ${bebopcVersion.raw} is not compatible. Requires >= ${MIN_VERSION.major}.${MIN_VERSION.minor}.${MIN_VERSION.patch}`,
			"Learn More"
		);
		if (selection === "Learn More") {
			openDocs();
		}
	} else if (!hasLspCommand(bebopcPath)) {
		log("bebopc does not support 'lsp' command");
		const selection = await window.showErrorMessage(
			`bebopc at "${bebopcPath}" does not support the language server. Please update to bebopc >= 4.0.0`,
			"Open Settings",
			"Learn More"
		);
		if (selection === "Open Settings") {
			commands.executeCommand("workbench.action.openSettings", "bebop.compiler.path");
		} else if (selection === "Learn More") {
			openDocs();
		}
	} else {
		// Start language server
		startLanguageClient();
		if (client) {
			context.subscriptions.push(client);
		}
	}

	log("Activation complete");
}

export function deactivate(): Thenable<void> | undefined {
	if (watchProcess) {
		watchProcess.kill();
		watchProcess = null;
	}
	if (!client) {
		return undefined;
	}
	return client.stop();
}

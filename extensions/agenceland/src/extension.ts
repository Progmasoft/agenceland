// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

import { execFile } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import { findInstructions } from './core/instructions';
import { agencelandHome, mainWorktreeFromList, Platform, projectDirectories, userDirectory } from './core/paths';
import { mergeSettings, parseSettings, Settings, settingsLayers } from './core/settings';

const run = promisify(execFile);

export function activate(context: vscode.ExtensionContext): void {
	const output = vscode.window.createOutputChannel('Agenceland');
	context.subscriptions.push(
		output,
		vscode.commands.registerCommand('agenceland.showProjectContext', async () => {
			output.clear();
			output.show(true);
			try {
				output.appendLine(await describeContext());
			} catch (error) {
				output.appendLine(`The project context could not be read: ${error instanceof Error ? error.message : String(error)}`);
			}
		}),
	);
}

function platform(): Platform {
	const current = process.platform;
	if (current === 'linux' || current === 'darwin' || current === 'win32') {
		return current;
	}
	throw new Error(`Agenceland does not run on ${current}`);
}

/** What an agent of each workspace folder would read and where it would write. */
async function describeContext(): Promise<string> {
	const p = platform();
	const home = agencelandHome(p, process.env, os.homedir());
	const user = userDirectory(p, home);
	const lines: string[] = [`Agenceland home: ${home}`, `User directory: ${user}`];
	// The extension is always on; trust decides whether its agents may act.
	lines.push(vscode.workspace.isTrusted
		? 'Workspace: trusted; agents may run commands and change files as the permission mode allows.'
		: 'Workspace: not trusted; there is no agent window here and no agent runs.');

	const folders = vscode.workspace.workspaceFolders ?? [];
	if (folders.length === 0) {
		lines.push('', 'No folder is open: there is no project.');
		return lines.join('\n');
	}
	for (const folder of folders) {
		lines.push('', ...await describeFolder(folder, p, home, user));
	}
	return lines.join('\n');
}

/** What an agent of one workspace folder would read and where it would write. */
async function describeFolder(folder: vscode.WorkspaceFolder, p: Platform, home: string, user: string): Promise<string[]> {
	if (folder.uri.scheme !== 'file') {
		return [`${folder.name}: not a local folder (${folder.uri.scheme}); Agenceland works on local projects.`];
	}
	// The instructions, settings and shared memory are those of the open
	// worktree; the private memory and sessions are keyed on the main one.
	const { worktree: project, main } = await projectOf(folder.uri.fsPath);
	const directories = projectDirectories(p, home, main);
	const instructions = findInstructions(
		relative => isFile(path.join(project, relative)),
		relative => isFile(path.join(user, relative)),
	);
	const lines = [`Project ${folder.name}: ${project}`];
	if (main !== project) {
		lines.push(`  Main worktree: ${main}`);
	}
	const instructionsPath = instructions ? path.join(instructions.scope === 'user' ? user : project, instructions.file) : undefined;
	lines.push(`  Instructions: ${instructions && instructionsPath ? `${instructionsPath} (${instructions.scope})` : 'none'}`);
	lines.push(`  Private memory: ${directories.memory}`);
	lines.push(`  Shared memory: ${path.join(project, '.agenceland', 'memory')}`);
	lines.push(`  Session records: ${directories.sessions}`);
	lines.push(...describeSettings(user, project));
	return lines;
}

/** The settings layers of a project, each read or not, and how many keys are in effect. */
function describeSettings(user: string, project: string): string[] {
	const lines: string[] = [];
	const layers: Settings[] = [];
	for (const layer of settingsLayers) {
		const file = path.join(layer.scope === 'user' ? user : project, layer.file);
		const text = readText(file);
		if (text === undefined) {
			lines.push(`  Settings (${layer.scope}): ${file}, none`);
			continue;
		}
		const parsed = parseSettings(text);
		if (parsed.ok) {
			layers.push(parsed.settings);
			lines.push(`  Settings (${layer.scope}): ${file}, ${Object.keys(parsed.settings).length} key(s)`);
		} else {
			lines.push(`  Settings (${layer.scope}): ${file}, not read: ${parsed.errors.join('; ')}`);
		}
	}
	lines.push(`  Settings in effect: ${Object.keys(mergeSettings(layers)).length} key(s)`);
	return lines;
}

/**
 * The worktree a folder is in and the main worktree of its repository, both
 * canonical; outside Git both are the folder itself.
 */
function projectOf(folder: string): Promise<{ worktree: string; main: string }> {
	const outsideGit = () => {
		const itself = canonical(folder);
		return { worktree: itself, main: itself };
	};
	return run('git', ['rev-parse', '--show-toplevel'], { cwd: folder })
		.then(async top => {
			const worktree = canonical(top.stdout.trim());
			const list = await run('git', ['worktree', 'list', '--porcelain'], { cwd: folder });
			const main = mainWorktreeFromList(list.stdout);
			return { worktree, main: main === undefined ? worktree : canonical(main) };
		})
		// Not in Git, or Git is not installed.
		.catch(outsideGit);
}

/** The path with its links resolved, or the path as it is when that fails. */
function canonical(file: string): string {
	try {
		return fs.realpathSync.native(file);
	} catch {
		return file;
	}
}

// The paths below are built from the Agenceland home and the open folders,
// not taken from text a project or a model wrote.
function isFile(file: string): boolean {
	try {
		// eslint-disable-next-line security/detect-non-literal-fs-filename -- see above
		return fs.statSync(file).isFile();
	} catch {
		return false;
	}
}

function readText(file: string): string | undefined {
	try {
		// eslint-disable-next-line security/detect-non-literal-fs-filename -- see above
		return fs.readFileSync(file, 'utf8');
	} catch {
		return undefined;
	}
}

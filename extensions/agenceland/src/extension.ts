// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

import { execFile } from 'node:child_process';
import * as fs from 'node:fs';
import * as os from 'node:os';
import * as path from 'node:path';
import { promisify } from 'node:util';
import * as vscode from 'vscode';
import { findProjectInstructions, userInstructionFile } from './core/instructions';
import { agencelandHome, mainWorktreeOf, Platform, projectDirectories, userDirectory } from './core/paths';
import { mergeSettings, parseSettings, Settings, settingsLayers } from './core/settings';

const run = promisify(execFile);

export function activate(context: vscode.ExtensionContext): void {
	const output = vscode.window.createOutputChannel('Agenceland');
	context.subscriptions.push(
		output,
		vscode.commands.registerCommand('agenceland.showProjectContext', async () => {
			output.clear();
			output.show(true);
			output.appendLine(await describeContext());
		}),
	);
}

export function deactivate(): void { }

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
	const userInstructions = path.join(user, userInstructionFile);
	lines.push(`User instructions: ${isFile(userInstructions) ? userInstructions : 'none'}`);

	const folders = vscode.workspace.workspaceFolders ?? [];
	if (folders.length === 0) {
		lines.push('', 'No folder is open: there is no project.');
		return lines.join('\n');
	}
	for (const folder of folders) {
		if (folder.uri.scheme !== 'file') {
			lines.push('', `${folder.name}: not a local folder (${folder.uri.scheme}); Agenceland works on local projects.`);
			continue;
		}
		const project = await projectRoot(folder.uri.fsPath);
		const directories = projectDirectories(p, home, project);
		const instructions = findProjectInstructions(relative => isFile(path.join(project, relative)));
		lines.push('', `Project ${folder.name}: ${project}`);
		lines.push(`  Instructions: ${instructions ? path.join(project, instructions) : 'none'}`);
		lines.push(`  Private memory: ${directories.memory}`);
		lines.push(`  Shared memory: ${path.join(project, '.agenceland', 'memory')}`);
		lines.push(`  Session records: ${directories.sessions}`);

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
	}
	return lines.join('\n');
}

/**
 * The project a folder belongs to: the main worktree of its Git repository,
 * so that the session worktrees of a repository share one project, or the
 * folder itself outside Git. Either way its canonical path, links resolved.
 */
async function projectRoot(folder: string): Promise<string> {
	try {
		const { stdout } = await run('git', ['rev-parse', '--path-format=absolute', '--git-common-dir'], { cwd: folder });
		return fs.realpathSync.native(mainWorktreeOf(platform(), stdout.trim()));
	} catch {
		return fs.realpathSync.native(folder);
	}
}

function isFile(file: string): boolean {
	try {
		return fs.statSync(file).isFile();
	} catch {
		return false;
	}
}

function readText(file: string): string | undefined {
	try {
		return fs.readFileSync(file, 'utf8');
	} catch {
		return undefined;
	}
}

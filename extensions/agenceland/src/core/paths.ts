// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

// Where Agenceland keeps what belongs to the user and to each project. These
// paths are part of the product: they are documented, and agents and people
// edit the files below them.

import { createHash } from 'node:crypto';
import * as path from 'node:path';

/** The platforms Agenceland runs on, as Node.js names them. */
export type Platform = 'linux' | 'darwin' | 'win32';

/** Environment variables, as `process.env` holds them. */
export type Environment = Readonly<Record<string, string | undefined>>;

/**
 * The directory of Agenceland's own files: `$XDG_CONFIG_HOME/Agenceland` or
 * `~/.config/Agenceland` on Linux and macOS, `%APPDATA%\Agenceland` on
 * Windows. It is the user data directory of the editor too, so the editor's
 * `User/settings.json` is the user settings of Agenceland.
 */
export function agencelandHome(platform: Platform, environment: Environment, home: string): string {
	if (platform === 'win32') {
		// An empty APPDATA is as good as none.
		const configured = environment.APPDATA;
		const appData = configured !== undefined && configured !== '' ? configured : path.win32.join(home, 'AppData', 'Roaming');
		return path.win32.join(appData, 'Agenceland');
	}
	// A relative XDG_CONFIG_HOME is invalid by the specification and ignored.
	const configured = environment.XDG_CONFIG_HOME;
	const config = configured !== undefined && path.posix.isAbsolute(configured) ? configured : path.posix.join(home, '.config');
	return path.posix.join(config, 'Agenceland');
}

function pathOf(platform: Platform): path.PlatformPath {
	return platform === 'win32' ? path.win32 : path.posix;
}

/** `User/` below the home: settings, memory, skills, commands, agents and AGENCELAND.md of the user. */
export function userDirectory(platform: Platform, agencelandHomePath: string): string {
	return pathOf(platform).join(agencelandHomePath, 'User');
}

function encode(text: string): string {
	return text.replace(/[^A-Za-z0-9]/g, '-');
}

/**
 * The most characters of a project key's readable part. With the hash the
 * key stays well below the 255 bytes a file system allows for one name.
 */
export const maxReadableKeyLength = 200;

/**
 * The name of a project's directory below `Projects/`: the canonical path of
 * the project with every character that is not an ASCII letter or digit
 * turned into `-`, then `-` and the first 16 hexadecimal digits of the
 * SHA-256 of that path. On Linux and macOS the separator before the project's
 * own folder becomes `--`: `/home/ada/Projects/example` is
 * `-home-ada-Projects--example-<hash>`. On Windows every separator is one
 * `-`: `C:\Users\Ada\source\repos\example` is
 * `C--Users-Ada-source-repos-example-<hash>`. The readable part tells a person
 * which project it is; the hash keeps two paths that read the same apart.
 */
export function projectKey(platform: Platform, canonicalPath: string): string {
	let readable = encode(canonicalPath);
	if (platform !== 'win32') {
		const last = canonicalPath.lastIndexOf('/');
		if (last > 0) {
			readable = `${encode(canonicalPath.slice(0, last))}--${encode(canonicalPath.slice(last + 1))}`;
		}
	}
	// A longer path keeps its end, where the project's own folder is; the hash
	// of the whole path still tells two projects apart.
	if (readable.length > maxReadableKeyLength) {
		readable = readable.slice(-maxReadableKeyLength);
	}
	const hash = createHash('sha256').update(canonicalPath, 'utf8').digest('hex').slice(0, 16);
	return `${readable}-${hash}`;
}

/** What Agenceland keeps of one project on this machine, outside the project. */
export interface ProjectDirectories {
	/** `Projects/<key>/`. */
	readonly root: string;
	/** `memory/`: the private memory of the project, never committed. */
	readonly memory: string;
	/** `sessions/`: the records of the project's agent sessions, for replay. */
	readonly sessions: string;
}

export function projectDirectories(platform: Platform, agencelandHomePath: string, canonicalProjectPath: string): ProjectDirectories {
	const p = pathOf(platform);
	const root = p.join(agencelandHomePath, 'Projects', projectKey(platform, canonicalProjectPath));
	return { root, memory: p.join(root, 'memory'), sessions: p.join(root, 'sessions') };
}

/**
 * The main worktree of a repository, from the output of `git worktree list
 * --porcelain`: its first entry, whatever the layout of the Git directory
 * (a submodule, a separate Git directory, a bare repository). Every session
 * worktree of a repository keys its private memory on it. `undefined` when
 * the output names no worktree.
 */
export function mainWorktreeFromList(porcelain: string): string | undefined {
	for (const line of porcelain.split(/\r?\n/)) {
		if (line.startsWith('worktree ')) {
			return line.slice('worktree '.length);
		}
		if (line.trim() !== '') {
			return undefined;
		}
	}
	return undefined;
}

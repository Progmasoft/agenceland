// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

// The settings of Agenceland come in three layers; a later layer wins.

// A namespace import: the bundle takes the ESM build of jsonc-parser, which
// has no default export, and Node.js the CommonJS one.
import * as jsonc from 'jsonc-parser';

/** A layer of settings and where it comes from. */
export interface SettingsLayer {
	/** `user`, `project` or `local`. */
	readonly scope: 'user' | 'project' | 'local';
	/** The file, relative to its directory: the user directory or the project. */
	readonly file: string;
}

/**
 * The layers, first to last: the user's `settings.json`, the project's
 * `.agenceland/settings.json` (committed, shared by the team), and
 * `.agenceland/settings.local.json` (this person's, kept out of Git).
 */
export const settingsLayers: readonly SettingsLayer[] = [
	{ scope: 'user', file: 'settings.json' },
	{ scope: 'project', file: '.agenceland/settings.json' },
	{ scope: 'local', file: '.agenceland/settings.local.json' },
];

export type Settings = { readonly [key: string]: unknown };

/** What a settings file held, or why it could not be read. */
export type ParsedSettings =
	| { readonly ok: true; readonly settings: Settings }
	| { readonly ok: false; readonly errors: readonly string[] };

/**
 * Reads a settings file: JSON with comments and trailing commas, as the
 * editor writes `settings.json`. The top level must be an object.
 */
export function parseSettings(text: string): ParsedSettings {
	// A new file, or one with only comments in it, holds no settings.
	if (jsonc.stripComments(text).trim() === '') {
		return { ok: true, settings: {} };
	}
	const errors: jsonc.ParseError[] = [];
	const value: unknown = jsonc.parse(text, errors, { allowTrailingComma: true, disallowComments: false });
	if (errors.length > 0) {
		return {
			ok: false,
			errors: errors.map(error => `${jsonc.printParseErrorCode(error.error)} at offset ${error.offset}`),
		};
	}
	if (!isObject(value)) {
		return { ok: false, errors: ['the top level is not an object'] };
	}
	return { ok: true, settings: value };
}

function isObject(value: unknown): value is Settings {
	return typeof value === 'object' && value !== null && !Array.isArray(value);
}

/**
 * The layers merged, first to last: an object merges key by key with the
 * object below it; anything else, an array included, replaces what is below.
 */
export function mergeSettings(layers: readonly Settings[]): Settings {
	let merged: Settings = {};
	for (const layer of layers) {
		merged = mergeTwo(merged, layer);
	}
	return merged;
}

function mergeTwo(below: Settings, above: Settings): Settings {
	const result: { [key: string]: unknown } = { ...below };
	for (const [key, value] of Object.entries(above)) {
		const current = result[key];
		result[key] = isObject(current) && isObject(value) ? mergeTwo(current, value) : value;
	}
	return result;
}

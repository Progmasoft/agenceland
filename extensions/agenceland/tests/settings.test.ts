// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { mergeSettings, parseSettings, settingsLayers } from '../src/core/settings.ts';

test('the layers are user, project and local, in that order', () => {
	assert.deepEqual(settingsLayers.map(layer => layer.scope), ['user', 'project', 'local']);
	assert.deepEqual(settingsLayers.map(layer => layer.file), ['settings.json', '.agenceland/settings.json', '.agenceland/settings.local.json']);
});

test('settings files are JSON with comments and trailing commas', () => {
	const parsed = parseSettings('{\n  // the mode\n  "agenceland.permissions": "auto",\n}\n');
	assert.deepEqual(parsed, { ok: true, settings: { 'agenceland.permissions': 'auto' } });
	assert.deepEqual(parseSettings(''), { ok: true, settings: {} });
	assert.deepEqual(parseSettings('  \n'), { ok: true, settings: {} });
	assert.deepEqual(parseSettings('// nothing yet\n'), { ok: true, settings: {} });
});

test('a broken file or a top level that is not an object is not read', () => {
	const broken = parseSettings('{ "a": ');
	assert.equal(broken.ok, false);
	for (const text of ['[]', '1', '"text"', 'null']) {
		const parsed = parseSettings(text);
		assert.equal(parsed.ok, false, text);
		if (!parsed.ok) {
			assert.deepEqual(parsed.errors, ['the top level is not an object']);
		}
	}
});

test('a later layer wins; objects merge key by key, arrays are replaced', () => {
	const merged = mergeSettings([
		{ mode: 'ask', sandbox: { network: false, allow: ['a'] }, models: ['x'] },
		{ sandbox: { allow: ['b'] }, models: ['y'] },
		{ mode: 'auto' },
	]);
	assert.deepEqual(merged, { mode: 'auto', sandbox: { network: false, allow: ['b'] }, models: ['y'] });
	assert.deepEqual(mergeSettings([]), {});
});

test('a key named __proto__ stays a key and changes no prototype', () => {
	const parsed = parseSettings('{ "__proto__": { "polluted": true }, "a": { "__proto__": { "b": 1 } } }');
	assert.equal(parsed.ok, true);
	if (!parsed.ok) {
		return;
	}
	const merged = mergeSettings([{}, parsed.settings]);
	assert.equal(Object.getPrototypeOf(merged), Object.prototype);
	assert.equal(({} as Record<string, unknown>)['polluted'], undefined);
	assert.equal((merged as Record<string, unknown>)['polluted'], undefined);
});

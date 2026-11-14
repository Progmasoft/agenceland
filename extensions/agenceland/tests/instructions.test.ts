// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { findInstructions, projectInstructionFiles } from '../src/core/instructions.ts';

const files = (...names: string[]) => (relative: string) => names.includes(relative);
const none = files();

test('the first place of the search order wins and the rest are not looked at', () => {
	assert.deepEqual(projectInstructionFiles, ['AGENCELAND.md', '.agenceland/AGENCELAND.md', '.agents/AGENCELAND.md', '.claude/AGENCELAND.md']);
	const looked: string[] = [];
	const found = findInstructions(relative => {
		looked.push(relative);
		return relative === '.agenceland/AGENCELAND.md' || relative === '.claude/AGENCELAND.md';
	}, files('AGENCELAND.md'));
	assert.deepEqual(found, { scope: 'project', file: '.agenceland/AGENCELAND.md' });
	assert.deepEqual(looked, ['AGENCELAND.md', '.agenceland/AGENCELAND.md']);
	assert.deepEqual(findInstructions(files('.claude/AGENCELAND.md', 'AGENCELAND.md'), none), { scope: 'project', file: 'AGENCELAND.md' });
	assert.deepEqual(findInstructions(files('.claude/AGENCELAND.md'), none), { scope: 'project', file: '.claude/AGENCELAND.md' });
});

test('AGENTS.md comes after the project places, the user file after AGENTS.md', () => {
	const user = files('AGENCELAND.md');
	assert.deepEqual(findInstructions(files('AGENTS.md', '.agents/AGENCELAND.md'), user), { scope: 'project', file: '.agents/AGENCELAND.md' });
	assert.deepEqual(findInstructions(files('AGENTS.md'), user), { scope: 'project', file: 'AGENTS.md' });
	assert.deepEqual(findInstructions(none, user), { scope: 'user', file: 'AGENCELAND.md' });
	assert.equal(findInstructions(none, none), undefined);
});

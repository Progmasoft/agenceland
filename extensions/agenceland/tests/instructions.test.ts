// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { findProjectInstructions, projectInstructionFiles } from '../src/core/instructions.ts';

const existing = (...files: string[]) => (relative: string) => files.includes(relative);

test('the first place of the search order wins', () => {
	assert.deepEqual(projectInstructionFiles, ['AGENCELAND.md', '.agenceland/AGENCELAND.md', '.agents/AGENCELAND.md', '.claude/AGENCELAND.md']);
	assert.equal(findProjectInstructions(existing('.claude/AGENCELAND.md', 'AGENCELAND.md')), 'AGENCELAND.md');
	assert.equal(findProjectInstructions(existing('.claude/AGENCELAND.md', '.agents/AGENCELAND.md')), '.agents/AGENCELAND.md');
	assert.equal(findProjectInstructions(existing('.claude/AGENCELAND.md')), '.claude/AGENCELAND.md');
});

test('AGENTS.md is read only when no AGENCELAND.md exists', () => {
	assert.equal(findProjectInstructions(existing('AGENTS.md')), 'AGENTS.md');
	assert.equal(findProjectInstructions(existing('AGENTS.md', '.agenceland/AGENCELAND.md')), '.agenceland/AGENCELAND.md');
	assert.equal(findProjectInstructions(existing()), undefined);
});

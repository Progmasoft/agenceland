// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

import assert from 'node:assert/strict';
import { test } from 'node:test';
import { agencelandHome, mainWorktreeFromList, maxReadableKeyLength, projectDirectories, projectKey, userDirectory } from '../src/core/paths.ts';

test('the home follows each platform', () => {
	assert.equal(agencelandHome('linux', {}, '/home/ada'), '/home/ada/.config/Agenceland');
	assert.equal(agencelandHome('darwin', {}, '/Users/ada'), '/Users/ada/.config/Agenceland');
	assert.equal(agencelandHome('linux', { XDG_CONFIG_HOME: '/data/config' }, '/home/ada'), '/data/config/Agenceland');
	assert.equal(agencelandHome('linux', { XDG_CONFIG_HOME: 'relative' }, '/home/ada'), '/home/ada/.config/Agenceland');
	assert.equal(agencelandHome('win32', { APPDATA: 'C:\\Users\\Ada\\AppData\\Roaming' }, 'C:\\Users\\Ada'), 'C:\\Users\\Ada\\AppData\\Roaming\\Agenceland');
	assert.equal(agencelandHome('win32', {}, 'C:\\Users\\Ada'), 'C:\\Users\\Ada\\AppData\\Roaming\\Agenceland');
	assert.equal(agencelandHome('win32', { APPDATA: '' }, 'C:\\Users\\Ada'), 'C:\\Users\\Ada\\AppData\\Roaming\\Agenceland');
	assert.equal(userDirectory('win32', 'C:\\A\\Agenceland'), 'C:\\A\\Agenceland\\User');
	assert.equal(userDirectory('linux', '/h/.config/Agenceland'), '/h/.config/Agenceland/User');
});

test('a project key reads like the path and carries its hash', () => {
	// The examples of the product: -- before the project's folder on Linux
	// and macOS, one - for every separator on Windows.
	const linux = projectKey('linux', '/home/ada/Projects/example');
	assert.match(linux, /^-home-ada-Projects--example-[\da-f]{16}$/);
	assert.match(projectKey('darwin', '/Users/ada/Projects/example'), /^-Users-ada-Projects--example-[\da-f]{16}$/);
	const windows = projectKey('win32', 'C:\\Users\\Ada\\source\\repos\\example');
	assert.match(windows, /^C--Users-Ada-source-repos-example-[\da-f]{16}$/);
	assert.match(projectKey('linux', '/example'), /^-example-[\da-f]{16}$/);
	// Same readable part, different paths: the hash tells them apart.
	const a = projectKey('linux', '/a/b-c/d');
	const b = projectKey('linux', '/a-b/c/d');
	assert.equal(a.slice(0, -17), b.slice(0, -17));
	assert.notEqual(a, b);
	assert.equal(projectKey('linux', '/home/ada/Projects/example'), linux, 'the key is stable');
});

test('a long path keeps the end of its readable part and stays one valid name', () => {
	const deep = `/home/ada/${Array.from({ length: 40 }, (_, i) => `directory${i}`).join('/')}/example`;
	const key = projectKey('linux', deep);
	assert.ok(Buffer.byteLength(key) < 255, `${key.length} bytes`);
	assert.equal(key.length, maxReadableKeyLength + 17);
	assert.match(key, /-directory39--example-[\da-f]{16}$/);
	const other = projectKey('linux', deep.replace('directory0', 'directoryX'));
	assert.notEqual(key, other, 'paths that differ before the cut still differ');
});

test('project directories sit below Projects/<key>', () => {
	const directories = projectDirectories('linux', '/h/.config/Agenceland', '/src/app');
	assert.equal(directories.root, `/h/.config/Agenceland/Projects/${projectKey('linux', '/src/app')}`);
	assert.equal(directories.memory, `${directories.root}/memory`);
	assert.equal(directories.sessions, `${directories.root}/sessions`);
	const windows = projectDirectories('win32', 'C:\\A\\Agenceland', 'C:\\src\\app');
	assert.equal(windows.memory, `C:\\A\\Agenceland\\Projects\\${projectKey('win32', 'C:\\src\\app')}\\memory`);
});

test('the main worktree is the first entry of git worktree list', () => {
	const list = [
		'worktree /src/app',
		'HEAD 1a2b3c',
		'branch refs/heads/main',
		'',
		'worktree /home/ada/.config/Agenceland/Projects/x/worktrees/s1',
		'HEAD 4d5e6f',
		'branch refs/heads/agenceland/s1',
		'',
	].join('\n');
	assert.equal(mainWorktreeFromList(list), '/src/app');
	// A submodule or a separate Git directory lists its checkout; a bare
	// repository lists itself, marked bare.
	assert.equal(mainWorktreeFromList('worktree /super/libs/sub\nHEAD 1\ndetached\n'), '/super/libs/sub');
	assert.equal(mainWorktreeFromList('worktree /srv/git/app.git\nbare\n'), '/srv/git/app.git');
	assert.equal(mainWorktreeFromList('worktree C:/src/app\r\nHEAD 1\r\n'), 'C:/src/app');
	assert.equal(mainWorktreeFromList(''), undefined);
	assert.equal(mainWorktreeFromList('fatal: not a git repository\n'), undefined);
});

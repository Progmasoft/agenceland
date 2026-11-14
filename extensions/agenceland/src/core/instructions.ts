// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

// Which instruction file an agent reads: one file, the first that exists in
// a fixed order. The places after it are not looked at.

/** The places of the project's own instruction file, in search order. */
export const projectInstructionFiles: readonly string[] = [
	'AGENCELAND.md',
	'.agenceland/AGENCELAND.md',
	'.agents/AGENCELAND.md',
	'.claude/AGENCELAND.md',
];

/**
 * Next after {@link projectInstructionFiles}: the file many other agents
 * read, so a project written for them works unchanged.
 */
export const fallbackInstructionFile = 'AGENTS.md';

/**
 * Last, when the project has none of its own: the instruction file in the
 * user directory, which applies to every project of the user.
 */
export const userInstructionFile = 'AGENCELAND.md';

/** The instruction file an agent reads, and whose it is. */
export interface InstructionFile {
	/** `project` for a file of the project, `user` for the user's. */
	readonly scope: 'project' | 'user';
	/** Relative to the project, or to the user directory for `user`. */
	readonly file: string;
}

/**
 * The instruction file an agent reads, or `undefined` when there is none:
 * the first of {@link projectInstructionFiles}, then
 * {@link fallbackInstructionFile}, then the user's {@link userInstructionFile}.
 * `inProject` and `inUserDirectory` tell whether a relative path is a file.
 */
export function findInstructions(
	inProject: (relativePath: string) => boolean,
	inUserDirectory: (relativePath: string) => boolean,
): InstructionFile | undefined {
	for (const file of [...projectInstructionFiles, fallbackInstructionFile]) {
		if (inProject(file)) {
			return { scope: 'project', file };
		}
	}
	return inUserDirectory(userInstructionFile) ? { scope: 'user', file: userInstructionFile } : undefined;
}

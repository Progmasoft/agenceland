// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

// Which instruction files an agent reads, and in which order.

/**
 * The places of a project's instruction file, searched in this order; the
 * first that exists is the project's file.
 */
export const projectInstructionFiles: readonly string[] = [
	'AGENCELAND.md',
	'.agenceland/AGENCELAND.md',
	'.agents/AGENCELAND.md',
	'.claude/AGENCELAND.md',
];

/**
 * Read when none of {@link projectInstructionFiles} exists: the file many
 * other agents read, so a project written for them works unchanged.
 */
export const fallbackInstructionFile = 'AGENTS.md';

/** The instruction file of the user, in the user directory. */
export const userInstructionFile = 'AGENCELAND.md';

/**
 * The project's instruction file, relative to the project, or `undefined`
 * when there is none; `exists` tells whether a relative path is a file.
 */
export function findProjectInstructions(exists: (relativePath: string) => boolean): string | undefined {
	for (const candidate of projectInstructionFiles) {
		if (exists(candidate)) {
			return candidate;
		}
	}
	return exists(fallbackInstructionFile) ? fallbackInstructionFile : undefined;
}

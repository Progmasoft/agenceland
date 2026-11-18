<!-- SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com> -->
<!-- SPDX-License-Identifier: MIT -->

# Architecture

Agenceland is Code - OSS with changes. The repository holds the changes, not Code - OSS:

1. `scripts/prepare.sh` fetches the Code - OSS release named in `upstream.json` into `./vscode`, applies
   `patches/` in name order, removes the paths in `removed-paths.txt`, and merges `product.json` into Code - OSS's
   own through `scripts/merge-product.mjs`.
2. `extensions/` is copied in beside the built-in extensions of Code - OSS.
3. `scripts/build.sh <platform>` builds the prepared sources for one platform.

## The `agenceland` extension

The core extension, always on and hidden from the extension list:

| File | What it does |
| --- | --- |
| `src/extension.ts` | Activation and the commands |
| `src/core/instructions.ts` | The instruction files of agents |
| `src/core/paths.ts` | Where Agenceland keeps its files on each platform |
| `src/core/settings.ts` | Its settings |

It is bundled with esbuild (`esbuild.mts`); its tests are in `extensions/agenceland/tests/`.

## The agent core, `core/`

`agenceland-core` is a process of its own. Agenceland and its IntelliJ IDEA plugin start it and speak JSON-RPC 2.0 to
it on its standard input and output, in the frames of the Language Server Protocol (`Content-Length`, an empty line,
the JSON), so the logic of sessions is written once.

| Module | What it does |
| --- | --- |
| `protocol.rs` | The frames, requests, responses and error codes. A message over 16 MiB is refused before it is read. |
| `worktree.rs` | Each session starts the branch `agenceland/<session>` from the commit the repository has checked out, in a worktree of its own below `Projects/<key>/worktrees/`. Closing removes the worktree and keeps the branch; a worktree with changes that are not committed is kept unless the close says `discard`. |
| `permissions.rs` | Always Ask, Auto and Bypass Permissions, and what each decides for reading and writing in or outside the worktree, a command, the network and a push. |
| `files.rs` | The file tools. A path stays inside the worktree: `..`, absolute paths, symbolic links that lead out and the worktree's `.git` (its hooks and config would run outside the sandbox) are refused. A write to a file that changed since the agent last read it, or that it never read, is refused, so the agent reads the user's edit instead of overwriting it. Files are replaced through a temporary file beside them. |
| `record.rs` | The session record, `Projects/<key>/sessions/<session>.jsonl`: one event per line, numbered from 1, written out before the core answers. Replay reads it back and refuses a record with a gap or a line out of order. |
| `server.rs` | The methods: `initialize`, `session/create`, `permission/check`, `file/read`, `file/write` (with `approved` when the mode asks), `session/close`, `session/replay`, `shutdown`. Session ids are lower-case letters and digits, so one never names a path elsewhere. |

The front end computes `Projects/<key>` (`extensions/agenceland/src/core/paths.ts`) and passes it as `dataDirectory`.

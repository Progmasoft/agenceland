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

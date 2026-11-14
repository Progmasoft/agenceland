#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
# SPDX-License-Identifier: MIT
#
# Builds the prepared ./vscode for one platform into ./VSCode-<platform>.
# Usage: scripts/build.sh <platform>, such as linux-x64, darwin-arm64,
# darwin-x64, win32-x64. Needs the Node.js version of upstream.json and
# about 8 GB of memory. Code - OSS installs with the pnpm its package.json
# names in packageManager; Corepack, which comes with Node.js 24, runs that
# version and checks its hash when the field carries one. Nothing is
# installed globally.
set -euo pipefail

platform=${1:?usage: scripts/build.sh <platform>}
root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root/vscode"

want=$(node -e 'process.stdout.write(require(process.argv[1]).node)' "$root/upstream.json")
have=$(node --version)
if [ "$have" != "v$want" ]; then
  echo "error: Node.js $have; upstream.json needs v$want" >&2
  exit 1
fi
export COREPACK_ENABLE_DOWNLOAD_PROMPT=0
run_pnpm() {
  corepack pnpm "$@"
}
echo "pnpm $(run_pnpm --version), as vscode/package.json names it"

# What upstream's own build skips: Electron comes with the packaging step,
# and the browsers and model runtimes are only for tests and features that
# Agenceland does not build.
export ELECTRON_SKIP_BINARY_DOWNLOAD=1
export PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1
export ONNXRUNTIME_NODE_INSTALL=skip
export FOUNDRY_LOCAL_SKIP_INSTALL=1

# preinstall patches the V8 headers the native modules build against.
run_pnpm run preinstall
for attempt in 1 2 3; do
  if run_pnpm install --frozen-lockfile; then
    break
  fi
  if [ "$attempt" = 3 ]; then
    echo "error: pnpm install failed three times" >&2
    exit 1
  fi
  echo "pnpm install failed ($attempt); trying again" >&2
done
run_pnpm run gulp "vscode-$platform-min"
echo "built $root/VSCode-$platform"

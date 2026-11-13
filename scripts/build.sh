#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
# SPDX-License-Identifier: MIT
#
# Builds the prepared ./vscode for one platform into ./VSCode-<platform>.
# Usage: scripts/build.sh <platform>, such as linux-x64, darwin-arm64,
# darwin-x64, win32-x64. Needs the Node.js version of upstream.json and
# about 8 GB of memory.
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

export NODE_OPTIONS=--max-old-space-size=8192
npm ci
npm run gulp "vscode-$platform-min"
echo "built $root/VSCode-$platform"

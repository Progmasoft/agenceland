#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
# SPDX-License-Identifier: MIT
#
# Fetches the Code - OSS commit named in upstream.json into ./vscode, checks
# that it is that commit, applies patches/ and merges product.json.
# Usage: scripts/prepare.sh
set -euo pipefail

root=$(cd "$(dirname "$0")/.." && pwd)
cd "$root"

field() {
  node -e 'const u = require(process.argv[1]); process.stdout.write(u[process.argv[2]]);' \
    "$root/upstream.json" "$1"
}
repository=$(field repository)
tag=$(field tag)
commit=$(field commit)

if [ -e vscode ]; then
  echo "error: $root/vscode exists; remove it to prepare again" >&2
  exit 1
fi
git init -q vscode
git -C vscode remote add origin "$repository"
git -C vscode fetch -q --depth=1 origin "refs/tags/$tag:refs/tags/$tag"
actual=$(git -C vscode rev-parse "refs/tags/$tag^{commit}")
if [ "$actual" != "$commit" ]; then
  echo "error: tag $tag is $actual, upstream.json says $commit" >&2
  exit 1
fi
git -C vscode -c advice.detachedHead=false checkout -q "$commit"

for patch in patches/*.patch; do
  git -C vscode apply --verbose "$root/$patch"
done
node scripts/merge-product.mjs vscode
echo "prepared Code - OSS $tag ($commit) in $root/vscode"

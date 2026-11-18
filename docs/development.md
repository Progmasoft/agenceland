<!-- SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com> -->
<!-- SPDX-License-Identifier: MIT -->

# Development

The tests and the type check need only Node.js at the version in `upstream.json`:

```sh
npm ci
npm run typecheck
npm test
```

Building the product needs the Code - OSS prerequisites as well:

```sh
scripts/prepare.sh
scripts/build.sh linux-x64     # or darwin-arm64, darwin-x64, win32-x64
```

The agent core needs Rust (`core/rust-toolchain.toml` names the version) and Git:

```sh
cd core
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo doc --no-deps --locked
```

`tests/product.test.mjs` checks the product fields; the extension's tests check its paths, settings and
instructions. The core's unit tests check each module; `core/tests/session.rs` starts the `agenceland-core` process
and takes a session through a real Git repository, from its worktree to replay. CI runs the type check and the tests
and collects coverage, and runs the core's checks on Linux, macOS and Windows.

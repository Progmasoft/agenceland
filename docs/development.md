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

`tests/product.test.mjs` checks the product fields; the extension's tests check its paths, settings and
instructions. CI runs the type check and the tests and collects coverage.

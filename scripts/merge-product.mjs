// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

// Writes the product.json of Agenceland into a Code - OSS checkout: the
// fields of ./product.json replace those of the checkout, the rest is kept.
// Usage: node scripts/merge-product.mjs <checkout>

import { readFileSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { mergeProduct } from "./product.mjs";

const checkout = process.argv[2];
if (!checkout) {
  console.error("usage: node scripts/merge-product.mjs <checkout>");
  process.exit(2);
}
// The checkout is the directory scripts/prepare.sh made; its product.json is
// the one file this script writes.
const target = resolve(checkout, "product.json");
// eslint-disable-next-line security/detect-non-literal-fs-filename -- the checkout named on the command line
const upstream = JSON.parse(readFileSync(target, "utf8"));
const ours = JSON.parse(
  readFileSync(new URL("../product.json", import.meta.url), "utf8"),
);
const merged = JSON.stringify(mergeProduct(upstream, ours), null, "\t");
// eslint-disable-next-line security/detect-non-literal-fs-filename -- the checkout named on the command line
writeFileSync(target, `${merged}\n`);
console.log(`merged ${Object.keys(ours).length} fields into ${target}`);

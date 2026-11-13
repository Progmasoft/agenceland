// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

// Writes the product.json of Agenceland into a Code - OSS checkout: the
// fields of ./product.json replace those of the checkout, the rest is kept.
// Usage: node scripts/merge-product.mjs <checkout>

import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const checkout = process.argv[2];
if (!checkout) {
  console.error("usage: node scripts/merge-product.mjs <checkout>");
  process.exit(2);
}
const target = join(checkout, "product.json");
const upstream = JSON.parse(readFileSync(target, "utf8"));
const ours = JSON.parse(
  readFileSync(new URL("../product.json", import.meta.url), "utf8"),
);
writeFileSync(target, JSON.stringify({ ...upstream, ...ours }, null, "\t") + "\n");
console.log(`merged ${Object.keys(ours).length} fields into ${target}`);

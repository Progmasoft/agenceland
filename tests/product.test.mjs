// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { mergeProduct } from "../scripts/product.mjs";

const ours = JSON.parse(
  readFileSync(new URL("../product.json", import.meta.url), "utf8"),
);

test("our fields replace upstream's and the rest is kept", () => {
  const upstream = {
    nameShort: "Code - OSS",
    builtInExtensions: [{ name: "ms-vscode.js-debug" }],
    extensionsGallery: { serviceUrl: "https://example.invalid" },
  };
  const merged = mergeProduct(upstream, ours);
  assert.equal(merged.nameShort, "Agenceland");
  assert.deepEqual(merged.builtInExtensions, upstream.builtInExtensions);
  assert.equal(
    merged.extensionsGallery.serviceUrl,
    "https://open-vsx.org/vscode/gallery",
  );
  assert.equal(upstream.nameShort, "Code - OSS", "upstream is not changed");
});

test("anything but a JSON object is refused", () => {
  for (const wrong of [null, [], "text", 1]) {
    assert.throws(() => mergeProduct(wrong, ours), TypeError);
    assert.throws(() => mergeProduct({}, wrong), TypeError);
  }
});

test("product.json names Agenceland and Open VSX", () => {
  assert.equal(ours.nameShort, "Agenceland");
  assert.equal(ours.applicationName, "agenceland");
  assert.equal(ours.licenseName, "MIT");
  assert.match(ours.extensionsGallery.serviceUrl, /^https:\/\/open-vsx\.org\//);
  const appIds = [
    ours.win32x64AppId,
    ours.win32arm64AppId,
    ours.win32x64UserAppId,
    ours.win32arm64UserAppId,
  ];
  assert.equal(
    new Set(appIds).size,
    appIds.length,
    "every Windows AppId differs",
  );
  for (const id of appIds) {
    assert.match(
      id,
      /^\{\{[\dA-F]{8}-[\dA-F]{4}-[\dA-F]{4}-[\dA-F]{4}-[\dA-F]{12}\}$/,
    );
  }
});

test("Copilot keeps no rights and no updates", () => {
  const trusted = JSON.stringify(ours.trustedExtensionAuthAccess);
  assert.doesNotMatch(trusted, /copilot/i);
  assert.deepEqual(ours.builtInExtensionsEnabledWithAutoUpdates, []);
});

test("removed-paths.txt names the Copilot extension and only relative paths", () => {
  const paths = readFileSync(
    new URL("../removed-paths.txt", import.meta.url),
    "utf8",
  )
    .split("\n")
    .map((line) => line.replace(/#.*/, "").trim())
    .filter((line) => line.length > 0);
  assert.ok(paths.includes("extensions/copilot"));
  for (const path of paths) {
    assert.doesNotMatch(path, /^\/|\.\./, `${path} stays inside the checkout`);
  }
});

// SPDX-FileCopyrightText: 2026 Progmasoft <support@progmasoft.com>
// SPDX-License-Identifier: MIT

// How Agenceland's product.json goes over the one of Code - OSS.

/**
 * The product.json of Agenceland: the fields of `ours` replace those of
 * `upstream`; every other field of `upstream` is kept as it is.
 * @param {Record<string, unknown>} upstream
 * @param {Record<string, unknown>} ours
 * @returns {Record<string, unknown>}
 */
export function mergeProduct(upstream, ours) {
  for (const value of [upstream, ours]) {
    if (value === null || typeof value !== "object" || Array.isArray(value)) {
      throw new TypeError("a product.json is a JSON object");
    }
  }
  return { ...upstream, ...ours };
}

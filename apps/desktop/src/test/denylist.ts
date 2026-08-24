/**
 * The denylist `xtask denylist-audit` runs over `crates/`, pointed at what a
 * screen actually rendered.
 *
 * The crates check their own strings and `contract.test.ts` checks the shell's
 * source, but neither of those sees the page a person ends up reading: a
 * fixture with a forbidden word in it, or a component that composed a sentence
 * out of two harmless halves, only shows up after a render. `Graph.test.tsx`
 * did this inline first; four more routes want the same check, so it lives
 * here.
 */

import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const REPO = join(dirname(fileURLToPath(import.meta.url)), "..", "..", "..", "..");

/** The banned terms, read from the same file `xtask` reads. */
export function diagnosticTerms(): string[] {
  return readFileSync(join(REPO, "fixtures", "denylist", "diagnostic_terms.txt"), "utf8")
    .split("\n")
    .map((line) => line.trim())
    .filter((line) => line !== "" && !line.startsWith("#"))
    .map((line) => line.toLowerCase());
}

/**
 * Which banned terms appear in this text.
 *
 * An ASCII term is matched on a word boundary so that a word merely containing
 * one is not a hit; a CJK term has no boundary to speak of and is matched as a
 * substring, which is the same rule `xtask` applies.
 */
export function denylistHits(text: string, terms: readonly string[] = diagnosticTerms()): string[] {
  const lowered = text.toLowerCase();
  return terms.filter((term) => {
    const ascii = /^[\x20-\x7e]+$/.test(term);
    return ascii
      ? new RegExp(
          `(^|[^a-z0-9_])${term.replace(/[.*+?^${}()|[\]\\-]/g, "\\$&")}($|[^a-z0-9_])`,
        ).test(lowered)
      : lowered.includes(term);
  });
}

/** What the whole document says right now, for a denylist check on a render. */
export function renderedText(): string {
  return document.body.textContent ?? "";
}

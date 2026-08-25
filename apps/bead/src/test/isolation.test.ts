import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

// jsdom serves modules over a non-file URL, so the scan roots at the package
// directory (vitest runs with cwd = apps/bead) rather than import.meta.url.
const SRC = join(process.cwd(), "src");

function sourceFiles(dir: string): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) return sourceFiles(path);
    return /\.(ts|tsx)$/.test(entry.name) ? [path] : [];
  });
}

// WP-B02 says: do not reference @soul/desktop or any soul-* crate. A grep test
// is cheap and fails loudly the first time someone reaches across the fence.
describe("与 Soul 的隔离", () => {
  it("源码里没有 @soul/*、Tauri 或 soul-* crate 的引用", () => {
    const offenders = sourceFiles(SRC)
      .filter((path) => !path.endsWith("isolation.test.ts"))
      .filter((path) => /@soul\/|@tauri-apps|soul-core|soulcore|crates\/soul-/.test(readFileSync(path, "utf8")));
    expect(offenders).toEqual([]);
  });
});

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

/** Every source file except this one, whose job is to spell the patterns out. */
function scannedFiles(): string[] {
  return sourceFiles(SRC).filter((path) => !path.endsWith("isolation.test.ts"));
}

// WP-B02 says: do not reference @soul/desktop or any soul-* crate. A grep test
// is cheap and fails loudly the first time someone reaches across the fence.
describe("与 Soul 的隔离", () => {
  it("源码里没有 @soul/*、Tauri 或 soul-* crate 的引用", () => {
    const offenders = scannedFiles().filter((path) =>
      /@soul\/|@tauri-apps|soul-core|soulcore|crates\/soul-/.test(readFileSync(path, "utf8")),
    );
    expect(offenders).toEqual([]);
  });
});

/**
 * NE-1: a pattern never leaves the machine, so the shell must not contain a
 * call site that could send one. `new URL` is on the list only when the literal
 * carries a remote scheme — react-router builds path URLs the same way and is
 * not egress.
 */
const EGRESS_PATTERNS: ReadonlyArray<readonly [string, RegExp]> = [
  ["fetch(", /(^|[^.\w$])fetch\s*\(/],
  ["XMLHttpRequest", /\bXMLHttpRequest\b/],
  ["WebSocket", /\bWebSocket\b/],
  ["sendBeacon", /\bsendBeacon\b/],
  ["new URL(<远程 scheme>)", /new\s+URL\s*\(\s*['"`]\s*[a-zA-Z][a-zA-Z0-9+.-]*:\/\//],
  ["远程 URL 字面量", /\b(https?|wss?|ftps?):\/\//i],
];

function egressHits(name: string, text: string): string[] {
  return EGRESS_PATTERNS.filter(([, pattern]) => pattern.test(text)).map(
    ([label]) => `${name}: ${label}`,
  );
}

describe("不出网（NE-1）", () => {
  it("源码里没有 fetch / XMLHttpRequest / WebSocket / sendBeacon / 远程 URL", () => {
    const offenders = scannedFiles().flatMap((path) =>
      egressHits(path.slice(SRC.length + 1), readFileSync(path, "utf8")),
    );
    expect(offenders).toEqual([]);
  });

  // A scanner that matches nothing passes the case above for the wrong reason,
  // and a scanner that matches a route path fails it for the wrong reason.
  it("扫描器认得出网写法，也放过 react-router 的路径 URL", () => {
    const scheme = `${"http"}${"s://"}`;
    const planted = [
      `await fetch("${scheme}example.invalid/x");`,
      "new XMLHttpRequest();",
      'new WebSocket("wss" + "://x");',
      "navigator.sendBeacon(payload);",
      `new URL("${scheme}example.invalid/x");`,
    ];
    for (const line of planted) {
      expect(egressHits("planted", line)).not.toEqual([]);
    }

    const innocent = [
      'const url = new URL(location.pathname, location.origin);',
      'navigate("/assemble/proj-1");',
      '<Link to="/pattern/gal-slime-01">图纸</Link>',
      "const next = prefetchPattern(id);",
    ];
    for (const line of innocent) {
      expect(egressHits("innocent", line)).toEqual([]);
    }
  });
});

/**
 * The rules the shell has to keep that no single component can be asked to
 * keep on its own.
 *
 * Three of them are structural — where IPC may be called from, what vocabulary
 * may appear on screen, and whether the TypeScript view of the core still
 * matches the Rust one. All three are the kind of thing that decays quietly,
 * so they are read off the files rather than agreed in review.
 */

import { readFileSync, readdirSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, it } from "vitest";

import { COMMANDS } from "./core";
import { CLOUD_LABEL } from "./test/fakeCore";

const SRC = dirname(fileURLToPath(import.meta.url));
const DESKTOP = join(SRC, "..");
const REPO = join(DESKTOP, "..", "..");
const SHELL_RS = join(REPO, "crates", "soulcore", "src", "commands", "shell.rs");
const TAURI_COMMANDS_RS = join(DESKTOP, "src-tauri", "src", "commands.rs");

function sourceFiles(root: string, extensions: readonly string[]): string[] {
  const found: string[] = [];
  for (const entry of readdirSync(root, { withFileTypes: true })) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) {
      found.push(...sourceFiles(path, extensions));
    } else if (extensions.some((extension) => entry.name.endsWith(extension))) {
      found.push(path);
    }
  }
  return found;
}

const isTestSupport = (path: string): boolean =>
  path.includes(`${join("src", "test")}`) || /\.test\.tsx?$/.test(path);

describe("壳与核心的边界", () => {
  /**
   * The shell holds no business logic, and the way that stays true is that
   * only one module can call the core at all. `eslint.config.js` says the same
   * thing; this says it again in a form that does not depend on anyone running
   * the linter.
   */
  it("只有 core.ts 直接引用 Tauri 的 API", () => {
    const offenders = sourceFiles(SRC, [".ts", ".tsx"])
      .filter((path) => path !== join(SRC, "core.ts") && !isTestSupport(path))
      .filter((path) => readFileSync(path, "utf8").includes("@tauri-apps/api"))
      .map((path) => relative(DESKTOP, path));

    expect(offenders).toEqual([]);
  });

  it("界面用的命令名和 src-tauri 注册的一模一样", () => {
    const registered = readFileSync(TAURI_COMMANDS_RS, "utf8");
    const declared = [...registered.matchAll(/#\[tauri::command\]\s*pub fn (\w+)\s*\(/g)].map(
      (match) => match[1],
    );
    expect(declared.sort()).toEqual([...Object.values(COMMANDS)].sort());
  });

  /**
   * 尚未启用 is a promise about the build, and it is asserted on both sides of
   * the IPC. If someone rewords one of them, this fails rather than letting
   * the shell and the core disagree about what the user was told.
   */
  it("云端开关的文案和 soulcore 里的常量是同一句话", () => {
    const rust = readFileSync(SHELL_RS, "utf8");
    const match = /CLOUD_NOT_YET_AVAILABLE_LABEL: &str = "([^"]+)"/.exec(rust);
    expect(match?.[1]).toBe(CLOUD_LABEL);
  });
});

describe("界面用词", () => {
  /**
   * `xtask denylist-audit` reads the same file but only scans `crates/`. The
   * words it bans are about what the product says to the user, and the place
   * the product says things to the user is here, so the UI tree checks itself
   * against the same list.
   */
  it("不出现诊断词与量表词", () => {
    const terms = readFileSync(join(REPO, "fixtures", "denylist", "diagnostic_terms.txt"), "utf8")
      .split("\n")
      .map((line) => line.trim())
      .filter((line) => line !== "" && !line.startsWith("#"))
      .map((line) => line.toLowerCase());
    expect(terms.length).toBeGreaterThan(50);

    const hits: string[] = [];
    for (const path of sourceFiles(SRC, [".ts", ".tsx", ".css"])) {
      if (isTestSupport(path)) continue;
      const text = readFileSync(path, "utf8").toLowerCase();
      for (const term of terms) {
        const isAscii = /^[\x20-\x7e]+$/.test(term);
        const pattern = isAscii
          ? new RegExp(`(^|[^a-z0-9_])${term.replace(/[.*+?^${}()|[\]\\-]/g, "\\$&")}($|[^a-z0-9_])`)
          : null;
        const found = pattern === null ? text.includes(term) : pattern.test(text);
        if (found) hits.push(`${relative(DESKTOP, path)}: ${term}`);
      }
    }

    expect(hits).toEqual([]);
  });
});

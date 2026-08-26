import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

/**
 * BD15/BD16 in linter form for this subtree. The naming rule is not cosmetic:
 * the Rust oracle and this port have to agree on the public vocabulary, and
 * `confidence` is the pinned word.
 */
const ALGO = join(process.cwd(), "src", "algo");

function algoFiles(dir: string = ALGO): string[] {
  return readdirSync(dir, { withFileTypes: true }).flatMap((item) => {
    const path = join(dir, item.name);
    if (item.isDirectory()) return algoFiles(path);
    return /\.(ts|md|json)$/.test(item.name) ? [path] : [];
  });
}

function offenders(pattern: RegExp): string[] {
  return algoFiles()
    .filter((path) => !path.endsWith("constraints.test.ts"))
    .filter((path) => pattern.test(readFileSync(path, "utf8")));
}

describe("BD15 置信度只叫 confidence", () => {
  it("算法树里不出现 score / 得分 / 分数 / 评分", () => {
    expect(offenders(/\bscore\b|得分|分数|评分/i)).toEqual([]);
  });

  it("判定器确实以 confidence 命名", () => {
    const source = readFileSync(join(ALGO, "classify.ts"), "utf8");
    expect(source).toContain("confidence");
  });
});

describe("BD16 不写外网 URL 与 $schema", () => {
  it("算法树里没有 http/https 字面量", () => {
    expect(offenders(/https?:\/\//)).toEqual([]);
  });

  it("fixture 里没有 $schema 键", () => {
    expect(offenders(/\$schema/)).toEqual([]);
  });
});

describe("BD14 落点", () => {
  it("算法只落在 src/algo，没有 packages/bead-algo", () => {
    const packages = readdirSync(process.cwd(), { withFileTypes: true }).map((it) => it.name);
    expect(packages).not.toContain("packages");
    expect(algoFiles().length).toBeGreaterThan(0);
  });
});

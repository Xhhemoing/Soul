import { describe, expect, it } from "vitest";

import { CREATORS, PATTERNS, TAGS } from "./catalog.ts";
import { GRIDDED_PATTERN_IDS, fixtureGridFor } from "./grids.ts";
import { occupiedCount } from "../algo/grid.ts";
import { asPatternId } from "../stores/ids.ts";

/**
 * T-GAL-9 / T-GAL-10 / T-GAL-11: the shape of the WP-B08 expansion, checked
 * from outside the fixture. `grids.test.ts` already proves each grid agrees
 * with its catalog row; this file proves the *set* is the one D-GAL-2 asked
 * for and that nothing that shipped before it moved.
 */

/** The B02 catalog, verbatim. Append-only means these keep their ids and order. */
const ORIGINAL_IDS = [
  "gal-slime-01",
  "gal-torii-02",
  "gal-cakebox-03",
  "gal-lantern-04",
  "gal-arcade-05",
] as const;

const NEW_IDS = ["gal-gift-06", "gal-mochi-07", "gal-mush-08", "gal-quilt-09"] as const;

describe("T-GAL-9 扩容形状（D-GAL-2）", () => {
  it("既有 5 张图纸原样在场，且仍在数组最前——扩容是 append-only", () => {
    expect(PATTERNS.slice(0, ORIGINAL_IDS.length).map((pattern) => pattern.id)).toEqual([
      ...ORIGINAL_IDS,
    ]);
    // R-IE-4: 旧归档的 sourcePatternId 要能继续命中 catalog，所以 id 只增不改。
    expect(PATTERNS).toHaveLength(ORIGINAL_IDS.length + NEW_IDS.length);
    expect(PATTERNS.slice(ORIGINAL_IDS.length).map((pattern) => pattern.id)).toEqual([...NEW_IDS]);
  });

  it("既有 2 位创作者原样在场，本轮只新增 1 位", () => {
    expect(CREATORS.map((creator) => creator.id)).toEqual(["cr-mira", "cr-tonoya", "cr-nagi"]);
  });

  it("每个新增图纸都带网格；gal-cakebox-03 仍然没有（D-GAL-3）", () => {
    for (const id of NEW_IDS) {
      expect(GRIDDED_PATTERN_IDS).toContain(asPatternId(id));
    }
    expect(GRIDDED_PATTERN_IDS).not.toContain(asPatternId("gal-cakebox-03"));
    expect(fixtureGridFor(asPatternId("gal-cakebox-03"))).toBeNull();
  });

  it("四个标签各至少 2 张图纸，标签集不扩", () => {
    expect(TAGS).toEqual(["二次元", "像素游戏", "立体拼豆", "节日限定"]);
    const thin = TAGS.filter(
      (tag) => PATTERNS.filter((pattern) => pattern.tags.includes(tag)).length < 2,
    );
    expect(thin).toEqual([]);
    // 反过来：没有图纸挂着标签集之外的标签。
    for (const pattern of PATTERNS) {
      expect(pattern.tags.length).toBeGreaterThan(0);
      for (const tag of pattern.tags) expect(TAGS).toContain(tag);
    }
  });

  it("板型只有 square-28 / square-56，色板 ≤10 条且创作者可解析", () => {
    const creatorIds = new Set(CREATORS.map((creator) => creator.id));
    for (const pattern of PATTERNS) {
      expect(["square-28", "square-56"]).toContain(pattern.board);
      // `grids.ts` 用 `0`–`9` 编码格子，十条就是硬上限。
      expect(pattern.palette.length).toBeGreaterThan(0);
      expect(pattern.palette.length).toBeLessThanOrEqual(10);
      expect(creatorIds.has(pattern.creatorId)).toBe(true);
      expect(pattern.difficulty).toBeGreaterThanOrEqual(1);
      expect(pattern.difficulty).toBeLessThanOrEqual(5);
      for (const entry of pattern.palette) {
        expect(entry.hex).toMatch(/^#[0-9a-f]{6}$/);
      }
    }
    expect(new Set(PATTERNS.map((pattern) => pattern.id)).size).toBe(PATTERNS.length);
  });
});

describe("T-GAL-10 鸟居补网格（DEV-GAL-3）", () => {
  it("gal-torii-02 现在有 56×56 网格，展示数字全部由它派生", () => {
    const pattern = PATTERNS.find((candidate) => candidate.id === asPatternId("gal-torii-02"))!;
    const fixture = fixtureGridFor(asPatternId("gal-torii-02"));
    expect(fixture).not.toBeNull();
    expect(fixture!.grid.width).toBe(56);
    expect(fixture!.grid.height).toBe(56);
    expect(occupiedCount(fixture!.grid)).toBe(pattern.beadCount);
    // 1540 是 B02 的手写估值；补网格后它必须已经让位给派生值。
    expect(pattern.beadCount).not.toBe(1_540);
  });
});

/**
 * T-GAL-11: `isolation.test.ts` already scans every `.ts`/`.tsx` under `src`
 * recursively, so the new fixture and page files joined NE-1's net simply by
 * existing. What that scan cannot see is a *field* that is URL-shaped without
 * carrying a scheme — an `avatar: "cdn/mira.png"` would pass the regex and
 * still be a hole in BD15 the first time someone resolves it.
 */
const URLISH_KEYS =
  /^(url|href|src|link|avatar|homepage|site|website|profile|handle|icon|image|cover|thumb|social)$/i;

describe("T-GAL-11 fixture 数据零外链（BD15 / BD7）", () => {
  it("Creator / Pattern 上没有 URL 形状的字段", () => {
    for (const record of [...CREATORS, ...PATTERNS]) {
      for (const key of Object.keys(record)) {
        expect({ key, urlish: URLISH_KEYS.test(key) }).toEqual({ key, urlish: false });
      }
    }
    expect(Object.keys(CREATORS[0]!).sort()).toEqual(["bio", "checkIns", "id", "name"]);
  });

  it("序列化后的 fixture 里没有 scheme、协议相对地址或裸域名", () => {
    const serialized = JSON.stringify({ CREATORS, PATTERNS, TAGS });
    for (const pattern of [/[a-z][a-z0-9+.-]*:\/\//i, /(^|[^:])\/\//, /www\./i, /\.(com|net|org|io)\b/i]) {
      expect(serialized).not.toMatch(pattern);
    }
  });
});

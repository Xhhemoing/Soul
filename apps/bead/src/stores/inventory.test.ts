import { describe, expect, it } from "vitest";

import { PATTERNS } from "../fixtures/catalog.ts";
import { asCreatorId, asPatternId, mintProjectId } from "./ids.ts";
import {
  buildPurchaseText,
  clampBeads,
  normalizeCode,
  normalizeHex,
  parseHexColor,
  selectRequirements,
  selectShortages,
  selectSubstituteGroups,
  type RequirementRow,
} from "./inventory.ts";
import type { InventoryEntry, Pattern, Project, ProjectStatus } from "./types.ts";

// Section 8 of `docs/bead/reviews/round2-inventory.md` is a test list; the pure
// half of that list is this file.

function project(patternId: string | null, status: ProjectStatus = "active"): Project {
  return {
    id: mintProjectId(),
    title: "测试项目",
    sourcePatternId: patternId === null ? null : asPatternId(patternId),
    status,
    createdAt: 1,
    backdrop: "black",
    backdropColor: "#101014",
  };
}

function pattern(id: string, palette: Pattern["palette"]): Pattern {
  return {
    id: asPatternId(id),
    title: "测试图纸",
    creatorId: asCreatorId("cr-mira"),
    tags: [],
    difficulty: 1,
    beadCount: palette.reduce((sum, entry) => sum + entry.beads, 0),
    estimatedMinutes: 10,
    board: "square-28",
    palette,
  };
}

function lookupOf(...patterns: Pattern[]): (id: string) => Pattern | undefined {
  return (id) => patterns.find((candidate) => candidate.id === id);
}

function requirement(code: string, required: number, hex = "#123456"): RequirementRow {
  return { code, name: `名-${code}`, hex, required };
}

function stock(code: string, hex: string, beads: number, name = `名-${code}`): InventoryEntry {
  return { code, name, hex, beads };
}

describe("T-INV-1 录入边界的两个归一函数", () => {
  it("normalizeCode 去空白并大写", () => {
    expect(normalizeCode("  r04 ")).toBe("R04");
    expect(normalizeCode("h02")).toBe("H02");
    expect(normalizeCode("")).toBe("");
  });

  it("parseHexColor 只收六位十六进制，`#` 可省", () => {
    expect(parseHexColor("#D8412F")).toEqual({ r: 0xd8, g: 0x41, b: 0x2f });
    expect(parseHexColor("d8412f")).toEqual({ r: 0xd8, g: 0x41, b: 0x2f });
    expect(parseHexColor("  #ffffff  ")).toEqual({ r: 255, g: 255, b: 255 });
  });

  it("三位简写、位数不对、非十六进制字符一律拒收", () => {
    for (const bad of ["#abc", "abc", "#abcd", "#abcdefg", "#gggggg", "红色", "", "#12345"]) {
      expect(parseHexColor(bad)).toBeNull();
      expect(normalizeHex(bad)).toBeNull();
    }
  });

  it("normalizeHex 归一为小写带 #", () => {
    expect(normalizeHex("D8412F")).toBe("#d8412f");
    expect(normalizeHex("#FFFFFF")).toBe("#ffffff");
  });

  it("clampBeads 夹在 0–99999 的整数上", () => {
    expect(clampBeads(-5)).toBe(0);
    expect(clampBeads(12.7)).toBe(12);
    expect(clampBeads(1_000_000)).toBe(99_999);
    expect(clampBeads(Number.NaN)).toBe(0);
  });
});

describe("T-INV-2 需求聚合（D-INV-1 / D-INV-2）", () => {
  it("在拼项目的来源图纸 palette 按码求和", () => {
    const rows = selectRequirements([project("gal-slime-01"), project("gal-torii-02", "todo")]);
    const byCode = new Map(rows.map((row) => [row.code, row.required]));
    expect(byCode.get("B05")).toBe(108 + 300);
    expect(byCode.get("H02")).toBe(126);
    expect(byCode.get("R04")).toBe(520);
    expect(rows.find((row) => row.code === "B05")).toMatchObject({
      name: "墨黑",
      hex: "#1b1b1f",
    });
  });

  it("同一图纸实例化两次，需求翻倍", () => {
    const rows = selectRequirements([project("gal-slime-01"), project("gal-slime-01", "todo")]);
    expect(rows.find((row) => row.code === "H02")?.required).toBe(252);
  });

  it("空白项目没有 BOM，draft / done 不计", () => {
    expect(selectRequirements([project(null)])).toEqual([]);
    expect(
      selectRequirements([project("gal-slime-01", "draft"), project("gal-torii-02", "done")]),
    ).toEqual([]);
  });

  it("来源图纸已下架时跳过而不是崩", () => {
    expect(selectRequirements([project("gal-not-here")])).toEqual([]);
  });
});

describe("T-INV-3 缺口算术与排序（D-INV-8 / D-INV-12）", () => {
  const requirements = [requirement("A01", 100), requirement("B02", 100), requirement("C03", 250)];

  it("库存全覆盖时一行都不出", () => {
    const shortages = selectShortages(requirements, [
      stock("A01", "#111111", 100),
      stock("B02", "#222222", 400),
      stock("C03", "#333333", 250),
    ]);
    expect(shortages).toEqual([]);
  });

  it("部分覆盖只出差值", () => {
    const shortages = selectShortages(requirements, [stock("C03", "#333333", 40)]);
    expect(shortages.find((row) => row.code === "C03")).toMatchObject({
      required: 250,
      inStock: 40,
      shortage: 210,
    });
  });

  it("空库存出全量缺口，库存视为 0", () => {
    const shortages = selectShortages(requirements, []);
    expect(shortages.map((row) => row.shortage)).toEqual([250, 100, 100]);
    expect(shortages.every((row) => row.inStock === 0)).toBe(true);
  });

  it("排序：缺颗数降序，同数按码升序", () => {
    expect(selectShortages(requirements, []).map((row) => row.code)).toEqual([
      "C03",
      "A01",
      "B02",
    ]);
  });
});

describe("T-INV-4 替代池（D-INV-9，rust check_stock 的余量制）", () => {
  // 与 `algo/substitutes.test.ts` 同一对冻结色：ΔE00 2.9952 / 3.0012 卡在阈值两侧。
  const WANTED = "#78828c";
  const JUST_BELOW_3 = "#808a95";
  const JUST_ABOVE_3 = "#80848d";

  const wantedPattern = pattern("gal-boundary-01", [
    { code: "Z01", name: "灰蓝", hex: WANTED, beads: 100 },
  ]);

  function groupsFor(inventory: InventoryEntry[], patterns: Pattern[] = [wantedPattern]) {
    const requirements = selectRequirements(
      [project(patterns[0]!.id)],
      lookupOf(...patterns),
    );
    const shortages = selectShortages(requirements, inventory);
    return selectSubstituteGroups(shortages, requirements, inventory);
  }

  it("ΔE00 恰好越过 3 的候选被拒，严格小于才收", () => {
    const groups = groupsFor([
      stock("A1", JUST_BELOW_3, 50, "偏下"),
      stock("A2", JUST_ABOVE_3, 50, "偏上"),
    ]);
    expect(groups).toHaveLength(1);
    expect(groups[0]!.candidates.map((it) => it.code)).toEqual(["A1"]);
    expect(groups[0]!.candidates[0]!.deltaE).toBeCloseTo(2.9952, 4);
  });

  it("同码自身不进池，但仍然抵扣库存", () => {
    const groups = groupsFor([stock("Z01", WANTED, 10, "灰蓝"), stock("A1", JUST_BELOW_3, 50, "偏下")]);
    expect(groups[0]!.wanted).toMatchObject({ inStock: 10, shortage: 90 });
    expect(groups[0]!.candidates.map((it) => it.code)).toEqual(["A1"]);
  });

  it("remaining = 库存 − 该码自身需求，扣到 0 就不再是候选", () => {
    const twoColour = pattern("gal-boundary-02", [
      { code: "Z01", name: "灰蓝", hex: WANTED, beads: 100 },
      { code: "A1", name: "偏下", hex: JUST_BELOW_3, beads: 50 },
    ]);
    expect(groupsFor([stock("A1", JUST_BELOW_3, 50, "偏下")], [twoColour])[0]!.candidates).toEqual(
      [],
    );

    const partial = groupsFor([stock("A1", JUST_BELOW_3, 62, "偏下")], [twoColour]);
    expect(partial[0]!.candidates).toEqual([
      expect.objectContaining({ code: "A1", remaining: 12 }),
    ]);
  });

  it("排序：ΔE 升序，同 ΔE 按库存原序", () => {
    const near = pattern("gal-near-01", [
      { code: "R04", name: "朱红", hex: "#d8412f", beads: 300 },
    ]);
    const groups = groupsFor(
      [
        stock("F1", "#e04a38", 10, "远"),
        stock("F2", "#d94635", 10, "中"),
        stock("F3", "#da4331", 10, "近"),
        stock("F4", "#da4331", 10, "并列"),
      ],
      [near],
    );
    expect(groups[0]!.candidates.map((it) => it.code)).toEqual(["F3", "F4", "F2", "F1"]);
  });

  it("hex 坏掉的库存条目落出池外，不带崩整页", () => {
    const groups = groupsFor([
      stock("BAD", "红色", 50, "手改过的"),
      stock("A1", JUST_BELOW_3, 50, "偏下"),
    ]);
    expect(groups[0]!.candidates.map((it) => it.code)).toEqual(["A1"]);
  });

  it("没有够近的候选时该组给空数组而不是消失", () => {
    const groups = groupsFor([stock("W", "#ffffff", 900, "纯白")]);
    expect(groups).toHaveLength(1);
    expect(groups[0]!.candidates).toEqual([]);
  });
});

describe("T-INV-5 采购文本（D-INV-11 / BD15）", () => {
  const shopping = pattern("gal-shopping-01", [
    { code: "R04", name: "朱红", hex: "#d8412f", beads: 250 },
    { code: "Y01", name: "明黄", hex: "#f5d13b", beads: 230 },
  ]);
  const inventory = [stock("G12", "#da4331", 96, "Rose")];

  function text(): string {
    const requirements = selectRequirements([project(shopping.id)], lookupOf(shopping));
    const shortages = selectShortages(requirements, inventory);
    return buildPurchaseText(shortages, selectSubstituteGroups(shortages, requirements, inventory));
  }

  it("整串确定：码 / 名 / hex / 颗数四要素齐全，末行给合计", () => {
    expect(text()).toBe(
      [
        "拼豆采购清单（正在拼 / 待拼项目 vs 当前库存）",
        "",
        "R04 朱红 #d8412f 缺 250 颗",
        "  可替代：G12 Rose #da4331（ΔE00 0.66，库存余 96 颗）",
        "Y01 明黄 #f5d13b 缺 230 颗",
        "",
        "合计缺 480 颗，共 2 个色号",
      ].join("\n"),
    );
  });

  it("文本里没有任何链接（BD15：不带商购出口）", () => {
    expect(text()).not.toMatch(/http|:\/\//);
  });

  it("没有缺口时是空串（页面据此不渲染导出控件）", () => {
    expect(buildPurchaseText([], [])).toBe("");
  });
});

describe("T-INV-15 fixture 不变量（BOM 真源的地基）", () => {
  it("每张图纸 Σ palette.beads === beadCount", () => {
    for (const item of PATTERNS) {
      const sum = item.palette.reduce((total, entry) => total + entry.beads, 0);
      expect([item.id, sum]).toEqual([item.id, item.beadCount]);
    }
  });

  it("同一个码跨图纸的名称与 hex 一致（扁平码空间，BD20）", () => {
    const seen = new Map<string, { name: string; hex: string }>();
    for (const item of PATTERNS) {
      for (const entry of item.palette) {
        const previous = seen.get(entry.code);
        if (previous === undefined) seen.set(entry.code, { name: entry.name, hex: entry.hex });
        else expect([entry.code, previous]).toEqual([entry.code, { name: entry.name, hex: entry.hex }]);
      }
    }
    expect(seen.size).toBeGreaterThan(0);
  });
});

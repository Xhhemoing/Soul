import { describe, expect, it } from "vitest";

import { createGrid } from "../algo/grid.ts";
import { PATTERNS } from "../fixtures/catalog.ts";
import { asCreatorId, asPatternId, mintProjectId } from "./ids.ts";
import {
  buildPurchaseText,
  clampBeads,
  conversionRequirements,
  mergeRequirements,
  normalizeCode,
  normalizeHex,
  parseHexColor,
  selectRequirements,
  selectShortages,
  selectStock,
  selectSubstituteGroups,
  type RequirementRow,
} from "./inventory.ts";
import { createPatternDoc } from "./patterns.ts";
import {
  GALLERY_PALETTE,
  GENERIC_5MM_PALETTE,
  type InventoryEntry,
  type PaletteNamespaceId,
  type Pattern,
  type PatternDoc,
  type Project,
  type ProjectStatus,
} from "./types.ts";

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

// BD20: every row now travels with the palette it came from. These two helpers
// default to 画廊 because that is what every pre-BD20 case in this file meant.
function requirement(code: string, required: number, hex = "#123456"): RequirementRow {
  return { paletteId: GALLERY_PALETTE, code, name: `名-${code}`, hex, required };
}

function stock(
  code: string,
  hex: string,
  beads: number,
  name = `名-${code}`,
  paletteId: PaletteNamespaceId = GALLERY_PALETTE,
): InventoryEntry {
  return { paletteId, code, name, hex, beads };
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
    // WP-B08 / DEV-GAL-3: 鸟居本轮补上网格，颗数改由 `grids.ts` 派生（D-ASM-2），
    // 所以这里的两个鸟居数字跟着走——聚合规则本身一字未动。
    expect(byCode.get("B05")).toBe(108 + 266);
    expect(byCode.get("H02")).toBe(126);
    expect(byCode.get("R04")).toBe(790);
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

  // §4.5: the code text is no longer globally unique, so every line names its
  // palette. Without the tag「G07」would be ambiguous on a shopping list.
  it("整串确定：命名空间 / 码 / 名 / hex / 颗数五要素齐全，末行给合计", () => {
    expect(text()).toBe(
      [
        "拼豆采购清单（正在拼 / 待拼项目 vs 当前库存）",
        "",
        "【画廊】R04 朱红 #d8412f 缺 250 颗",
        "  可替代：【画廊】G12 Rose #da4331（ΔE00 0.66，库存余 96 颗）",
        "【画廊】Y01 明黄 #f5d13b 缺 230 颗",
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

describe("T-UP-18 G07 命名空间对撞（BD20 / §4.6）", () => {
  // 画廊 G07 是苔绿 #4c7a44，generic-5mm 的 G07 是 Silver #b7bfc6（色板第 7 条，
  // 下标 6）。BD20 记的就是这一撞：扁平码空间下它们会互相抵扣。
  const GALLERY_G07 = "#4c7a44";
  const GENERIC_G07 = "#b7bfc6";

  const galleryPattern = pattern("gal-moss-07", [
    { code: "G07", name: "苔绿", hex: GALLERY_G07, beads: 100 },
  ]);

  function convertedG07(cells: number): PatternDoc {
    const grid = createGrid(cells, 1, new Array<number>(cells).fill(6));
    return createPatternDoc(mintProjectId(), grid);
  }

  function rows(inventory: InventoryEntry[]) {
    const requirements = mergeRequirements(
      selectRequirements([project(galleryPattern.id)], lookupOf(galleryPattern)),
      conversionRequirements([convertedG07(96)]),
    );
    const shortages = selectShortages(requirements, inventory);
    return { requirements, shortages, groups: selectSubstituteGroups(shortages, requirements, inventory) };
  }

  it("转换需求就是 buildBom 的输出，码 / 名 / hex 全来自 generic-5mm", () => {
    expect(conversionRequirements([convertedG07(96)])).toEqual([
      {
        paletteId: GENERIC_5MM_PALETTE,
        code: "G07",
        name: "Silver",
        hex: GENERIC_G07,
        required: 96,
      },
    ]);
  });

  it("两个 G07 各算各的：画廊库存不抵扣 generic-5mm 的缺口", () => {
    const { requirements, shortages } = rows([stock("G07", GALLERY_G07, 100, "苔绿")]);
    expect(requirements).toHaveLength(2);
    // 画廊那行被自己的库存填满，所以不出缺口；generic 那行一颗都没有。
    expect(shortages).toEqual([
      expect.objectContaining({
        paletteId: GENERIC_5MM_PALETTE,
        code: "G07",
        required: 96,
        inStock: 0,
        shortage: 96,
      }),
    ]);
  });

  it("库存不足时两行同时出，互不合并", () => {
    const { shortages } = rows([stock("G07", GALLERY_G07, 40, "苔绿")]);
    expect(shortages.map((row) => [row.paletteId, row.shortage])).toEqual([
      [GENERIC_5MM_PALETTE, 96],
      [GALLERY_PALETTE, 60],
    ]);
  });

  it("替代池不跨命名空间：颜色够近的画廊豆也不入 generic-5mm 的池", () => {
    // #b8c0c7 与 generic-5mm G07 只差一点点，扁平码空间下必然入池。
    const { groups } = rows([stock("N01", "#b8c0c7", 500, "银灰", GALLERY_PALETTE)]);
    const generic = groups.find((group) => group.wanted.paletteId === GENERIC_5MM_PALETTE);
    expect(generic?.wanted.shortage).toBe(96);
    expect(generic?.candidates).toEqual([]);
  });

  it("selectStock 先按命名空间再按码排序", () => {
    const sorted = selectStock([
      stock("G07", GENERIC_G07, 1, "Silver", GENERIC_5MM_PALETTE),
      stock("Z99", "#000000", 1, "尾巴"),
      stock("G07", GALLERY_G07, 1, "苔绿"),
    ]);
    expect(sorted.map((entry) => [entry.paletteId, entry.code])).toEqual([
      [GALLERY_PALETTE, "G07"],
      [GALLERY_PALETTE, "Z99"],
      [GENERIC_5MM_PALETTE, "G07"],
    ]);
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

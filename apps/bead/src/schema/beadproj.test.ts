import { describe, expect, it } from "vitest";

import { createGrid } from "../algo/grid.ts";
import { mintProjectId } from "../stores/ids.ts";
import { createPatternDoc } from "../stores/patterns.ts";
import { GALLERY_PALETTE, GENERIC_5MM_PALETTE, type Project } from "../stores/types.ts";
import {
  MAX_IMPORT_FILE_BYTES,
  MAX_IMPORT_TITLE_LENGTH,
  UNTITLED_IMPORT,
  beadprojFileName,
  buildBeadprojEntry,
  buildBeadprojFile,
  checkImportFileSize,
  parseBeadproj,
  parseBeadprojText,
  serializeBeadproj,
  type BeadprojEntry,
  type BeadprojFile,
} from "./beadproj.ts";

// T-IE-1…6. Everything here is a pure function of a JSON value: the file-level
// gate, the three size gates and the entry-level rejections all decide before a
// single byte reaches storage.

function projectFields(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    title: "元宵提灯",
    sourcePatternId: null,
    status: "todo",
    createdAt: 1_730_000_000_000,
    backdrop: "black",
    backdropColor: "#101014",
    ...overrides,
  };
}

function patternFields(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    paletteId: GENERIC_5MM_PALETTE,
    width: 3,
    height: 2,
    cells: [0, 1, -1, 47, -1, 2],
    ...overrides,
  };
}

function fileWith(entry: unknown, inventory?: unknown): unknown {
  return {
    format: "beadproj",
    version: 1,
    projects: [entry],
    ...(inventory === undefined ? {} : { inventory }),
  };
}

function expectDocument(value: unknown) {
  const result = parseBeadproj(value);
  if (!result.ok) throw new Error(`预期通过文件级校验，实际 ${result.failure.code}`);
  return result.document;
}

function expectFailure(value: unknown) {
  const result = parseBeadproj(value);
  if (result.ok) throw new Error("预期文件级失败，实际通过");
  return result.failure;
}

function firstEntryError(value: unknown) {
  const document = expectDocument(value);
  expect(document.entries).toHaveLength(0);
  expect(document.entryErrors).toHaveLength(1);
  return document.entryErrors[0]!;
}

describe("T-IE-1 合法文件通过，重建结果只含白名单键", () => {
  it("单 entry 文件解出一条 entry，序号从 1 开始", () => {
    const document = expectDocument(
      fileWith({ project: projectFields(), pattern: patternFields() }),
    );
    expect(document.total).toBe(1);
    expect(document.entryErrors).toEqual([]);
    expect(document.entries[0]!.index).toBe(1);
    expect(document.entries[0]!.entry.project.title).toBe("元宵提灯");
    expect(document.entries[0]!.entry.pattern?.cells).toEqual([0, 1, -1, 47, -1, 2]);
  });

  it("种进去的未知键在输出里绝迹，不会被下一次导出写回", () => {
    const document = expectDocument(
      fileWith({
        project: projectFields({ id: "proj-smuggled", doneBits: [1, 2, 3] }),
        pattern: patternFields({ steps: [{ cells: [] }] }),
        progress: {
          mode: "color-by-color",
          stepIndex: 3,
          elapsedMs: 1000,
          updatedAt: 42,
          projectId: "proj-smuggled",
        },
        extra: true,
      }),
    );
    const { project, pattern, progress } = document.entries[0]!.entry;
    expect(Object.keys(project).sort()).toEqual([
      "backdrop",
      "backdropColor",
      "createdAt",
      "sourcePatternId",
      "status",
      "title",
    ]);
    expect(Object.keys(pattern!).sort()).toEqual(["cells", "height", "paletteId", "width"]);
    expect(Object.keys(progress!).sort()).toEqual([
      "elapsedMs",
      "mode",
      "stepIndex",
      "updatedAt",
    ]);
    expect(JSON.stringify(document.entries[0]!.entry)).not.toContain("smuggled");
  });

  it("provenance 在场时逐字段校验，不在场时不补", () => {
    const withProvenance = expectDocument(
      fileWith({
        project: projectFields(),
        pattern: patternFields({ provenance: { kind: "Photo", ditherApplied: true } }),
      }),
    );
    expect(withProvenance.entries[0]!.entry.pattern?.provenance).toEqual({
      kind: "Photo",
      ditherApplied: true,
    });

    const bogus = expectDocument(
      fileWith({
        project: projectFields(),
        pattern: patternFields({ provenance: { kind: "Sketch", ditherApplied: true } }),
      }),
    );
    expect(bogus.entries[0]!.entry.pattern?.provenance).toBeUndefined();
  });

  it("projects 为空数组合法（纯库存备份）", () => {
    const document = expectDocument({ format: "beadproj", version: 1, projects: [] });
    expect(document.total).toBe(0);
    expect(document.entries).toEqual([]);
  });
});

describe("T-IE-2 format / version 是 fail-closed 的门", () => {
  it("format 不对 → SCHEMA_INVALID", () => {
    expect(expectFailure({ format: "beadproject", version: 1, projects: [] }).code).toBe(
      "SCHEMA_INVALID",
    );
    expect(expectFailure({ version: 1, projects: [] }).code).toBe("SCHEMA_INVALID");
    expect(expectFailure(["beadproj"]).code).toBe("SCHEMA_INVALID");
  });

  it("version: 2 → UNSUPPORTED_VERSION，且不去解析 projects", () => {
    const failure = expectFailure({
      format: "beadproj",
      version: 2,
      projects: [{ project: projectFields() }],
    });
    expect(failure.code).toBe("UNSUPPORTED_VERSION");
    expect(failure.message).toContain("2");
  });

  it("projects / inventory 不是数组 → SCHEMA_INVALID", () => {
    expect(expectFailure({ format: "beadproj", version: 1, projects: {} }).code).toBe(
      "SCHEMA_INVALID",
    );
    expect(
      expectFailure({ format: "beadproj", version: 1, projects: [], inventory: 3 }).code,
    ).toBe("SCHEMA_INVALID");
  });

  it("坏 JSON 文本 → SCHEMA_INVALID，不抛异常", () => {
    const result = parseBeadprojText("{ 不是 JSON");
    expect(result.ok).toBe(false);
    if (!result.ok) expect(result.failure.code).toBe("SCHEMA_INVALID");
  });
});

describe("T-IE-3 cells / paletteId 违规 → entry 拒绝且带序号", () => {
  it("长度不符", () => {
    const error = firstEntryError(
      fileWith({ project: projectFields(), pattern: patternFields({ cells: [0, 1, 2] }) }),
    );
    expect(error).toMatchObject({ index: 1, code: "SCHEMA_INVALID" });
    expect(error.message).toContain("3");
  });

  it("越界码 -2 与 48 都不收", () => {
    for (const bad of [-2, 48, 1.5, "0"]) {
      const error = firstEntryError(
        fileWith({
          project: projectFields(),
          pattern: patternFields({ cells: [bad, 1, -1, 47, -1, 2] }),
        }),
      );
      expect(error.code).toBe("SCHEMA_INVALID");
    }
  });

  it("paletteId 不是 generic-5mm（含 gallery）→ 拒绝", () => {
    const error = firstEntryError(
      fileWith({
        project: projectFields(),
        pattern: patternFields({ paletteId: GALLERY_PALETTE }),
      }),
    );
    expect(error.message).toContain(GENERIC_5MM_PALETTE);
  });

  it("序号是文件里的位置：第 2 条坏了就报 2", () => {
    const document = expectDocument({
      format: "beadproj",
      version: 1,
      projects: [
        { project: projectFields(), pattern: patternFields() },
        { project: projectFields({ status: "沉睡" }), pattern: patternFields() },
        { project: projectFields(), pattern: patternFields() },
      ],
    });
    expect(document.total).toBe(3);
    expect(document.entries.map((accepted) => accepted.index)).toEqual([1, 3]);
    expect(document.entryErrors.map((error) => error.index)).toEqual([2]);
  });
});

describe("T-IE-4 尺寸三层门", () => {
  function sized(width: number, height: number): unknown {
    return fileWith({
      project: projectFields(),
      pattern: patternFields({ width, height, cells: new Array<number>(width * height).fill(-1) }),
    });
  }

  it("① 文件门在读字节之前判：20MB 通过，多一字节不通过", () => {
    expect(checkImportFileSize(MAX_IMPORT_FILE_BYTES)).toBeNull();
    const failure = checkImportFileSize(MAX_IMPORT_FILE_BYTES + 1);
    expect(failure?.code).toBe("FILE_TOO_LARGE");
    expect(failure?.message).toContain("20MB");
  });

  it("② 513×10 → SCHEMA_INVALID（超出格式效度上限）", () => {
    const error = firstEntryError(sized(513, 10));
    expect(error.code).toBe("SCHEMA_INVALID");
    expect(error.message).toContain("512");
  });

  it("③ 57×10 → GRID_TOO_LARGE_FOR_V0，文案点名两个数字", () => {
    const error = firstEntryError(sized(57, 10));
    expect(error.code).toBe("GRID_TOO_LARGE_FOR_V0");
    expect(error.message).toContain("57×10");
    expect(error.message).toContain("56×56");
  });

  it("56×56 通过；0 与负数不是正整数", () => {
    expect(expectDocument(sized(56, 56)).entries).toHaveLength(1);
    expect(firstEntryError(sized(0, 4)).code).toBe("SCHEMA_INVALID");
    expect(
      firstEntryError(
        fileWith({ project: projectFields(), pattern: patternFields({ width: -3 }) }),
      ).code,
    ).toBe("SCHEMA_INVALID");
  });
});

describe("T-IE-5 双射：sourcePatternId 与 pattern 同生共死（D-IE-9）", () => {
  it("null 来源却没有 pattern → 拒绝", () => {
    const error = firstEntryError(fileWith({ project: projectFields() }));
    expect(error.message).toContain("pattern");
  });

  it("画廊来源却带 pattern → 拒绝", () => {
    const error = firstEntryError(
      fileWith({
        project: projectFields({ sourcePatternId: "gal-lantern-04" }),
        pattern: patternFields(),
      }),
    );
    expect(error.message).toContain("pattern");
  });

  it("画廊来源不带 pattern → 通过（纯元数据 entry）", () => {
    const document = expectDocument(
      fileWith({ project: projectFields({ sourcePatternId: "gal-lantern-04" }) }),
    );
    expect(document.entries[0]!.entry.pattern).toBeUndefined();
  });

  it("sourcePatternId 是别的字符串 → 拒绝（不是 gal- 前缀）", () => {
    expect(firstEntryError(fileWith({ project: projectFields({ sourcePatternId: "proj-7" }) })).code).toBe(
      "SCHEMA_INVALID",
    );
  });
});

describe("T-IE-6 字段门：不修补、不静默截断", () => {
  it("非法 status / backdrop / createdAt 各自拒绝该 entry", () => {
    const cases: Record<string, unknown>[] = [
      { status: "paused" },
      { backdrop: "rainbow" },
      { createdAt: Number.POSITIVE_INFINITY },
      { createdAt: 0 },
      { createdAt: "1730000000000" },
      { backdropColor: 16 },
    ];
    for (const overrides of cases) {
      const error = firstEntryError(
        fileWith({ project: projectFields(overrides), pattern: patternFields() }),
      );
      expect(error.code).toBe("SCHEMA_INVALID");
    }
  });

  it("title 超过 256 字 → 拒绝该 entry，而不是截断", () => {
    const error = firstEntryError(
      fileWith({
        project: projectFields({ title: "豆".repeat(MAX_IMPORT_TITLE_LENGTH + 1) }),
        pattern: patternFields(),
      }),
    );
    expect(error.message).toContain(String(MAX_IMPORT_TITLE_LENGTH));
  });

  it("恰好 256 字通过；trim 后为空 → 未命名导入", () => {
    const long = expectDocument(
      fileWith({
        project: projectFields({ title: "豆".repeat(MAX_IMPORT_TITLE_LENGTH) }),
        pattern: patternFields(),
      }),
    );
    expect(long.entries[0]!.entry.project.title).toHaveLength(MAX_IMPORT_TITLE_LENGTH);

    const blank = expectDocument(
      fileWith({ project: projectFields({ title: "   " }), pattern: patternFields() }),
    );
    expect(blank.entries[0]!.entry.project.title).toBe(UNTITLED_IMPORT);
  });

  it("progress 复用 toProgressCursor 的规则：mode 不认得就拒绝整条 entry", () => {
    const error = firstEntryError(
      fileWith({
        project: projectFields(),
        pattern: patternFields(),
        progress: { mode: "spiral", stepIndex: 1, elapsedMs: 0, updatedAt: 1 },
      }),
    );
    expect(error.message).toContain("progress");
  });

  it("库存缺 / 未知 paletteId 一律丢弃并计数，绝不盖章（D-IE-12）", () => {
    const document = expectDocument(
      fileWith({ project: projectFields({ sourcePatternId: "gal-lantern-04" }) }, [
        { paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 500 },
        { code: "G07", name: "苔绿", hex: "#4c7a44", beads: 120 },
        { paletteId: "brand-perler", code: "P01", name: "White", hex: "#ffffff", beads: 10 },
        { paletteId: "gallery", code: "H02", name: "薄荷绿", hex: "#7fd6a2", beads: 1.5 },
      ]),
    );
    expect(document.inventory).toEqual([
      { paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 500 },
    ]);
    expect(document.droppedInventory).toBe(3);
  });
});

describe("写入侧：同一个 schema 的另一半", () => {
  const owner = mintProjectId();

  function project(overrides: Partial<Project> = {}): Project {
    return {
      id: owner,
      title: "元宵提灯",
      sourcePatternId: null,
      status: "draft",
      createdAt: 1_730_000_000_000,
      backdrop: "black",
      backdropColor: "#101014",
      ...overrides,
    };
  }

  const doc = createPatternDoc(owner, createGrid(3, 2, [0, 1, null, 47, null, 2]), {
    kind: "PixelArt",
    ditherApplied: false,
  });

  it("导出的 entry 过得了自己的校验器，且不带 id", () => {
    const entry = buildBeadprojEntry(project(), doc, null);
    expect(JSON.stringify(entry)).not.toContain(owner);
    const document = expectDocument(buildBeadprojFile([entry]));
    expect(document.entries).toHaveLength(1);
    expect(document.entries[0]!.entry.pattern?.cells).toEqual([0, 1, -1, 47, -1, 2]);
  });

  it("画廊项目导出成纯元数据 entry，即使手里有文档也不写 pattern（双射）", () => {
    const entry = buildBeadprojEntry(
      project({ sourcePatternId: "gal-lantern-04" as Project["sourcePatternId"] }),
      doc,
      null,
    );
    expect(entry.pattern).toBeUndefined();
    expect(expectDocument(buildBeadprojFile([entry])).entries).toHaveLength(1);
  });

  it("序列化确定：同一份状态两次导出逐字节相同，且不含时间戳字段", () => {
    const build = (): BeadprojFile => buildBeadprojFile([buildBeadprojEntry(project(), doc, null)]);
    expect(serializeBeadproj(build())).toBe(serializeBeadproj(build()));
    expect(serializeBeadproj(build())).not.toContain("exportedAt");
  });

  it("空库存不写 inventory 键；有库存则逐字段重建", () => {
    const entries: BeadprojEntry[] = [buildBeadprojEntry(project(), doc, null)];
    expect(buildBeadprojFile(entries).inventory).toBeUndefined();
    expect(
      buildBeadprojFile(entries, [
        { paletteId: GENERIC_5MM_PALETTE, code: "G07", name: "Silver", hex: "#b7bfc6", beads: 5 },
      ]).inventory,
    ).toEqual([
      { paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 5 },
    ]);
  });

  it("文件名消毒：空标题回落到 bead-project，路径分隔符进不去", () => {
    expect(beadprojFileName("元宵提灯")).toBe("元宵提灯.beadproj");
    expect(beadprojFileName("   ")).toBe("bead-project.beadproj");
    expect(beadprojFileName("../../etc/passwd")).toBe("etc-passwd.beadproj");
    expect(beadprojFileName("a".repeat(200))).toHaveLength(64 + ".beadproj".length);
  });
});

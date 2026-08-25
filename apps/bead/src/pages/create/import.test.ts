import { describe, expect, it, vi } from "vitest";

import { parseBeadproj, type BeadprojDocument } from "../../schema/beadproj.ts";
import { mintProjectId, type ProjectId } from "../../stores/ids.ts";
import type { InventoryEntry, PatternDoc, Project } from "../../stores/types.ts";
import {
  SNIFF_BYTES,
  describeImportResult,
  hasBeadprojExtension,
  importBeadproj,
  sniffBytes,
  sniffFile,
  type BeadprojImportDeps,
} from "./import.ts";

// T-IE-7 plus the write sequence D-IE-10 spells out. No DOM and no storage:
// the deps are recorded so the *order* of the calls is what gets asserted.

const PNG_HEAD = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
const JPEG_HEAD = [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46];

function bytes(...values: number[]): Uint8Array {
  return new Uint8Array(values);
}

function fileOf(name: string, values: readonly number[]): File {
  return new File([new Uint8Array(values)], name);
}

describe("T-IE-7 魔数嗅探：只看开头，认得出就停手", () => {
  it("PNG / JPEG 头识别为图片", () => {
    expect(sniffBytes(bytes(...PNG_HEAD))).toBe("png");
    expect(sniffBytes(bytes(...JPEG_HEAD))).toBe("jpeg");
  });

  it("首字节 { 识别为 json，前导 BOM 与空白不算内容", () => {
    expect(sniffBytes(new TextEncoder().encode('{"format":"beadproj"}'))).toBe("json");
    expect(sniffBytes(bytes(0x20, 0x0a, 0x7b))).toBe("json");
    expect(sniffBytes(bytes(0xef, 0xbb, 0xbf, 0x7b))).toBe("json");
  });

  it("其余一律 unknown——包括伪 .pat 字节与 JSON 数组", () => {
    expect(sniffBytes(bytes(0x50, 0x41, 0x54, 0x00, 0x01, 0x02))).toBe("unknown");
    expect(sniffBytes(new TextEncoder().encode("GAMEDEV\u0001binary"))).toBe("unknown");
    expect(sniffBytes(new TextEncoder().encode("[1,2,3]"))).toBe("unknown");
    expect(sniffBytes(bytes())).toBe("unknown");
  });

  it("短文件不越界：3 字节、15 字节都只是分类不同，不抛异常", () => {
    expect(sniffBytes(bytes(0x89, 0x50, 0x4e))).toBe("unknown");
    expect(sniffBytes(new Uint8Array(15))).toBe("unknown");
    expect(sniffBytes(bytes(...PNG_HEAD.slice(0, 4)))).toBe("png");
  });

  it("读文件时只切前 16 字节，后面的字节根本没被要过", async () => {
    const big = fileOf("mystery.pat", new Array<number>(4096).fill(0x41));
    const slice = vi.spyOn(big, "slice");
    expect(await sniffFile(big)).toBe("unknown");
    expect(slice).toHaveBeenCalledWith(0, SNIFF_BYTES);

    const short = fileOf("short.gamedev", new Array<number>(15).fill(0x42));
    expect(await sniffFile(short)).toBe("unknown");
    expect(await sniffFile(fileOf("a.png", PNG_HEAD))).toBe("png");
  });

  it("扩展名只在 json 分支上做一次判断", () => {
    expect(hasBeadprojExtension("bead-projects.beadproj")).toBe(true);
    expect(hasBeadprojExtension("BEAD.BEADPROJ")).toBe(true);
    expect(hasBeadprojExtension("notes.json")).toBe(false);
  });
});

interface Recorder {
  readonly calls: string[];
  readonly projects: Project[];
  readonly docs: PatternDoc[];
  readonly cursors: { projectId: ProjectId; stepIndex: number }[];
  readonly inventory: InventoryEntry[];
}

function recorder(
  onSave: (doc: PatternDoc) => Promise<void> = async () => {},
  seedInventory: readonly InventoryEntry[] = [],
): { deps: BeadprojImportDeps; log: Recorder } {
  const log: Recorder = { calls: [], projects: [], docs: [], cursors: [], inventory: [] };
  return {
    log,
    deps: {
      async savePatternDoc(doc) {
        log.calls.push("savePatternDoc");
        await onSave(doc);
        log.docs.push(doc);
      },
      addProject(project) {
        log.calls.push("addProject");
        log.projects.push(project);
      },
      upsertProgress(cursor) {
        log.calls.push("upsertProgress");
        log.cursors.push({ projectId: cursor.projectId, stepIndex: cursor.stepIndex });
      },
      addInventoryEntry(entry) {
        log.calls.push("addInventoryEntry");
        log.inventory.push(entry);
      },
      inventory: seedInventory,
    },
  };
}

function entry(title: string, cell = 0): unknown {
  return {
    project: {
      title,
      sourcePatternId: null,
      status: "todo",
      createdAt: 1_730_000_000_000,
      backdrop: "black",
      backdropColor: "#101014",
    },
    pattern: { paletteId: "generic-5mm", width: 2, height: 1, cells: [cell, -1] },
    progress: { mode: "row-by-row", stepIndex: 4, elapsedMs: 9_000, updatedAt: 7 },
  };
}

function documentOf(...entries: unknown[]): BeadprojDocument {
  const parsed = parseBeadproj({ format: "beadproj", version: 1, projects: entries });
  if (!parsed.ok) throw new Error("测试样本本身不合法");
  return parsed.document;
}

describe("D-IE-10 导入执行序：文档先落盘，项目后铸造", () => {
  it("每条 entry 都是 savePatternDoc → addProject → upsertProgress", async () => {
    const { deps, log } = recorder();
    const result = await importBeadproj(documentOf(entry("甲"), entry("乙", 3)), deps);

    expect(log.calls).toEqual([
      "savePatternDoc",
      "addProject",
      "upsertProgress",
      "savePatternDoc",
      "addProject",
      "upsertProgress",
    ]);
    expect(result).toMatchObject({ total: 2, imported: 2, failedIndex: null });
    expect(log.projects.map((project) => project.title)).toEqual(["甲", "乙"]);
    // The cells travelled through the store's own codec, not a second one.
    expect([...log.docs[1]!.cells]).toEqual([3, -1]);
  });

  it("画廊来源的 entry 不写文档，只铸项目", async () => {
    const { deps, log } = recorder();
    await importBeadproj(
      documentOf({
        project: {
          title: "画廊来的",
          sourcePatternId: "gal-lantern-04",
          status: "done",
          createdAt: 5,
          backdrop: "white",
          backdropColor: "#ffffff",
        },
      }),
      deps,
    );
    expect(log.calls).toEqual(["addProject"]);
    expect(log.projects[0]!.sourcePatternId).toBe("gal-lantern-04");
    expect(log.projects[0]!.status).toBe("done");
  });

  it("T-IE-10 断路：第 2 条写盘失败 → 第 2 条不铸项目、第 3 条不执行", async () => {
    let seen = 0;
    const { deps, log } = recorder(async () => {
      seen += 1;
      if (seen === 2) throw new Error("配额用完了");
    });
    const result = await importBeadproj(
      documentOf(entry("甲"), entry("乙"), entry("丙")),
      deps,
    );

    expect(result).toMatchObject({ total: 3, imported: 1, failedIndex: 2 });
    expect(log.projects.map((project) => project.title)).toEqual(["甲"]);
    expect(log.calls).toEqual(["savePatternDoc", "addProject", "upsertProgress", "savePatternDoc"]);
    expect(describeImportResult(result)).toContain("已导入 1 / 3");
    expect(describeImportResult(result)).toContain("第 2 条");
  });

  it("被校验器拒绝的 entry 计入分母，但不拦住后面的条目", async () => {
    const { deps, log } = recorder();
    const result = await importBeadproj(
      documentOf(entry("甲"), { project: { title: "坏的" } }, entry("丙")),
      deps,
    );
    expect(result).toMatchObject({ total: 3, imported: 2, failedIndex: null });
    expect(result.entryErrors.map((error) => error.index)).toEqual([2]);
    expect(log.projects.map((project) => project.title)).toEqual(["甲", "丙"]);
  });

  it("id 一律现铸：文件里没有 id，两次导入互不相干（D-IE-8）", async () => {
    const first = recorder();
    const second = recorder();
    const file = documentOf(entry("甲"), entry("乙"));
    await importBeadproj(file, first.deps);
    await importBeadproj(file, second.deps);

    const ids = [...first.log.projects, ...second.log.projects].map((project) => project.id);
    expect(new Set(ids).size).toBe(4);
    for (const [index, doc] of [...first.log.docs, ...second.log.docs].entries()) {
      expect(doc.projectId).toBe(ids[index]);
    }
  });

  it("游标盖新 id；updatedAt 归 action 盖章（BD19），导入端不传", async () => {
    const { deps, log } = recorder();
    await importBeadproj(documentOf(entry("甲")), deps);
    expect(log.cursors).toEqual([{ projectId: log.projects[0]!.id, stepIndex: 4 }]);
  });
});

describe("D-IE-12 库存合并：本地为准，绝不盖章", () => {
  const localG07: InventoryEntry = {
    paletteId: "gallery",
    code: "G07",
    name: "苔绿",
    hex: "#4c7a44",
    beads: 120,
  };

  function withInventory(...rows: unknown[]): BeadprojDocument {
    const parsed = parseBeadproj({
      format: "beadproj",
      version: 1,
      projects: [],
      inventory: rows,
    });
    if (!parsed.ok) throw new Error("测试样本本身不合法");
    return parsed.document;
  }

  it("同码不同命名空间并存；已存在的 (paletteId, code) 跳过；缺命名空间的丢弃", async () => {
    const { deps, log } = recorder(undefined, [localG07]);
    const result = await importBeadproj(
      withInventory(
        { paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 500 },
        { paletteId: "gallery", code: "G07", name: "苔绿", hex: "#4c7a44", beads: 999 },
        { code: "G09", name: "无名", hex: "#000000", beads: 10 },
      ),
      deps,
    );

    expect(result).toMatchObject({
      inventoryAdded: 1,
      inventorySkipped: 1,
      inventoryDropped: 1,
    });
    expect(log.inventory).toEqual([
      { paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 500 },
    ]);
    expect(describeImportResult(result)).toContain("库存新增 1 / 跳过 1 / 丢弃 1");
  });

  it("同一份库存导入两次是幂等的：第二次全跳过", async () => {
    const rows = withInventory({
      paletteId: "generic-5mm",
      code: "G07",
      name: "Silver",
      hex: "#b7bfc6",
      beads: 500,
    });
    const first = recorder();
    const firstResult = await importBeadproj(rows, first.deps);
    expect(firstResult.inventoryAdded).toBe(1);

    const second = recorder(undefined, first.log.inventory);
    const secondResult = await importBeadproj(rows, second.deps);
    expect(secondResult).toMatchObject({ inventoryAdded: 0, inventorySkipped: 1 });
    expect(second.log.inventory).toEqual([]);
  });

  it("大小写与空白不同的同一个色号算同一条（既有归一化）", async () => {
    const { deps, log } = recorder(undefined, [
      { paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 1 },
    ]);
    const result = await importBeadproj(
      withInventory({
        paletteId: "generic-5mm",
        code: " g07 ",
        name: "Silver",
        hex: "#b7bfc6",
        beads: 500,
      }),
      deps,
    );
    expect(result.inventorySkipped).toBe(1);
    expect(log.inventory).toEqual([]);
  });
});

describe("汇总文案", () => {
  it("一条都没有的空归档也说得出话", async () => {
    const { deps } = recorder();
    const result = await importBeadproj(documentOf(), deps);
    expect(describeImportResult(result)).toBe("已导入 0 / 0 个项目。");
  });

  it("项目 id 由调用方注入时也照样串起来（可重复的测试铸造）", async () => {
    const forced = mintProjectId();
    const { deps, log } = recorder();
    await importBeadproj(documentOf(entry("甲")), { ...deps, mintId: () => forced });
    expect(log.projects[0]!.id).toBe(forced);
    expect(log.docs[0]!.projectId).toBe(forced);
  });
});

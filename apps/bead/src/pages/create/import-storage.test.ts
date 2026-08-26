import { describe, expect, it } from "vitest";
import { IDBFactory } from "fake-indexeddb";

import { createGrid } from "../../algo/grid.ts";
import { parseBeadprojText, serializeBeadproj } from "../../schema/beadproj.ts";
import { STORE, readStoreEntries } from "../../stores/bead-v1.ts";
import { asPatternId, mintProjectId } from "../../stores/ids.ts";
import { createPatternDoc } from "../../stores/patterns.ts";
import { STORAGE_KEY, createRepository, type Repository } from "../../stores/repository.ts";
import { FlakyStorage } from "../../test/flaky-storage.ts";
import type {
  InventoryEntry,
  PatternDoc,
  ProgressCursor,
  Project,
} from "../../stores/types.ts";
import { collectBeadprojArchive } from "./export.ts";
import { importBeadproj } from "./import.ts";

/**
 * T-IE-8…12: the round trip against a real `bead-v1` on `fake-indexeddb`.
 *
 * The little `Library` below stands in for the reducer — the same three
 * actions, the same last-write-wins arrays, persisted through the same
 * repository — so the assertions are about what reaches disk rather than about
 * React. `store.tsx` owns the wiring and the panel test drives it end to end.
 */

const OWNER_GRID = createGrid(3, 2, [0, 1, null, 47, null, 2]);

class Library {
  projects: Project[] = [];
  inventory: InventoryEntry[] = [];
  progress: ProgressCursor[] = [];
  /** BD19: the action stamps `updatedAt`, and the import path cannot restore it. */
  clock = 4_242;

  constructor(readonly repo: Repository) {}

  get deps() {
    return {
      savePatternDoc: (doc: PatternDoc) => this.repo.savePatternDoc(doc),
      addProject: (project: Project) => {
        this.projects = [project, ...this.projects];
      },
      upsertProgress: (cursor: Omit<ProgressCursor, "updatedAt">) => {
        this.progress = [
          ...this.progress.filter((row) => row.projectId !== cursor.projectId),
          { ...cursor, updatedAt: (this.clock += 1) },
        ];
      },
      addInventoryEntry: (entry: InventoryEntry) => {
        this.inventory = [...this.inventory, entry];
      },
      inventory: this.inventory,
    };
  }

  async flush(): Promise<void> {
    await this.repo.saveProjects(this.projects);
    await this.repo.saveInventory(this.inventory);
    await this.repo.saveProgress(this.progress);
  }
}

function library() {
  const factory = new IDBFactory();
  const legacy = new FlakyStorage();
  return { factory, legacy, library: new Library(createRepository(legacy, factory)) };
}

function converted(title: string): Project {
  return {
    id: mintProjectId(),
    title,
    sourcePatternId: null,
    status: "active",
    createdAt: 1_730_000_000_000,
    backdrop: "custom",
    backdropColor: "#223344",
  };
}

function gallery(title: string): Project {
  return {
    id: mintProjectId(),
    title,
    sourcePatternId: asPatternId("gal-lantern-04"),
    status: "done",
    createdAt: 1_720_000_000_000,
    backdrop: "white",
    backdropColor: "#ffffff",
  };
}

const STOCK: InventoryEntry[] = [
  { paletteId: "gallery", code: "G07", name: "苔绿", hex: "#4c7a44", beads: 120 },
  { paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 500 },
];

/** A seeded library: one converted project with a document, one gallery one. */
async function seeded() {
  const bundle = library();
  const source = converted("元宵提灯");
  const fixture = gallery("画廊来的");
  const doc = createPatternDoc(source.id, OWNER_GRID, { kind: "PixelArt", ditherApplied: false });

  await bundle.library.repo.savePatternDoc(doc);
  bundle.library.projects = [source, fixture];
  bundle.library.inventory = [...STOCK];
  bundle.library.progress = [
    {
      projectId: source.id,
      mode: "outline-infill",
      stepIndex: 6,
      elapsedMs: 91_000,
      updatedAt: 1_730_000_000_001,
    },
  ];
  await bundle.library.flush();
  return { ...bundle, source, fixture, doc };
}

async function exportAll(from: Library): Promise<string> {
  const { file } = await collectBeadprojArchive({
    projects: from.projects,
    inventory: from.inventory,
    progress: from.progress,
    loadPatternDoc: (id) => from.repo.loadPatternDoc(id),
  });
  return serializeBeadproj(file);
}

async function importInto(into: Library, text: string) {
  const parsed = parseBeadprojText(text);
  if (!parsed.ok) throw new Error(`归档没通过校验：${parsed.failure.code}`);
  const result = await importBeadproj(parsed.document, into.deps);
  await into.flush();
  return result;
}

describe("T-IE-8 旗舰往返：导出全部 → 清空 → 导入", () => {
  it("逐字段回来（id 与游标 updatedAt 除外，两者按设计重铸）", async () => {
    const before = await seeded();
    const text = await exportAll(before.library);

    // 清空 = 一套全新的仓与库，跟浏览器换机器同义。
    const after = library();
    const result = await importInto(after.library, text);
    expect(result).toMatchObject({ total: 2, imported: 2, failedIndex: null });

    const restored = await after.library.repo.loadProjects();
    expect(restored.map((project) => project.title).sort()).toEqual(
      ["元宵提灯", "画廊来的"].sort(),
    );

    const restoredSource = restored.find((project) => project.sourcePatternId === null)!;
    expect(restoredSource).toMatchObject({
      title: before.source.title,
      status: before.source.status,
      createdAt: before.source.createdAt,
      backdrop: before.source.backdrop,
      backdropColor: before.source.backdropColor,
    });
    expect(restoredSource.id).not.toBe(before.source.id);

    const restoredFixture = restored.find((project) => project.sourcePatternId !== null)!;
    expect(restoredFixture).toMatchObject({
      title: before.fixture.title,
      sourcePatternId: before.fixture.sourcePatternId,
      status: "done",
      backdrop: "white",
    });

    const restoredDoc = await after.library.repo.loadPatternDoc(restoredSource.id);
    expect(restoredDoc).not.toBeNull();
    expect(restoredDoc!.width).toBe(3);
    expect(restoredDoc!.height).toBe(2);
    expect([...restoredDoc!.cells]).toEqual([...before.doc.cells]);
    expect(restoredDoc!.provenance).toEqual({ kind: "PixelArt", ditherApplied: false });

    const restoredProgress = await after.library.repo.loadProgress();
    expect(restoredProgress).toHaveLength(1);
    expect(restoredProgress[0]).toMatchObject({
      projectId: restoredSource.id,
      mode: "outline-infill",
      stepIndex: 6,
      elapsedMs: 91_000,
    });

    expect(await after.library.repo.loadInventory()).toEqual(STOCK);
  });

  it("导出内容对同一份 state 逐字节确定，且不含时间戳", async () => {
    const before = await seeded();
    const once = await exportAll(before.library);
    const twice = await exportAll(before.library);
    expect(once).toBe(twice);
    expect(JSON.parse(once)).not.toHaveProperty("exportedAt");
  });

  it("再导出一次与第一次的归档等价（往返稳定，不逐次漂移）", async () => {
    const before = await seeded();
    const first = await exportAll(before.library);

    const after = library();
    await importInto(after.library, first);
    const second = JSON.parse(await exportAll(after.library)) as {
      projects: { project: { title: string } }[];
    };
    const original = JSON.parse(first) as typeof second;
    expect(second.projects.map((entry) => entry.project.title).sort()).toEqual(
      original.projects.map((entry) => entry.project.title).sort(),
    );
  });

  it("孤儿文档不进归档（D-IE-14：由 projects 数组驱动枚举）", async () => {
    const before = await seeded();
    const orphan = mintProjectId();
    await before.library.repo.savePatternDoc(createPatternDoc(orphan, createGrid(2, 1, [5, null])));

    const parsed = JSON.parse(await exportAll(before.library)) as { projects: unknown[] };
    expect(parsed.projects).toHaveLength(2);
    expect(JSON.stringify(parsed)).not.toContain(orphan);
  });
});

describe("T-IE-9 同一文件导入两次 → 两套独立项目，零覆盖", () => {
  it("项目、文档、游标各成双，id 互不相同", async () => {
    const before = await seeded();
    const text = await exportAll(before.library);

    const after = library();
    await importInto(after.library, text);
    await importInto(after.library, text);

    const projects = await after.library.repo.loadProjects();
    expect(projects).toHaveLength(4);
    expect(new Set(projects.map((project) => project.id)).size).toBe(4);

    const converted = projects.filter((project) => project.sourcePatternId === null);
    expect(converted).toHaveLength(2);
    for (const project of converted) {
      const doc = await after.library.repo.loadPatternDoc(project.id);
      expect(doc).not.toBeNull();
      expect([...doc!.cells]).toEqual([...before.doc.cells]);
    }

    const progress = await after.library.repo.loadProgress();
    expect(progress).toHaveLength(2);
    expect(new Set(progress.map((cursor) => cursor.projectId)).size).toBe(2);
  });
});

describe("T-IE-11 BD20：两个命名空间的 G07 并存互不抵扣", () => {
  it("导入带 generic-5mm G07 的归档，本地画廊 G07 原封不动", async () => {
    const after = library();
    after.library.inventory = [STOCK[0]!];
    await after.library.flush();

    // Hand-written rather than built: the third row is missing its namespace,
    // which is a file the exporter cannot produce but a hand edit can.
    const archive = JSON.stringify({
      format: "beadproj",
      version: 1,
      projects: [],
      inventory: [
        { paletteId: "generic-5mm", code: "G07", name: "Silver", hex: "#b7bfc6", beads: 500 },
        { paletteId: "gallery", code: "G07", name: "苔绿", hex: "#4c7a44", beads: 9_999 },
        { code: "G09", name: "无名", hex: "#123456", beads: 8 },
      ],
    });
    const result = await importInto(after.library, archive);

    expect(result).toMatchObject({
      inventoryAdded: 1,
      inventorySkipped: 1,
      inventoryDropped: 1,
    });
    const stock = await after.library.repo.loadInventory();
    expect(stock).toHaveLength(2);
    expect(stock.find((row) => row.paletteId === "gallery")).toEqual(STOCK[0]);
    expect(stock.find((row) => row.paletteId === "generic-5mm")?.beads).toBe(500);
  });
});

describe("T-IE-12 存储卫生：网格只有 patterns 仓一份", () => {
  it("导入一轮后，state 仓与 localStorage 都不含格子数据", async () => {
    const before = await seeded();
    const text = await exportAll(before.library);

    const after = library();
    await importInto(after.library, text);

    const patterns = await readStoreEntries(after.factory, STORE.patterns);
    expect(patterns).toHaveLength(1);
    expect(patterns[0]!.value).toMatchObject({ width: 3, height: 2 });

    // The `state` store holds one whole array per key and nothing else; an
    // import that had smuggled a grid in beside the metadata would show up here.
    const state = await readStoreEntries(after.factory, STORE.state);
    expect(state.map((row) => row.key).sort()).toEqual(["inventory", "projects"]);
    expect(JSON.stringify(state)).not.toContain("cells");

    // Nothing this work package does writes the legacy blob, and no second
    // grid store appears beside `patterns`.
    expect(after.legacy.getItem(STORAGE_KEY)).toBeNull();
    expect(after.legacy.length).toBe(0);
  });
});

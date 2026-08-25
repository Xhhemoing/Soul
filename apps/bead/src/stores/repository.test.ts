import { describe, expect, it } from "vitest";
import { IDBFactory } from "fake-indexeddb";

import {
  STORAGE_KEY,
  createRepository,
  isInventoryEntry,
  parsePersistedState,
} from "./repository.ts";
import { STATE_KEY, STORE, readStoreEntries } from "./bead-v1.ts";
import { createGrid } from "../algo/grid.ts";
import { asPatternId, mintProjectId } from "./ids.ts";
import { createPatternDoc } from "./patterns.ts";
import { FlakyStorage } from "../test/flaky-storage.ts";
import {
  GALLERY_PALETTE,
  GENERIC_5MM_PALETTE,
  type InventoryEntry,
  type ProgressCursor,
  type Project,
} from "./types.ts";

function project(title = "元宵提灯"): Project {
  return {
    id: mintProjectId(),
    title,
    sourcePatternId: asPatternId("gal-lantern-04"),
    status: "todo",
    createdAt: 7,
    backdrop: "black",
    backdropColor: "#101014",
  };
}

function entry(overrides: Partial<InventoryEntry> = {}): InventoryEntry {
  return {
    paletteId: GALLERY_PALETTE,
    code: "H02",
    name: "薄荷绿",
    hex: "#7fd6a2",
    beads: 200,
    ...overrides,
  };
}

function cursor(owner: Project, overrides: Partial<ProgressCursor> = {}): ProgressCursor {
  return {
    projectId: owner.id,
    mode: "color-by-color",
    stepIndex: 2,
    elapsedMs: 61_000,
    updatedAt: 1_700_000_000_000,
    ...overrides,
  };
}


describe("Repository（D-UI-5：接口全 async）", () => {
  it("项目、收藏、库存分别往返 localStorage", async () => {
    globalThis.localStorage.clear();
    const repo = createRepository();
    const saved = project();

    await repo.saveProjects([saved]);
    await repo.saveFavorites([asPatternId("gal-slime-01")]);
    await repo.saveInventory([entry()]);

    expect(await repo.loadProjects()).toEqual([saved]);
    expect(await repo.loadFavorites()).toEqual(["gal-slime-01"]);
    expect(await repo.loadInventory()).toHaveLength(1);
    expect(globalThis.localStorage.getItem(STORAGE_KEY)).toContain("元宵提灯");
  });

  it("坏数据被丢掉而不是让应用崩掉", () => {
    expect(parsePersistedState(null).projects).toEqual([]);
    expect(parsePersistedState("{ 不是 json").projects).toEqual([]);
    expect(
      parsePersistedState(
        JSON.stringify({ projects: [{ id: "bad-1", title: "x", status: "todo" }], favorites: ["nope"] }),
      ),
    ).toEqual({ projects: [], favorites: [], inventory: [], progress: [] });
  });

  // T-INV-14: 库存条目每个字段都要过检。缺 hex 会让色块拿到 undefined 背景，
  // NaN / 小数颗数会顺着缺口求和一路污染下去。
  it("畸形库存条目被丢掉，好的那条留下（WP-B05）", () => {
    const good = { code: "H02", name: "薄荷绿", hex: "#7fd6a2", beads: 200 };
    const parsed = parsePersistedState(
      JSON.stringify({
        inventory: [
          good,
          { code: "R04", name: "朱红", beads: 10 },
          { code: "Y01", name: "明黄", hex: "#f5d13b", beads: Number.NaN },
          { code: "B05", name: "墨黑", hex: "#1b1b1f", beads: 12.5 },
          { code: "C01", name: "纯白", hex: "#ffffff", beads: -3 },
          { code: "N06", hex: "#c9ccd4", beads: 1 },
          { name: "没有码", hex: "#000000", beads: 1 },
          null,
          "库存",
        ],
      }),
    );
    // BD20 只加了命名空间：没写命名空间的存量条目在这里被盖成 gallery。
    expect(parsed.inventory).toEqual([{ ...good, paletteId: GALLERY_PALETTE }]);
  });

  // T-UP-7 的纯函数一半：盖章只对「没写」生效，「写错」照旧丢掉。
  it("命名空间：缺省盖 gallery，未知值整条丢，已知值原样留", () => {
    const parsed = parsePersistedState(
      JSON.stringify({
        inventory: [
          { code: "H02", name: "薄荷绿", hex: "#7fd6a2", beads: 200 },
          { paletteId: "hama-h", code: "H01", name: "外来", hex: "#000000", beads: 5 },
          { paletteId: GENERIC_5MM_PALETTE, code: "G07", name: "Silver", hex: "#b7bfc6", beads: 9 },
        ],
      }),
    );
    expect(parsed.inventory.map((row) => [row.paletteId, row.code])).toEqual([
      [GALLERY_PALETTE, "H02"],
      [GENERIC_5MM_PALETTE, "G07"],
    ]);
    expect(isInventoryEntry(entry())).toBe(true);
    expect(isInventoryEntry({ ...entry(), paletteId: "hama-h" })).toBe(false);
    expect(isInventoryEntry({ code: "H02", name: "薄荷绿", hex: "#7fd6a2", beads: 200 })).toBe(
      false,
    );
  });

  it("缺 backdrop 或 backdrop 不是预设的记录被丢掉", () => {
    const withoutBackdrop: Partial<Project> = project();
    delete withoutBackdrop.backdrop;
    const wrongKind = { ...project(), backdrop: "rainbow" };
    const wrongColor = { ...project(), backdropColor: 0 };

    expect(
      parsePersistedState(JSON.stringify({ projects: [withoutBackdrop, wrongKind, wrongColor] })).projects,
    ).toEqual([]);
    expect(parsePersistedState(JSON.stringify({ projects: [project()] })).projects).toHaveLength(1);
  });
});

describe("步游标第四键（BD19 / T-ASM-6）", () => {
  it("往返 localStorage，并跟着项目一起落在同一个 blob 里", async () => {
    globalThis.localStorage.clear();
    const repo = createRepository();
    const owner = project();

    await repo.saveProjects([owner]);
    await repo.saveProgress([cursor(owner)]);

    expect(await repo.loadProgress()).toEqual([cursor(owner)]);
    expect(await repo.loadProjects()).toEqual([owner]);
  });

  it("畸形条目被丢掉，不带崩整份 blob", () => {
    const owner = project();
    const parsed = parsePersistedState(
      JSON.stringify({
        projects: [owner],
        progress: [
          cursor(owner),
          { ...cursor(owner), projectId: "gal-slime-01" },
          { ...cursor(owner), mode: "spiral" },
          { ...cursor(owner), stepIndex: -1 },
          { ...cursor(owner), elapsedMs: Number.NaN },
          { ...cursor(owner), updatedAt: "刚刚" },
          null,
          "游标",
        ],
      }),
    );
    expect(parsed.progress).toEqual([cursor(owner)]);
  });

  it("孤儿游标被剪除", () => {
    const owner = project();
    const gone = project("已删掉的项目");
    const parsed = parsePersistedState(
      JSON.stringify({ projects: [owner], progress: [cursor(owner), cursor(gone)] }),
    );
    expect(parsed.progress.map((entry) => entry.projectId)).toEqual([owner.id]);
  });

  it("同一项目多条时取 updatedAt 最大的那条", () => {
    const owner = project();
    const parsed = parsePersistedState(
      JSON.stringify({
        projects: [owner],
        progress: [
          cursor(owner, { stepIndex: 1, updatedAt: 10 }),
          cursor(owner, { stepIndex: 9, updatedAt: 30 }),
          cursor(owner, { stepIndex: 4, updatedAt: 20 }),
        ],
      }),
    );
    expect(parsed.progress).toHaveLength(1);
    expect(parsed.progress[0]?.stepIndex).toBe(9);
  });
});

describe("BD19 红线：落盘的游标只有五个键（T-ASM-7）", () => {
  it("多余的键在写盘前就被剥掉，网格与步数组进不了 blob", async () => {
    globalThis.localStorage.clear();
    const repo = createRepository();
    const owner = project();
    await repo.saveProjects([owner]);

    const smuggled = {
      ...cursor(owner),
      cells: [1, 2, 3],
      steps: [{ cells: [] }],
      doneBits: "1010",
      grid: { width: 28, height: 28 },
    } as unknown as ProgressCursor;
    await repo.saveProgress([smuggled]);

    const raw = globalThis.localStorage.getItem(STORAGE_KEY) ?? "";
    const stored = (JSON.parse(raw) as { progress: Record<string, unknown>[] }).progress;
    expect(stored).toHaveLength(1);
    expect(Object.keys(stored[0] ?? {}).sort()).toEqual([
      "elapsedMs",
      "mode",
      "projectId",
      "stepIndex",
      "updatedAt",
    ]);
    for (const banned of ["cells", "steps", "doneBits", "grid"]) {
      expect(raw).not.toContain(banned);
    }
  });
});

describe("DATA-1：写失败不得静默降级", () => {
  it("写失败后读跟着走内存副本，不再回旧 localStorage", async () => {
    const storage = new FlakyStorage();
    const repo = createRepository(storage);
    const first = project("落地的那张");
    const second = project("配额满之后的那张");

    await repo.saveProjects([first]);
    storage.full = true;
    await repo.saveProjects([first, second]);

    // 旧行为：这里回 [first]——写进内存的那张凭空消失，且毫无提示。
    expect(await repo.loadProjects()).toEqual([first, second]);
  });

  it("失败后的 read-modify-write 不会把丢掉的写重新覆盖回去", async () => {
    const storage = new FlakyStorage();
    const repo = createRepository(storage);
    const kept = project("失败后新增");

    await repo.saveProjects([]);
    storage.full = true;
    await repo.saveProjects([kept]);
    await repo.saveFavorites([asPatternId("gal-slime-01")]);

    expect(await repo.loadProjects()).toEqual([kept]);
    expect(await repo.loadFavorites()).toEqual(["gal-slime-01"]);
    expect(storage.getItem(STORAGE_KEY)).not.toContain("失败后新增");
  });

  it("失败作为信号暴露出来，且只通知一次", async () => {
    const storage = new FlakyStorage();
    const repo = createRepository(storage);
    const notifications: number[] = [];
    const unsubscribe = repo.subscribeToPersistence(() => notifications.push(1));

    expect(repo.isPersistenceFailed()).toBe(false);

    storage.full = true;
    await repo.saveProjects([project()]);
    await repo.saveInventory([entry()]);

    expect(repo.isPersistenceFailed()).toBe(true);
    expect(notifications).toHaveLength(1);

    unsubscribe();
    expect(repo.isPersistenceFailed()).toBe(true);
  });

  it("完全没有 localStorage 的嵌入环境一开始就报告失败", async () => {
    expect(createRepository().isPersistenceFailed()).toBe(false); // jsdom 有 localStorage

    const repo = createRepositoryWithoutLocalStorage();
    expect(repo.isPersistenceFailed()).toBe(true);
    await repo.saveProjects([project()]);
    expect(await repo.loadProjects()).toHaveLength(1); // 会话内可用，只是不落盘
  });
});

// ---------------------------------------------------------------------------
// bead-v1（round2-data §5，BD19 的触发点就是本 PR）。每个用例自带一个
// `IDBFactory`，所以库之间互不串味；legacy 源用 FlakyStorage 而不是 jsdom 的
// localStorage，免得迁移把别的用例的键吃掉。

function beadV1(legacy: Storage = new FlakyStorage()) {
  const factory = new IDBFactory();
  return { factory, legacy, repo: createRepository(legacy, factory) };
}

function cursorRow(owner: Project, overrides: Partial<ProgressCursor> = {}): ProgressCursor {
  return cursor(owner, overrides);
}

describe("T-UP-1 bead-v1 往返（§5.1 / §5.2）", () => {
  it("PatternDoc 的 Int16Array 原样回来，网格不变", async () => {
    const { repo } = beadV1();
    const owner = project();
    const grid = createGrid(3, 2, [0, 1, null, 47, null, 2]);
    const doc = createPatternDoc(owner.id, grid, { kind: "PixelArt", ditherApplied: false });

    await repo.savePatternDoc(doc);
    const back = await repo.loadPatternDoc(owner.id);

    expect(back?.cells).toBeInstanceOf(Int16Array);
    expect([...(back?.cells ?? [])]).toEqual([0, 1, -1, 47, -1, 2]);
    expect(back).toEqual(doc);
    expect(repo.isPersistenceFailed()).toBe(false);
  });

  it("三个整数组与游标各回各仓，六个老方法形状不变", async () => {
    const { repo } = beadV1();
    const owner = project();

    await repo.saveProjects([owner]);
    await repo.saveFavorites([asPatternId("gal-slime-01")]);
    await repo.saveInventory([entry()]);
    await repo.saveProgress([cursorRow(owner)]);

    expect(await repo.loadProjects()).toEqual([owner]);
    expect(await repo.loadFavorites()).toEqual(["gal-slime-01"]);
    expect(await repo.loadInventory()).toEqual([entry()]);
    expect(await repo.loadProgress()).toEqual([cursorRow(owner)]);
  });

  it("没有文档的项目读回 null，不是抛错", async () => {
    const { repo } = beadV1();
    expect(await repo.loadPatternDoc(project().id)).toBeNull();
  });
});

describe("T-UP-2 / T-UP-7 迁移（§5.4 + §2.1 对账）", () => {
  const legacyBlob = (owner: Project) =>
    JSON.stringify({
      projects: [owner],
      favorites: ["gal-slime-01"],
      // 存量条目没有 paletteId：D-INV-3 说 v0 录入的全是画廊码。
      inventory: [{ code: "R04", name: "朱红", hex: "#d8412f", beads: 60 }],
      progress: [cursorRow(owner)],
    });

  it("四个键搬成三数组 + 逐条游标，原键删掉", async () => {
    const owner = project();
    const legacy = new FlakyStorage();
    legacy.setItem(STORAGE_KEY, legacyBlob(owner));
    const { factory, repo } = beadV1(legacy);

    expect(await repo.loadProjects()).toEqual([owner]);
    expect(legacy.getItem(STORAGE_KEY)).toBeNull();

    const state = await readStoreEntries(factory, STORE.state);
    expect(state.map((row) => row.key).sort()).toEqual([
      STATE_KEY.favorites,
      STATE_KEY.inventory,
      STATE_KEY.projects,
    ]);

    // 游标不进 state 仓：它逐条落在 progress 仓，keyPath 就是 projectId。
    const progress = await readStoreEntries(factory, STORE.progress);
    expect(progress).toEqual([{ key: owner.id, value: cursorRow(owner) }]);
  });

  it("存量库存条目被盖上 gallery", async () => {
    const owner = project();
    const legacy = new FlakyStorage();
    legacy.setItem(STORAGE_KEY, legacyBlob(owner));
    const { repo } = beadV1(legacy);

    expect(await repo.loadInventory()).toEqual([
      { paletteId: GALLERY_PALETTE, code: "R04", name: "朱红", hex: "#d8412f", beads: 60 },
    ]);
  });

  it("再开一次不重复迁：迁移后的改动不会被旧 blob 盖回去", async () => {
    const owner = project();
    const legacy = new FlakyStorage();
    legacy.setItem(STORAGE_KEY, legacyBlob(owner));
    const first = beadV1(legacy);

    await first.repo.loadProjects();
    await first.repo.saveProjects([]);

    const second = createRepository(legacy, first.factory);
    expect(await second.loadProjects()).toEqual([]);
    expect(await second.loadFavorites()).toEqual(["gal-slime-01"]);
  });

  it("没有旧键时开库是空的，不凭空造记录", async () => {
    const { repo } = beadV1();
    expect(await repo.loadProjects()).toEqual([]);
    expect(await repo.loadProgress()).toEqual([]);
  });
});

describe("T-UP-3 deleteProject 级联（§5.3 唯一跨仓不变量）", () => {
  it("一笔事务里同时删掉 patterns 与 progress，别的项目不受影响", async () => {
    const { factory, repo } = beadV1();
    const doomed = project("要删的");
    const kept = project("留着的");

    await repo.saveProjects([doomed, kept]);
    await repo.savePatternDoc(createPatternDoc(doomed.id, createGrid(2, 1, [0, 1])));
    await repo.savePatternDoc(createPatternDoc(kept.id, createGrid(2, 1, [2, 3])));
    await repo.saveProgress([cursorRow(doomed), cursorRow(kept)]);

    await repo.deleteProject(doomed.id);

    expect(await repo.loadPatternDoc(doomed.id)).toBeNull();
    expect(await repo.loadPatternDoc(kept.id)).not.toBeNull();
    expect((await repo.loadProgress()).map((row) => row.projectId)).toEqual([kept.id]);
    expect(
      (await readStoreEntries(factory, STORE.patterns)).map((row) => row.key),
    ).toEqual([kept.id]);
  });
});

describe("T-UP-4 打开失败（DATA-1 在 IDB 上重申）", () => {
  const brokenFactory = {
    open() {
      throw new DOMException("拒绝打开", "SecurityError");
    },
  } as unknown as IDBFactory;

  it("失败可观察，读写落到内存视图，会话内照常可用", async () => {
    const repo = createRepository(new FlakyStorage(), brokenFactory);
    const notifications: number[] = [];
    repo.subscribeToPersistence(() => notifications.push(1));

    expect(await repo.loadProjects()).toEqual([]);
    expect(repo.isPersistenceFailed()).toBe(true);
    expect(notifications).toHaveLength(1);

    const seeded = project();
    await repo.saveProjects([seeded]);
    await repo.saveProgress([cursorRow(seeded)]);
    expect(await repo.loadProjects()).toEqual([seeded]);
    expect(await repo.loadProgress()).toEqual([cursorRow(seeded)]);
  });

  it("savePatternDoc 拒绝而不是假装成功（D-UP-10 在等这一下）", async () => {
    const repo = createRepository(new FlakyStorage(), brokenFactory);
    const owner = project();
    await expect(
      repo.savePatternDoc(createPatternDoc(owner.id, createGrid(1, 1, [0]))),
    ).rejects.toThrow();
    expect(repo.isPersistenceFailed()).toBe(true);
  });

  it("没有 IndexedDB 的宿主退回 localStorage 骨架，且明说存不下网格", async () => {
    const legacy = new FlakyStorage();
    const repo = createRepository(legacy, null);
    const owner = project();

    await repo.saveProjects([owner]);
    expect(legacy.getItem(STORAGE_KEY)).toContain(owner.id);

    await expect(
      repo.savePatternDoc(createPatternDoc(owner.id, createGrid(1, 1, [0]))),
    ).rejects.toThrow();
    // BD19 红线：网格无论如何都不进 bead.state。
    expect(legacy.getItem(STORAGE_KEY)).not.toContain("cells");
    expect(repo.isPersistenceFailed()).toBe(true);
  });
});

describe("T-UP-5 读回校验（仓里的坏文档不带崩页面）", () => {
  it("手改成越界下标的文档读回 null", async () => {
    const { factory, repo } = beadV1();
    const owner = project();
    await repo.savePatternDoc(createPatternDoc(owner.id, createGrid(2, 1, [0, 1])));

    const db = await new Promise<IDBDatabase>((resolve, reject) => {
      const request = factory.open("bead-v1", 1);
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    await new Promise<void>((resolve, reject) => {
      const tx = db.transaction(STORE.patterns, "readwrite");
      tx.objectStore(STORE.patterns).put({
        projectId: owner.id,
        paletteId: GENERIC_5MM_PALETTE,
        width: 2,
        height: 1,
        cells: new Int16Array([0, 9999]),
      });
      tx.oncomplete = () => resolve();
      tx.onerror = () => reject(tx.error);
    });
    db.close();

    expect(await repo.loadPatternDoc(owner.id)).toBeNull();
  });
});

describe("T-UP-6 saveProgress 数组签名在 progress 仓上的剪枝", () => {
  it("被剪掉的 projectId 连同它的行一起消失", async () => {
    const { factory, repo } = beadV1();
    const owner = project();
    const gone = project("后来删掉的");

    await repo.saveProjects([owner, gone]);
    await repo.saveProgress([cursorRow(owner), cursorRow(gone)]);
    expect(await repo.loadProgress()).toHaveLength(2);

    // 项目没了，游标就该没：这正是 sanitizeProgress 的语义，换仓不换意思。
    await repo.saveProjects([owner]);
    await repo.saveProgress([cursorRow(owner), cursorRow(gone)]);

    expect((await repo.loadProgress()).map((row) => row.projectId)).toEqual([owner.id]);
    expect((await readStoreEntries(factory, STORE.progress)).map((row) => row.key)).toEqual([
      owner.id,
    ]);
  });

  it("同一项目多条时仍取 updatedAt 最大的那条，仓里只留一行", async () => {
    const { repo } = beadV1();
    const owner = project();
    await repo.saveProjects([owner]);
    await repo.saveProgress([
      cursorRow(owner, { stepIndex: 1, updatedAt: 10 }),
      cursorRow(owner, { stepIndex: 9, updatedAt: 30 }),
    ]);

    const stored = await repo.loadProgress();
    expect(stored).toHaveLength(1);
    expect(stored[0]?.stepIndex).toBe(9);
  });
});

describe("T-UP-15 落盘键集：派生数据一个都没进去", () => {
  it("保存转换文档后，四个仓里没有 Step[]、没有 BOM、没有第二份网格", async () => {
    const { factory, repo } = beadV1();
    const owner = project();
    const grid = createGrid(4, 2, [0, 1, 2, 3, null, 4, 5, null]);

    await repo.saveProjects([owner]);
    await repo.savePatternDoc(
      createPatternDoc(owner.id, grid, { kind: "Photo", ditherApplied: true }),
    );
    await repo.saveProgress([cursorRow(owner)]);

    const patterns = await readStoreEntries(factory, STORE.patterns);
    expect(patterns).toHaveLength(1);
    expect(Object.keys(patterns[0]!.value as object).sort()).toEqual([
      "cells",
      "height",
      "paletteId",
      "projectId",
      "provenance",
      "width",
    ]);

    const progress = await readStoreEntries(factory, STORE.progress);
    expect(Object.keys(progress[0]!.value as object).sort()).toEqual([
      "elapsedMs",
      "mode",
      "projectId",
      "stepIndex",
      "updatedAt",
    ]);

    const state = await readStoreEntries(factory, STORE.state);
    const everything = JSON.stringify([state, progress]);
    for (const banned of ["cells", "steps", "bom", "doneBits", "grid"]) {
      expect(everything).not.toContain(banned);
    }
  });
});

/** `globalThis.localStorage` 缺席的嵌入环境。 */
function createRepositoryWithoutLocalStorage() {
  const original = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: undefined });
  try {
    return createRepository();
  } finally {
    if (original) Object.defineProperty(globalThis, "localStorage", original);
    else Reflect.deleteProperty(globalThis, "localStorage");
  }
}

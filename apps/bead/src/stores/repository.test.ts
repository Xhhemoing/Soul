import { describe, expect, it } from "vitest";

import { STORAGE_KEY, createRepository, parsePersistedState } from "./repository.ts";
import { asPatternId, mintProjectId } from "./ids.ts";
import { FlakyStorage } from "../test/flaky-storage.ts";
import type { ProgressCursor, Project } from "./types.ts";

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
    await repo.saveInventory([{ code: "H02", name: "薄荷绿", hex: "#7fd6a2", beads: 200 }]);

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
    await repo.saveInventory([{ code: "H02", name: "薄荷绿", hex: "#7fd6a2", beads: 200 }]);

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

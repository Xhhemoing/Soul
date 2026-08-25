import { describe, expect, it } from "vitest";

import { STORAGE_KEY, createRepository, parsePersistedState } from "./repository.ts";
import { asPatternId, mintProjectId } from "./ids.ts";
import type { Project } from "./types.ts";

function project(): Project {
  return {
    id: mintProjectId(),
    title: "元宵提灯",
    sourcePatternId: asPatternId("gal-lantern-04"),
    status: "todo",
    createdAt: 7,
    backdrop: "black",
    backdropColor: "#101014",
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
    ).toEqual({ projects: [], favorites: [], inventory: [] });
  });
});

import { describe, expect, it } from "vitest";

import {
  ID_PREFIX,
  asPatternId,
  asProjectId,
  isCreatorId,
  isPatternId,
  isProjectId,
  mintProjectId,
} from "./ids.ts";
import { CREATORS, PATTERNS } from "../fixtures/catalog.ts";
import { createProjectFromPattern } from "./projects.ts";

// R1: the prefixes are pinned here so a later work package cannot drift them
// without a red test.
describe("id 命名空间前缀", () => {
  it("前缀是 gal- / proj- / cr-", () => {
    expect(ID_PREFIX).toEqual({ pattern: "gal-", project: "proj-", creator: "cr-" });
  });

  it("fixture 里的图纸与创作者 id 都带前缀", () => {
    for (const pattern of PATTERNS) {
      expect(isPatternId(pattern.id)).toBe(true);
      expect(isProjectId(pattern.id)).toBe(false);
    }
    for (const creator of CREATORS) {
      expect(isCreatorId(creator.id)).toBe(true);
    }
  });

  it("新建项目 id 一律 proj- 开头且互不相同", () => {
    const first = mintProjectId(1);
    const second = mintProjectId(1);
    expect(isProjectId(first)).toBe(true);
    expect(isProjectId(second)).toBe(true);
    expect(first).not.toBe(second);
  });

  it("从图纸实例化出来的项目带 proj- 前缀并记住来源图纸", () => {
    const pattern = PATTERNS[0];
    expect(pattern).toBeDefined();
    if (!pattern) return;
    const project = createProjectFromPattern(pattern, "todo");
    expect(isProjectId(project.id)).toBe(true);
    expect(project.sourcePatternId).toBe(pattern.id);
    expect(isPatternId(project.sourcePatternId ?? "")).toBe(true);
  });

  it("裸前缀和错前缀都不算合法 id", () => {
    expect(isPatternId("gal-")).toBe(false);
    expect(isProjectId("proj-")).toBe(false);
    expect(isCreatorId("cr-")).toBe(false);
    expect(isProjectId("gal-slime-01")).toBe(false);
    expect(isPatternId("proj-abc")).toBe(false);
    expect(() => asProjectId("gal-slime-01")).toThrow(/proj-/);
    expect(() => asPatternId("cr-mira")).toThrow(/gal-/);
  });
});

import { describe, expect, it } from "vitest";
import { screen } from "@testing-library/react";

import { renderApp } from "../test/render.tsx";
import { mintProjectId } from "../stores/ids.ts";
import { THEME_STORAGE_KEY } from "../stores/theme.tsx";
import type { Project } from "../stores/types.ts";

function project(overrides: Partial<Project> = {}): Project {
  return {
    id: mintProjectId(),
    title: "夏日鸟居",
    sourcePatternId: null,
    status: "active",
    createdAt: 1,
    backdrop: "black",
    backdropColor: "#101014",
    ...overrides,
  };
}

describe("主题 token", () => {
  it("切换主题写 data-theme 并持久化", async () => {
    const { user } = renderApp({ theme: "light" });
    expect(document.documentElement.dataset["theme"]).toBe("light");

    await user.click(await screen.findByRole("button", { name: "浅色主题" }));
    expect(document.documentElement.dataset["theme"]).toBe("dark");
    expect(globalThis.localStorage.getItem(THEME_STORAGE_KEY)).toBe("dark");

    await user.click(screen.getByRole("button", { name: "深色主题" }));
    expect(document.documentElement.dataset["theme"]).toBe("light");
  });
});

describe("拼装背景与应用主题互不影响（D-UI-6）", () => {
  it("背景读项目记录，不读 data-theme", async () => {
    const seeded = project({ backdrop: "white" });
    renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] }, theme: "dark" });

    const backdrop = await screen.findByTestId("assemble-backdrop");
    expect(backdrop).toHaveAttribute("data-backdrop", "white");
    expect(document.documentElement.dataset["theme"]).toBe("dark");
  });

  it("改背景是逐项目的，不动应用主题", async () => {
    const seeded = project({ backdrop: "black" });
    const { user } = renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded] },
      theme: "light",
    });

    await user.click(await screen.findByRole("radio", { name: "白色背景" }));
    expect(screen.getByTestId("assemble-backdrop")).toHaveAttribute("data-backdrop", "white");
    expect(document.documentElement.dataset["theme"]).toBe("light");
  });

  it("自定义背景色存进项目记录", async () => {
    const seeded = project({ backdrop: "black" });
    const { user, repository } = renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded] },
    });

    await user.click(await screen.findByRole("radio", { name: "自定义背景" }));
    const stored = await repository.loadProjects();
    expect(stored[0]?.backdrop).toBe("custom");
  });
});

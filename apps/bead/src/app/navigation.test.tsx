import { describe, expect, it } from "vitest";
import { screen, waitFor, within } from "@testing-library/react";

import { currentPath, renderApp } from "../test/render.tsx";
import { mintProjectId } from "../stores/ids.ts";
import type { Project } from "../stores/types.ts";

function seedProject(overrides: Partial<Project> = {}): Project {
  return {
    id: mintProjectId(),
    title: "史莱姆小队",
    sourcePatternId: null,
    status: "active",
    createdAt: 1,
    backdrop: "black",
    backdropColor: "#101014",
    ...overrides,
  };
}

describe("底栏导航", () => {
  it("四个入口都在，且「+」有可读名称", async () => {
    renderApp();
    const nav = await screen.findByRole("navigation", { name: "主导航" });
    for (const name of ["灵感", "拼装台", "创作与导入", "资产"]) {
      expect(within(nav).getByRole("link", { name })).toBeInTheDocument();
    }
    expect(within(nav).getByText("+")).toBeInTheDocument();
  });

  it("激活项标记 aria-current", async () => {
    const { router } = renderApp({ route: "/workspace" });
    const nav = await screen.findByRole("navigation", { name: "主导航" });
    expect(within(nav).getByRole("link", { name: "拼装台" })).toHaveAttribute("aria-current", "page");
    expect(within(nav).getByRole("link", { name: "灵感" })).not.toHaveAttribute("aria-current");
    expect(currentPath(router)).toBe("/workspace");
  });

  it("点击底栏可以在四条主路由之间来回", async () => {
    const { user, router } = renderApp();
    const nav = await screen.findByRole("navigation", { name: "主导航" });

    await user.click(within(nav).getByRole("link", { name: "拼装台" }));
    expect(currentPath(router)).toBe("/workspace");
    expect(await screen.findByRole("heading", { level: 1, name: "拼装台" })).toBeInTheDocument();

    await user.click(within(nav).getByRole("link", { name: "创作与导入" }));
    expect(currentPath(router)).toBe("/create");
    expect(await screen.findByRole("heading", { level: 1, name: "创作与导入" })).toBeInTheDocument();

    await user.click(within(nav).getByRole("link", { name: "资产" }));
    expect(currentPath(router)).toBe("/inventory");
    expect(await screen.findByRole("heading", { level: 1, name: "资产" })).toBeInTheDocument();

    await user.click(within(nav).getByRole("link", { name: "灵感" }));
    expect(currentPath(router)).toBe("/explore");
    expect(await screen.findByRole("heading", { level: 1, name: "灵感" })).toBeInTheDocument();
  });
});

describe("路由表", () => {
  it("/ 重定向到 /explore", async () => {
    const { router } = renderApp({ route: "/" });
    await waitFor(() => expect(currentPath(router)).toBe("/explore"));
    expect(await screen.findByRole("heading", { level: 1, name: "灵感" })).toBeInTheDocument();
  });

  it("/pattern/:id 渲染画廊图纸真实内容", async () => {
    renderApp({ route: "/pattern/gal-slime-01" });
    expect(await screen.findByRole("heading", { level: 1, name: "史莱姆小队" })).toBeInTheDocument();
    // D-ASM-2: the grids fixture is the authority for this count.
    expect(screen.getByText("300")).toBeInTheDocument();
    expect(screen.getByText(/H02 薄荷绿/)).toBeInTheDocument();
  });

  it("/creator/:id 渲染创作者与作品集", async () => {
    renderApp({ route: "/creator/cr-mira" });
    expect(await screen.findByRole("heading", { level: 1, name: "Mira 拼豆铺" })).toBeInTheDocument();
    expect(screen.getByText(/被拼打卡数：128/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: /史莱姆小队/ })).toBeInTheDocument();
  });

  it("未知路由落到兜底页", async () => {
    renderApp({ route: "/nope" });
    expect(await screen.findByRole("heading", { level: 1, name: "页面不存在" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "回到灵感" })).toBeInTheDocument();
  });

  it("每条路由有自己的标题", async () => {
    renderApp({ route: "/inventory" });
    await waitFor(() => expect(document.title).toBe("资产 · BeadFlow"));
  });
});

describe("双 chrome", () => {
  it("常规路由在 AppShell 内", async () => {
    renderApp({ route: "/explore" });
    expect(await screen.findByRole("navigation", { name: "主导航" })).toBeInTheDocument();
  });

  it("/assemble/:id 在 AppShell 之外渲染，且有退出控件", async () => {
    const project = seedProject();
    const { user, router } = renderApp({
      route: `/assemble/${project.id}`,
      seed: { projects: [project] },
    });

    expect(await screen.findByRole("heading", { level: 1, name: "史莱姆小队" })).toBeInTheDocument();
    expect(screen.queryByRole("navigation", { name: "主导航" })).not.toBeInTheDocument();

    await user.click(screen.getByRole("link", { name: "退出拼装" }));
    await waitFor(() => expect(currentPath(router)).toBe("/workspace"));
    expect(await screen.findByRole("navigation", { name: "主导航" })).toBeInTheDocument();
  });

  it("Escape 也能退出沉浸页", async () => {
    const project = seedProject();
    const { user, router } = renderApp({
      route: `/assemble/${project.id}`,
      seed: { projects: [project] },
    });
    await screen.findByRole("heading", { level: 1, name: "史莱姆小队" });
    await user.keyboard("{Escape}");
    await waitFor(() => expect(currentPath(router)).toBe("/workspace"));
  });
});

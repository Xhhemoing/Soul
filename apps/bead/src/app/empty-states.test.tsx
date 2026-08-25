import { describe, expect, it } from "vitest";
import { screen, within } from "@testing-library/react";

import { renderApp } from "../test/render.tsx";
import { asPatternId, mintProjectId } from "../stores/ids.ts";
import type { Project } from "../stores/types.ts";

// Section 3 of the round-1 IA review is a test list; this file is that list.

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

describe("个人条零态", () => {
  it("没有正在拼的项目时给一句解释和主 CTA", async () => {
    renderApp();
    const strip = await screen.findByRole("region", { name: "个人条" });
    expect(within(strip).getByText("还没有正在拼的项目")).toBeInTheDocument();
    expect(within(strip).getByRole("link", { name: "开始一个项目" })).toHaveAttribute("href", "/create");
  });

  it("待办与收藏计数为 0 时仍然显示，不隐藏", async () => {
    renderApp();
    const strip = await screen.findByRole("region", { name: "个人条" });
    const todo = within(strip).getByRole("link", { name: /待办/ });
    const favorites = within(strip).getByRole("link", { name: /收藏/ });
    expect(todo).toHaveTextContent("0");
    expect(favorites).toHaveTextContent("0");
    expect(favorites).toHaveAttribute("href", "/explore?fav=1");
  });

  it("有项目时进度卡指向该项目的拼装页", async () => {
    const seeded = project();
    renderApp({ seed: { projects: [seeded] } });
    const strip = await screen.findByRole("region", { name: "个人条" });
    expect(within(strip).getByRole("link", { name: /夏日鸟居/ })).toHaveAttribute(
      "href",
      `/assemble/${seeded.id}`,
    );
  });
});

describe("Explore 发现流零态", () => {
  it("标签筛选后为空时给清除筛选", async () => {
    renderApp({ route: "/explore?tag=不存在的标签" });
    expect(await screen.findByText("没有匹配的图纸")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "清除筛选" })).toHaveAttribute("href", "/explore");
  });

  it("收藏筛选后为空时同样给清除筛选", async () => {
    renderApp({ route: "/explore?fav=1" });
    expect(await screen.findByText("没有匹配的图纸")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "清除筛选" })).toBeInTheDocument();
  });

  it("收藏过的图纸会出现在收藏筛选里", async () => {
    renderApp({ route: "/explore?fav=1", seed: { favorites: [asPatternId("gal-torii-02")] } });
    expect(await screen.findByRole("link", { name: /夏日鸟居/ })).toBeInTheDocument();
    expect(screen.queryByText("没有匹配的图纸")).not.toBeInTheDocument();
  });
});

describe("Workspace 三段零态", () => {
  it("三段各自给一句解释", async () => {
    renderApp({ route: "/workspace" });
    const inProgress = await screen.findByRole("region", { name: "正在拼" });
    expect(within(inProgress).getByText(/还没有正在拼的项目/)).toBeInTheDocument();
    expect(within(inProgress).getByRole("link", { name: "去灵感挑图纸" })).toHaveAttribute(
      "href",
      "/explore",
    );
    expect(within(inProgress).getByRole("link", { name: "新建项目" })).toHaveAttribute("href", "/create");

    const drafts = screen.getByRole("region", { name: "草稿与设计" });
    expect(within(drafts).getByText(/草稿是还没开拼的自定义图纸/)).toBeInTheDocument();

    const done = screen.getByRole("region", { name: "历史完工" });
    expect(within(done).getByText(/拼完的项目会连同用时一起收在这里/)).toBeInTheDocument();
  });

  it("三段按项目状态分流", async () => {
    renderApp({
      route: "/workspace",
      seed: {
        projects: [
          project({ title: "进行中的图", status: "active" }),
          project({ title: "草稿图", status: "draft" }),
          project({ title: "完工图", status: "done" }),
        ],
      },
    });
    const inProgress = await screen.findByRole("region", { name: "正在拼" });
    expect(within(inProgress).getByText("进行中的图")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "草稿与设计" })).getByText("草稿图")).toBeInTheDocument();
    expect(within(screen.getByRole("region", { name: "历史完工" })).getByText("完工图")).toBeInTheDocument();
  });
});

describe("Inventory 零态", () => {
  it("没有库存时说明为什么要先录入", async () => {
    renderApp({ route: "/inventory" });
    expect(await screen.findByText("录入库存后才能做缺口预警")).toBeInTheDocument();
    expect(screen.getByText(/库存录入与编辑归 WP-B05/)).toBeInTheDocument();
  });
});

describe("详情页守卫", () => {
  it("未知图纸 id 提示已下架并给返回", async () => {
    renderApp({ route: "/pattern/gal-unknown" });
    expect(await screen.findByText("图纸不存在或已下架")).toBeInTheDocument();
    expect(screen.getAllByRole("link", { name: "返回灵感" }).length).toBeGreaterThan(0);
  });

  it("未知创作者 id 提示不存在", async () => {
    renderApp({ route: "/creator/cr-unknown" });
    expect(await screen.findByText("创作者不存在")).toBeInTheDocument();
  });

  it("未知项目 id 显示守卫屏而不是自动跳转", async () => {
    const { router } = renderApp({ route: "/assemble/proj-unknown" });
    expect(await screen.findByRole("heading", { level: 1, name: "无法进入拼装" })).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "退出到拼装台" })).toBeInTheDocument();
    expect(router.state.location.pathname).toBe("/assemble/proj-unknown");
  });
});

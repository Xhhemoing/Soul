import { describe, expect, it } from "vitest";
import { screen, within } from "@testing-library/react";

import { CREATORS, PATTERNS } from "../../fixtures/catalog.ts";
import { fixtureGridFor } from "../../fixtures/grids.ts";
import { asPatternId, isProjectId } from "../../stores/ids.ts";
import { toPatternDoc } from "../../stores/patterns.ts";
import { createInMemoryRepository, type Repository } from "../../stores/repository.ts";
import { currentPath, renderApp } from "../../test/render.tsx";
import { FORK_SAVE_ERROR, FORK_UNAVAILABLE_NOTE } from "./PatternDetailPage.tsx";

/**
 * T-GAL-3…8: 「Fork 改色」driven through the real route table.
 *
 * 团子三兄弟 is the pattern under test wherever the size does not matter: it is
 * a 28×28 board with five colours, so a case that walks it into the editor and
 * on into an assembly session stays inside a second.
 */
const MOCHI = "gal-mochi-07";
const MOCHI_TITLE = "团子三兄弟";
/** D-GAL-6 pins these; `fork.test.ts` holds the whole table. */
const MOCHI_BEADS = 138;

function forkButton(): HTMLElement {
  return screen.getByRole("button", { name: "Fork 改色" });
}

describe("T-GAL-3 Fork 一键（D-GAL-7 / D-GAL-8 / D-GAL-9）", () => {
  it("文档先落仓、项目后 mint、落在 /edit/:id 且守卫放行", async () => {
    const repository = createInMemoryRepository();
    const { user, router } = renderApp({ route: `/pattern/${MOCHI}`, repository });

    await user.click(await screen.findByRole("button", { name: "Fork 改色" }));
    await screen.findByTestId("editor-grid");

    const projects = await repository.loadProjects();
    expect(projects).toHaveLength(1);
    const project = projects[0]!;
    expect(project).toMatchObject({
      title: `${MOCHI_TITLE}（Fork）`,
      status: "draft",
      // D-GAL-8: 非 null 会让编辑器关门、让拼装读回 fixture 原色、让 B07 拒收。
      sourcePatternId: null,
    });
    expect(isProjectId(project.id)).toBe(true);
    expect(currentPath(router)).toBe(`/edit/${project.id}`);

    const doc = await repository.loadPatternDoc(project.id);
    expect(doc).toMatchObject({ paletteId: "generic-5mm", width: 28, height: 28 });
    expect(doc!.cells).toHaveLength(28 * 28);
    expect([...doc!.cells].filter((cell) => cell !== -1)).toHaveLength(MOCHI_BEADS);
    // 读回校验（D-UP-12）：量化值域在 [0,48)，整篇文档过得去。
    expect(toPatternDoc(doc)).not.toBeNull();
    // D-GAL-9: fork 既非 PixelArt 也非 Photo，不带 provenance。
    expect(doc!.provenance).toBeUndefined();
  });

  it("网格搬过来的是原图形状，不是空板", async () => {
    const repository = createInMemoryRepository();
    const { user } = renderApp({ route: `/pattern/${MOCHI}`, repository });

    await user.click(await screen.findByRole("button", { name: "Fork 改色" }));
    await screen.findByTestId("editor-grid");

    const fixture = fixtureGridFor(asPatternId(MOCHI))!;
    const doc = (await repository.loadPatternDoc((await repository.loadProjects())[0]!.id))!;
    expect([...doc.cells].map((cell) => cell === -1)).toEqual(
      fixture.grid.cells.map((cell) => cell === null),
    );
  });
});

describe("T-GAL-4 写盘失败（D-UP-10 / D-UP-16 的镜像）", () => {
  it("savePatternDoc 注入 reject → 内联错误，不 mint、不导航", async () => {
    const backing = createInMemoryRepository();
    const repository: Repository = {
      ...backing,
      savePatternDoc: () => Promise.reject(new Error("配额满了")),
    };
    const { user, router } = renderApp({ route: `/pattern/${MOCHI}`, repository });

    await user.click(await screen.findByRole("button", { name: "Fork 改色" }));

    expect(await screen.findByRole("alert")).toHaveTextContent(FORK_SAVE_ERROR);
    expect(await backing.loadProjects()).toEqual([]);
    expect(currentPath(router)).toBe(`/pattern/${MOCHI}`);
    expect(screen.queryByTestId("editor-grid")).not.toBeInTheDocument();
    // 失败之后按钮仍可用：重试就是同一颗键。
    expect(forkButton()).toBeEnabled();
  });
});

describe("T-GAL-5 可用性门（D-GAL-10）", () => {
  it("无网格图纸：按钮禁用，理由常显", async () => {
    renderApp({ route: "/pattern/gal-cakebox-03" });
    expect(await screen.findByRole("button", { name: "Fork 改色" })).toBeDisabled();
    expect(screen.getByText(FORK_UNAVAILABLE_NOTE)).toBeInTheDocument();
  });

  it("有网格图纸：按钮可用，不摆理由", async () => {
    renderApp({ route: `/pattern/${MOCHI}` });
    expect(await screen.findByRole("button", { name: "Fork 改色" })).toBeEnabled();
    expect(screen.queryByText(FORK_UNAVAILABLE_NOTE)).not.toBeInTheDocument();
  });

  it("Fork 是 DetailActions 的第四键，卡片仍是整块单链接（D-GAL-11）", async () => {
    const { user } = renderApp({ route: "/explore" });
    const card = await screen.findByRole("link", { name: new RegExp(MOCHI_TITLE) });
    expect(within(card).queryByRole("button")).not.toBeInTheDocument();

    await user.click(card);
    const actions = (await screen.findByRole("button", { name: "加入待拼" })).parentElement!;
    expect([...actions.querySelectorAll("button")].map((node) => node.textContent)).toEqual([
      "加入待拼",
      "转入工作台",
      "收藏",
      "Fork 改色",
    ]);
  });
});

describe("T-GAL-6 闭环：改色后拼装拿到的是新色（D-GAL-5 / D-GAL-8 ②）", () => {
  it("fork → 编辑器全局替换 G09 → 去拼装，画布用 generic-5mm 的新色", async () => {
    const { user } = renderApp({ route: `/pattern/${MOCHI}` });

    await user.click(await screen.findByRole("button", { name: "Fork 改色" }));
    await screen.findByTestId("editor-grid");

    // 墨黑 #1b1b1f 量化成 G09 Charcoal（70 颗），换成 G22 Yellow。
    await user.selectOptions(screen.getByLabelText("要替换的色号"), "8");
    await user.selectOptions(screen.getByLabelText("替换成"), "21");
    await user.click(screen.getByRole("button", { name: "应用替换" }));
    expect(screen.getByRole("button", { name: /G22 Yellow · 70 颗/ })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "去拼装" }));
    const grid = await screen.findByTestId("assemble-grid");

    const backgrounds = [...grid.children].map((cell) => (cell as HTMLElement).style.background);
    const used = new Set(backgrounds.filter((value) => value !== ""));
    expect(backgrounds.filter((value) => value === "rgb(255, 212, 0)")).toHaveLength(70);
    // 画廊的墨黑 / 樱粉 / 抹茶绿再也不会出现：拼的是 fork 后的豆图，不是 fixture。
    for (const galleryHex of ["rgb(27, 27, 31)", "rgb(242, 167, 195)", "rgb(159, 197, 90)"]) {
      expect(used.has(galleryHex)).toBe(false);
    }
    expect(grid.getAttribute("aria-label")).toMatch(/色号 G\d\d /);
  });
});

describe("T-GAL-7 多次 Fork 与无社交（D-GAL-13）", () => {
  it("同一张图 fork 两次 = 两个独立项目", async () => {
    const repository = createInMemoryRepository();
    const first = renderApp({ route: `/pattern/${MOCHI}`, repository });
    await first.user.click(await screen.findByRole("button", { name: "Fork 改色" }));
    await screen.findByTestId("editor-grid");
    first.unmount();

    const second = renderApp({ route: `/pattern/${MOCHI}`, repository });
    await second.user.click(await screen.findByRole("button", { name: "Fork 改色" }));
    await screen.findByTestId("editor-grid");

    const projects = await repository.loadProjects();
    expect(projects).toHaveLength(2);
    expect(new Set(projects.map((project) => project.id)).size).toBe(2);
    for (const project of projects) {
      expect(project.title).toBe(`${MOCHI_TITLE}（Fork）`);
      expect(await repository.loadPatternDoc(project.id)).not.toBeNull();
    }
  });

  it("fork 不动 catalog，也不给创作者加打卡数", async () => {
    const checkInsBefore = CREATORS.map((creator) => creator.checkIns);
    const catalogBefore = JSON.stringify(PATTERNS);

    const { user } = renderApp({ route: `/pattern/${MOCHI}` });
    await user.click(await screen.findByRole("button", { name: "Fork 改色" }));
    await screen.findByTestId("editor-grid");

    expect(CREATORS.map((creator) => creator.checkIns)).toEqual(checkInsBefore);
    expect(JSON.stringify(PATTERNS)).toBe(catalogBefore);
  });
});

describe("T-GAL-8 详情页真网格预览（DEV-GAL-2 / D-GAL-12）", () => {
  it("有网格：渲染预览，非交互、格子非聚焦、容器对读屏隐藏", async () => {
    renderApp({ route: `/pattern/${MOCHI}` });
    const preview = await screen.findByTestId("pattern-preview");

    expect(preview).toHaveAttribute("aria-hidden", "true");
    expect(preview.children).toHaveLength(28 * 28);
    expect(preview.querySelector("button, a, input, [tabindex]")).toBeNull();
    // 配色清单就是文字替代面，它还在。
    expect(screen.getByRole("region", { name: "配色清单" })).toBeInTheDocument();
  });

  it("56×56 的图纸也是单实例 3136 个节点（D-ASM-6）", async () => {
    renderApp({ route: "/pattern/gal-quilt-09" });
    expect((await screen.findByTestId("pattern-preview")).children).toHaveLength(56 * 56);
  });

  it("无网格图纸保持占位，不假装有图", async () => {
    renderApp({ route: "/pattern/gal-cakebox-03" });
    await screen.findByRole("heading", { level: 1, name: "立体蛋糕盒" });
    expect(screen.queryByTestId("pattern-preview")).not.toBeInTheDocument();
  });
});

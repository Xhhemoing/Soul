import { describe, expect, it, vi } from "vitest";
import { fireEvent, screen, within } from "@testing-library/react";

import { createGrid } from "../../algo/grid.ts";
import { asPatternId, mintProjectId, type ProjectId } from "../../stores/ids.ts";
import { createPatternDoc } from "../../stores/patterns.ts";
import { createInMemoryRepository, type Repository } from "../../stores/repository.ts";
import type { PatternDoc, ProgressCursor, Project } from "../../stores/types.ts";
import { currentPath, renderApp } from "../../test/render.tsx";

/**
 * T-ED-8…19: the editor driven through the real route table.
 *
 * Autosave is debounced by 800ms of real time (D-ED-15), so the cases that care
 * about a write poll the repository instead of installing fake timers — the
 * user-event session in `renderApp` predates the timer swap and would deadlock
 * against it.
 */

const AUTOSAVE_TIMEOUT = 3000;

function blankProject(overrides: Partial<Project> = {}): Project {
  return {
    id: mintProjectId(),
    title: "空白板",
    sourcePatternId: null,
    status: "draft",
    createdAt: 20,
    backdrop: "black",
    backdropColor: "#101014",
    ...overrides,
  };
}

function emptyDoc(id: ProjectId, size = 4): PatternDoc {
  return createPatternDoc(id, createGrid(size, size));
}

interface OpenOptions {
  readonly project?: Project;
  readonly doc?: PatternDoc;
  readonly progress?: readonly ProgressCursor[];
  /** Wraps the seeded double, e.g. to make `savePatternDoc` reject. */
  readonly wrap?: (base: Repository) => Repository;
}

async function openEditor({ project, doc, progress, wrap }: OpenOptions = {}) {
  const owner = project ?? blankProject();
  const backing = createInMemoryRepository(
    { projects: [owner], ...(progress === undefined ? {} : { progress: [...progress] }) },
    [doc ?? emptyDoc(owner.id)],
  );
  const app = renderApp({
    route: `/edit/${owner.id}`,
    repository: wrap === undefined ? backing : wrap(backing),
  });
  await screen.findByTestId("editor-grid");
  return { ...app, project: owner, repository: backing };
}

function cellAt(x: number, y: number): HTMLElement {
  const found = screen
    .getByTestId("editor-grid")
    .querySelector<HTMLElement>(`[data-x="${x}"][data-y="${y}"]`);
  if (found === null) throw new Error(`画布上没有格子 (${x}, ${y})`);
  return found;
}

function beadCount(): number {
  const label = screen.getByTestId("editor-grid").getAttribute("aria-label") ?? "";
  return Number(/共 (\d+) 颗豆/.exec(label)?.[1] ?? -1);
}

/** Installs a DOM method jsdom does not implement, and takes it away again. */
function stubOwn<T extends object, K extends keyof T>(host: T, key: K, value: T[K]): () => void {
  const original = Object.getOwnPropertyDescriptor(host, key);
  Object.defineProperty(host, key, { configurable: true, writable: true, value });
  return () => {
    if (original === undefined) Reflect.deleteProperty(host, key);
    else Object.defineProperty(host, key, original);
  };
}

async function occupiedInStore(repository: Repository, id: ProjectId): Promise<number> {
  const doc = await repository.loadPatternDoc(id);
  return doc === null ? -1 : [...doc.cells].filter((cell) => cell !== -1).length;
}

describe("T-ED-8 空白铸造（D-ED-4，沿用 D-UP-10 的次序）", () => {
  it("填表确认 → 文档先落仓、项目后 mint、落在 /edit/:id", async () => {
    const repository = createInMemoryRepository();
    const { user, router } = renderApp({ route: "/create?entry=blank", repository });

    await user.click(await screen.findByRole("link", { name: "空白项目" }));
    const panel = await screen.findByRole("region", { name: "空白项目 说明" });
    await user.type(within(panel).getByLabelText("项目名称"), "小恐龙");
    await user.click(within(panel).getByRole("button", { name: "新建并开始编辑" }));

    await screen.findByTestId("editor-grid");
    const projects = await repository.loadProjects();
    expect(projects).toHaveLength(1);
    expect(projects[0]).toMatchObject({
      title: "小恐龙",
      status: "draft",
      sourcePatternId: null,
    });

    const doc = await repository.loadPatternDoc(projects[0]!.id);
    expect(doc).toMatchObject({ paletteId: "generic-5mm", width: 28, height: 28 });
    expect(doc?.provenance).toBeUndefined();
    expect([...(doc?.cells ?? [])].every((cell) => cell === -1)).toBe(true);
    expect(currentPath(router)).toBe(`/edit/${projects[0]!.id}`);
  });

  it("板型可选 56×56，上限就在这里（D-ED-5）", async () => {
    const repository = createInMemoryRepository();
    const { user } = renderApp({ route: "/create?entry=blank", repository });

    await user.click(await screen.findByRole("radio", { name: "56×56" }));
    await user.click(screen.getByRole("button", { name: "新建并开始编辑" }));

    await screen.findByTestId("editor-grid");
    const projects = await repository.loadProjects();
    expect(await repository.loadPatternDoc(projects[0]!.id)).toMatchObject({
      width: 56,
      height: 56,
    });
    expect(screen.queryByRole("radio", { name: "112×112" })).not.toBeInTheDocument();
  });

  it("空标题回落到「未命名豆图」", async () => {
    const repository = createInMemoryRepository();
    const { user } = renderApp({ route: "/create?entry=blank", repository });

    await user.click(await screen.findByRole("button", { name: "新建并开始编辑" }));
    await screen.findByTestId("editor-grid");

    expect((await repository.loadProjects())[0]).toMatchObject({ title: "未命名豆图" });
  });

  it("savePatternDoc 注入 reject → 内联错误，一个项目都不 mint", async () => {
    const backing = createInMemoryRepository();
    const repository: Repository = {
      ...backing,
      savePatternDoc: () => Promise.reject(new Error("配额满了")),
    };
    const { user, router } = renderApp({ route: "/create?entry=blank", repository });

    await user.click(await screen.findByRole("button", { name: "新建并开始编辑" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("空白豆图没有保存");
    expect(await backing.loadProjects()).toEqual([]);
    expect(currentPath(router)).toBe("/create?entry=blank");
  });

  it("blank 入口不再摆占位说明", async () => {
    renderApp({ route: "/create?entry=blank" });
    const panel = await screen.findByRole("region", { name: "空白项目 说明" });
    expect(panel).not.toHaveTextContent("不是缺陷");
    expect(panel).not.toHaveTextContent("WP-B06");
  });
});

describe("T-ED-9 守卫链四态（D-ED-18）", () => {
  it("未知 id：不跳转，给出路", async () => {
    const { router } = renderApp({ route: "/edit/proj-nobody" });
    expect(await screen.findByText(/这个项目不存在/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "回拼装台" })).toHaveAttribute("href", "/workspace");
    expect(currentPath(router)).toBe("/edit/proj-nobody");
  });

  it("画廊图纸 id 不是项目 id，同一屏拦下", async () => {
    renderApp({ route: "/edit/gal-slime-01" });
    expect(await screen.findByText(/这个项目不存在/)).toBeInTheDocument();
  });

  // D-GAL-14 ①: 守卫行为一字未动，改的是它说的话——WP-B08 到了，画廊项目的出路
  // 不再是「等下一个工作包」，而是回详情页 Fork 一份副本。
  it("画廊项目：只读，文案指路详情页 Fork", async () => {
    const project = blankProject({ sourcePatternId: asPatternId("gal-slime-01") });
    const { router } = renderApp({ route: `/edit/${project.id}`, seed: { projects: [project] } });

    const note = await screen.findByText(/只读/);
    expect(note).toHaveTextContent(/Fork/);
    expect(note).toHaveTextContent(/详情页/);
    expect(screen.queryByText(/WP-B08/)).not.toBeInTheDocument();
    expect(screen.queryByTestId("editor-grid")).not.toBeInTheDocument();
    expect(screen.getByRole("link", { name: "回拼装台" })).toBeInTheDocument();
    expect(currentPath(router)).toBe(`/edit/${project.id}`);
  });

  it("文档未命中：说清楚，不空白", async () => {
    const project = blankProject();
    renderApp({ route: `/edit/${project.id}`, seed: { projects: [project] } });

    expect(await screen.findByText(/这个项目还没有豆图文档/)).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "回拼装台" })).toBeInTheDocument();
  });

  it("读取中：有加载态、有出路，不自动跳走", async () => {
    const project = blankProject();
    const repository: Repository = {
      ...createInMemoryRepository({ projects: [project] }),
      loadPatternDoc: () => new Promise(() => {}),
    };
    renderApp({ route: `/edit/${project.id}`, repository });

    expect(await screen.findByText("正在读取豆图……")).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "回拼装台" })).toBeInTheDocument();
  });
});

describe("T-ED-10 画笔与对称（D-ED-8 / D-ED-9）", () => {
  it("点格上色，活动色「空」即橡皮", async () => {
    const { user } = await openEditor();
    expect(beadCount()).toBe(0);

    await user.click(cellAt(0, 0));
    expect(beadCount()).toBe(1);
    expect(screen.getByRole("button", { name: /G06 Black · 1 颗/ })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "空（橡皮）" }));
    await user.click(cellAt(0, 0));
    expect(beadCount()).toBe(0);
  });

  it("四向对称一笔四格，油漆桶不受对称影响", async () => {
    const { user } = await openEditor();
    await user.click(screen.getByRole("radio", { name: "四向" }));
    await user.click(cellAt(0, 0));
    expect(beadCount()).toBe(4);

    await user.click(screen.getByRole("radio", { name: "关" }));
    await user.click(cellAt(1, 1));
    expect(beadCount()).toBe(5);
  });

  it("按住拖过去是一笔，中间的格子不会漏", async () => {
    await openEditor();
    fireEvent.pointerDown(cellAt(0, 0), { pointerId: 1 });
    fireEvent.pointerMove(cellAt(3, 0), { pointerId: 1 });
    fireEvent.pointerUp(cellAt(3, 0), { pointerId: 1 });

    expect(beadCount()).toBe(4);
    // 一笔 = 一条历史：撤销一次整笔消失。
    fireEvent.click(screen.getByRole("button", { name: "撤销" }));
    expect(beadCount()).toBe(0);
  });

  it("指针捕获把 pointermove 重定向到容器后，拖笔依然落满整条", async () => {
    await openEditor();
    const grid = screen.getByTestId("editor-grid");
    // jsdom 两样都没有，所以上面那条测试走的是「未捕获」路径。真实浏览器里
    // 捕获一开，其后每个 pointermove 的 target 都是容器，格子只能靠命中测试
    // 找回——这里把两者都补上，钉住生产路径。
    const captured: number[] = [];
    const restoreCapture = stubOwn(Element.prototype, "setPointerCapture", (pointerId: number) => {
      captured.push(pointerId);
    });
    let under: Element | null = null;
    const restoreHitTest = stubOwn(document, "elementFromPoint", () => under);

    try {
      fireEvent.pointerDown(cellAt(0, 0), { pointerId: 1 });
      expect(captured).toEqual([1]);
      for (const x of [1, 2, 3]) {
        under = cellAt(x, 0);
        fireEvent.pointerMove(grid, { pointerId: 1, clientX: x * 12, clientY: 0 });
      }
      fireEvent.pointerUp(grid, { pointerId: 1 });
    } finally {
      restoreHitTest();
      restoreCapture();
    }

    expect(beadCount()).toBe(4);
    fireEvent.click(screen.getByRole("button", { name: "撤销" }));
    expect(beadCount()).toBe(0);
  });

  it("捕获期间指针移出板外不画格，回到板上笔画继续", async () => {
    await openEditor();
    const grid = screen.getByTestId("editor-grid");
    const restoreCapture = stubOwn(Element.prototype, "setPointerCapture", () => {});
    let under: Element | null = null;
    const restoreHitTest = stubOwn(document, "elementFromPoint", () => under);

    try {
      fireEvent.pointerDown(cellAt(0, 0), { pointerId: 1 });
      under = document.body; // 板外：命中测试拿到的不是格子
      fireEvent.pointerMove(grid, { pointerId: 1 });
      expect(beadCount()).toBe(1);

      under = cellAt(0, 1);
      fireEvent.pointerMove(grid, { pointerId: 1 });
      fireEvent.pointerUp(grid, { pointerId: 1 });
    } finally {
      restoreHitTest();
      restoreCapture();
    }

    expect(beadCount()).toBe(2);
  });
});

describe("T-ED-11 油漆桶与拾色器（D-ED-10 / D-ED-11）", () => {
  it("油漆桶灌满空区", async () => {
    const { user } = await openEditor();
    await user.click(screen.getByRole("radio", { name: "油漆桶" }));
    await user.click(cellAt(2, 2));
    expect(beadCount()).toBe(16);
  });

  it("拾色器取色后自动回画笔", async () => {
    const { user } = await openEditor();
    await user.click(screen.getByRole("button", { name: "G22 Yellow" }));
    await user.click(cellAt(0, 0));

    await user.click(screen.getByRole("button", { name: "G01 White" }));
    await user.click(screen.getByRole("radio", { name: "拾色器" }));
    await user.click(cellAt(0, 0));

    expect(screen.getByRole("radio", { name: "画笔" })).toBeChecked();
    expect(screen.getByRole("button", { name: "G22 Yellow" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(beadCount()).toBe(1); // 取色不画格
  });
});

describe("T-ED-12 色号替换（D-ED-12）", () => {
  it("源列表只列在用码并带颗数，应用后画布与统计同步更新", async () => {
    const { user } = await openEditor();
    await user.click(cellAt(0, 0));
    await user.click(cellAt(1, 0));

    const source = screen.getByLabelText("要替换的色号");
    expect(within(source).getAllByRole("option")).toHaveLength(1);
    expect(within(source).getByRole("option", { name: /G06 Black · 2 颗/ })).toBeInTheDocument();
    expect(screen.getByText("将改写 2 颗")).toBeInTheDocument();

    await user.selectOptions(screen.getByLabelText("替换成"), "21");
    await user.click(screen.getByRole("button", { name: "应用替换" }));

    expect(screen.getByRole("button", { name: /G22 Yellow · 2 颗/ })).toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /G06 Black · 2 颗/ })).not.toBeInTheDocument();
    expect(beadCount()).toBe(2);
  });

  it("目标「空」就是擦除；源 == 目标时按钮禁用", async () => {
    const { user } = await openEditor();
    await user.click(screen.getByRole("button", { name: "G01 White" }));
    await user.click(cellAt(0, 0));

    // G01 的下标是 0，目标下拉的默认值也是 0：同码同码，按钮关着。
    expect(screen.getByRole("button", { name: "应用替换" })).toBeDisabled();

    await user.selectOptions(screen.getByLabelText("替换成"), "empty");
    await user.click(screen.getByRole("button", { name: "应用替换" }));
    expect(beadCount()).toBe(0);
  });
});

describe("T-ED-13 用色统计（D-ED-13）", () => {
  it("按颗数降序、下标升序，且点行就换活动色", async () => {
    const { user } = await openEditor();
    await user.click(screen.getByRole("button", { name: "G22 Yellow" }));
    await user.click(cellAt(0, 0));
    await user.click(screen.getByRole("button", { name: "G06 Black" }));
    await user.click(cellAt(1, 0));
    await user.click(cellAt(2, 0));

    const rows = within(screen.getByRole("list", { name: "用色清单" })).getAllByRole("button");
    expect(rows.map((row) => row.textContent)).toEqual([
      "【通用5mm】G06 Black · 2 颗",
      "【通用5mm】G22 Yellow · 1 颗",
    ]);
    expect(screen.getByText("合计 3 颗")).toBeInTheDocument();

    await user.click(rows[1]!);
    expect(screen.getByRole("button", { name: "G22 Yellow" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
  });
});

describe("T-ED-14 撤销 / 重做（D-ED-14 / D-ASM-13）", () => {
  it("按钮与快捷键等价，栈空时按钮禁用", async () => {
    const { user } = await openEditor();
    expect(screen.getByRole("button", { name: "撤销" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "重做" })).toBeDisabled();

    await user.click(cellAt(0, 0));
    await user.click(screen.getByRole("button", { name: "撤销" }));
    expect(beadCount()).toBe(0);
    await user.click(screen.getByRole("button", { name: "重做" }));
    expect(beadCount()).toBe(1);

    await user.keyboard("{Control>}z{/Control}");
    expect(beadCount()).toBe(0);
    await user.keyboard("{Control>}{Shift>}Z{/Shift}{/Control}");
    expect(beadCount()).toBe(1);
    await user.keyboard("{Control>}z{/Control}");
    await user.keyboard("{Control>}y{/Control}");
    expect(beadCount()).toBe(1);
  });

  it("焦点在下拉里时快捷键不触发（D-ASM-13）", async () => {
    const { user } = await openEditor();
    await user.click(cellAt(0, 0));

    screen.getByLabelText("替换成").focus();
    await user.keyboard("{Control>}z{/Control}");
    expect(beadCount()).toBe(1);
  });
});

describe("T-ED-15 自动保存（D-ED-15）", () => {
  it("改格 → 防抖后仓里已更新；期间不抢跑", async () => {
    const { user, project, repository } = await openEditor();

    await user.click(cellAt(0, 0));
    expect(await occupiedInStore(repository, project.id)).toBe(0);
    expect(screen.getByRole("status")).toHaveTextContent("");

    await vi.waitFor(async () => expect(await occupiedInStore(repository, project.id)).toBe(1), {
      timeout: AUTOSAVE_TIMEOUT,
    });
    expect(screen.getByRole("status")).toHaveTextContent("已保存");
  });

  it("pagehide 立刻 flush，不等防抖", async () => {
    const { user, project, repository } = await openEditor();

    await user.click(cellAt(0, 0));
    fireEvent(window, new Event("pagehide"));

    await vi.waitFor(async () => expect(await occupiedInStore(repository, project.id)).toBe(1), {
      timeout: 300,
    });
  });

  it("离开编辑器也 flush，重进读回同一网格", async () => {
    const { user, project, repository } = await openEditor({
      project: blankProject({ title: "回来的板" }),
    });

    await user.click(cellAt(1, 1));
    await user.click(screen.getByRole("link", { name: "← 回拼装台" }));

    await screen.findByRole("heading", { level: 1, name: "拼装台" });
    await vi.waitFor(async () => expect(await occupiedInStore(repository, project.id)).toBe(1), {
      timeout: 300,
    });

    await user.click(screen.getByRole("link", { name: "编辑豆图" }));
    await screen.findByTestId("editor-grid");
    expect(beadCount()).toBe(1);
  });
});

describe("T-ED-16 进度游标归零（D-ED-16 / DEV-ED-3）", () => {
  const cursor = (projectId: ProjectId): ProgressCursor => ({
    projectId,
    mode: "row-by-row",
    stepIndex: 5,
    elapsedMs: 61_000,
    updatedAt: 100,
  });

  it("保存成功后 stepIndex 归 0，mode 与 elapsedMs 不动，提示句可见", async () => {
    const project = blankProject({ status: "active" });
    const { user, repository } = await openEditor({ project, progress: [cursor(project.id)] });

    expect(screen.getByText("编辑会使拼装进度回到第 1 步")).toBeInTheDocument();
    await user.click(cellAt(0, 0));

    await vi.waitFor(
      async () =>
        expect(await repository.loadProgress()).toEqual([
          expect.objectContaining({
            projectId: project.id,
            mode: "row-by-row",
            stepIndex: 0,
            elapsedMs: 61_000,
          }),
        ]),
      { timeout: AUTOSAVE_TIMEOUT },
    );
  });

  it("没有游标就不凭空造一个", async () => {
    const { user, project, repository } = await openEditor();

    expect(screen.queryByText("编辑会使拼装进度回到第 1 步")).not.toBeInTheDocument();
    await user.click(cellAt(0, 0));

    await vi.waitFor(async () => expect(await occupiedInStore(repository, project.id)).toBe(1), {
      timeout: AUTOSAVE_TIMEOUT,
    });
    expect(await repository.loadProgress()).toEqual([]);
  });
});

describe("T-ED-17 保存失败（D-ED-15 / D-UP-16）", () => {
  it("状态文案说「保存失败」，画布继续可编辑，不新增错误 UI", async () => {
    const { user } = await openEditor({
      wrap: (base) => ({ ...base, savePatternDoc: () => Promise.reject(new Error("配额满了")) }),
    });

    await user.click(cellAt(0, 0));
    await vi.waitFor(() => expect(screen.getByRole("status")).toHaveTextContent("保存失败"), {
      timeout: AUTOSAVE_TIMEOUT,
    });

    await user.click(cellAt(1, 0));
    expect(beadCount()).toBe(2);
    // DATA-1 的横幅由仓库的 persistenceFailed 决定，编辑器不另立一套错误 UI。
    expect(screen.queryByText("本地存储不可用，本次更改不会保存。")).not.toBeInTheDocument();
  });
});

describe("T-ED-18 Workspace 入口（D-ED-17 ② / DEV-ED-4）", () => {
  it("空白草稿卡出「编辑豆图」，画廊项目卡没有", async () => {
    const draft = blankProject({ title: "草稿板" });
    const gallery = blankProject({
      title: "画廊项目",
      status: "active",
      sourcePatternId: asPatternId("gal-slime-01"),
    });
    renderApp({ route: "/workspace", seed: { projects: [draft, gallery] } });

    const drafts = await screen.findByRole("region", { name: "草稿与设计" });
    expect(within(drafts).getByRole("link", { name: "编辑豆图" })).toHaveAttribute(
      "href",
      `/edit/${draft.id}`,
    );

    const inProgress = screen.getByRole("region", { name: "正在拼" });
    expect(within(inProgress).getByText("画廊项目")).toBeInTheDocument();
    expect(within(inProgress).queryByRole("link", { name: "编辑豆图" })).not.toBeInTheDocument();
  });
});

describe("T-ED-19 闭环收编（D-ED-17 ③）", () => {
  it("画几格 →「去拼装」→ 项目转 active 并渲染拼装网格", async () => {
    const { user, router, project, repository } = await openEditor();

    expect(screen.getByRole("button", { name: "去拼装" })).toBeDisabled();
    await user.click(cellAt(0, 0));
    await user.click(cellAt(1, 0));
    await user.click(screen.getByRole("button", { name: "去拼装" }));

    await screen.findByTestId("assemble-grid");
    expect(currentPath(router)).toBe(`/assemble/${project.id}`);
    expect((await repository.loadProjects())[0]).toMatchObject({ status: "active" });
    // 走的是 D-UP-14 的 null 分支：网格来自刚刚 flush 的 PatternDoc。
    expect(await occupiedInStore(repository, project.id)).toBe(2);
  });

  it("已经在拼的项目只导航，不改状态", async () => {
    const { user, repository } = await openEditor({ project: blankProject({ status: "todo" }) });

    await user.click(cellAt(0, 0));
    await user.click(screen.getByRole("button", { name: "去拼装" }));

    await screen.findByTestId("assemble-grid");
    expect((await repository.loadProjects())[0]).toMatchObject({ status: "todo" });
  });
});

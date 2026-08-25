import { afterEach, describe, expect, it, vi } from "vitest";
import { act, screen, waitFor, within } from "@testing-library/react";

import { currentPath, renderApp } from "../../test/render.tsx";
import { FlakyStorage } from "../../test/flaky-storage.ts";
import { fixtureGridFor } from "../../fixtures/grids.ts";
import {
  createInMemoryRepository,
  createRepository,
  type Repository,
} from "../../stores/repository.ts";
import { asPatternId, asProjectId, mintProjectId } from "../../stores/ids.ts";
import { createGrid } from "../../algo/grid.ts";
import { splitSteps } from "../../algo/steps.ts";
import { createPatternDoc } from "../../stores/patterns.ts";
import type { PatternDoc, ProgressCursor, Project } from "../../stores/types.ts";
import { LOADING_GRID_NOTE, NO_GRID_NOTE } from "./AssemblePage.tsx";

const SLIME = asPatternId("gal-slime-01");
const SIDE = 28;

function project(overrides: Partial<Project> = {}): Project {
  return {
    id: mintProjectId(),
    title: "史莱姆小队",
    sourcePatternId: SLIME,
    status: "active",
    createdAt: 1,
    backdrop: "black",
    backdropColor: "#101014",
    ...overrides,
  };
}

function cursorFor(owner: Project, overrides: Partial<ProgressCursor> = {}): ProgressCursor {
  return {
    projectId: owner.id,
    mode: "color-by-color",
    stepIndex: 0,
    elapsedMs: 0,
    updatedAt: 100,
    ...overrides,
  };
}

function beads(state: "done" | "current" | "pending"): HTMLElement[] {
  return screen.queryAllByTestId("assemble-grid").flatMap((grid) => [
    ...grid.querySelectorAll<HTMLElement>(`[data-cell-state="${state}"]`),
  ]);
}

/** Row indices the current highlight covers — the canvas is row-major. */
function currentRows(): number[] {
  const grid = screen.getByTestId("assemble-grid");
  const rows = new Set<number>();
  [...grid.children].forEach((cell, index) => {
    if (cell.getAttribute("data-cell-state") === "current") rows.add(Math.floor(index / SIDE));
  });
  return [...rows].sort((a, b) => a - b);
}

const slimeSteps = (mode: Parameters<typeof splitSteps>[1]) =>
  splitSteps(fixtureGridFor(SLIME)!.grid, mode);

afterEach(() => {
  vi.useRealTimers();
});

describe("进场与三态（T-ASM-8）", () => {
  it("默认单色模式：当前步整步高亮，其余是 15% 的未来步", async () => {
    const seeded = project();
    renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    await screen.findByTestId("assemble-grid");
    const first = slimeSteps("color-by-color")[0]!;
    expect(beads("current")).toHaveLength(first.cells.length);
    expect(beads("done")).toHaveLength(0);
    expect(beads("pending")).toHaveLength(300 - first.cells.length);
    // 空格子不是豆子，不参与三态。
    expect(beads("current").length + beads("pending").length).toBe(300);
  });

  it("画布容器自带随步更新的文字标签，格子本身对读屏隐藏", async () => {
    const seeded = project();
    renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    const grid = await screen.findByTestId("assemble-grid");
    expect(grid).toHaveAttribute("role", "img");
    expect(grid.getAttribute("aria-label")).toMatch(/第 1\/4 步 · 色号 C01 纯白/);
    expect(grid.children[0]).toHaveAttribute("aria-hidden", "true");
  });
});

describe("悬浮球与键盘（T-ASM-9 / D-ASM-13）", () => {
  it("下一步与撤销移动高亮，方向键等价", async () => {
    const seeded = project();
    const { user } = renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    await screen.findByTestId("assemble-grid");
    const steps = slimeSteps("color-by-color");
    const orb = screen.getByRole("group", { name: "拼装控制" });

    expect(within(orb).getByRole("button", { name: "撤销" })).toBeDisabled();

    await user.click(within(orb).getByRole("button", { name: "下一步" }));
    expect(beads("done")).toHaveLength(steps[0]!.cells.length);
    expect(beads("current")).toHaveLength(steps[1]!.cells.length);

    await user.keyboard("{ArrowRight}");
    expect(beads("current")).toHaveLength(steps[2]!.cells.length);

    await user.keyboard("{ArrowLeft}");
    expect(beads("current")).toHaveLength(steps[1]!.cells.length);

    await user.click(within(orb).getByRole("button", { name: "撤销" }));
    expect(beads("current")).toHaveLength(steps[0]!.cells.length);
    expect(beads("done")).toHaveLength(0);
  });

  it("焦点在背景色输入框里时方向键不动游标", async () => {
    const seeded = project({ backdrop: "custom", backdropColor: "#223344" });
    const { user } = renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    await screen.findByTestId("assemble-grid");
    const before = beads("current").length;
    const colorInput = screen.getByLabelText("背景色");
    colorInput.focus();
    await user.keyboard("{ArrowRight}");

    expect(beads("current")).toHaveLength(before);
    expect(beads("done")).toHaveLength(0);
  });
});

describe("模式切换与恢复（T-ASM-10 / T-ASM-11 / D-ASM-4）", () => {
  it("换模式把游标归零，并把新模式写进仓库", async () => {
    const seeded = project();
    const { user, repository } = renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded] },
    });

    await screen.findByTestId("assemble-grid");
    await user.click(screen.getByRole("button", { name: "下一步" }));
    expect(beads("done").length).toBeGreaterThan(0);

    await user.click(screen.getByRole("radio", { name: "逐行扫描" }));

    expect(beads("done")).toHaveLength(0);
    expect(currentRows()).toEqual([2]);
    await waitFor(async () => {
      const stored = await repository.loadProgress();
      expect(stored[0]).toMatchObject({ projectId: seeded.id, mode: "row-by-row", stepIndex: 0 });
    });
  });

  it("常显一句「切换模式将从第 1 步开始」", async () => {
    const seeded = project();
    renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });
    expect(await screen.findByText(/切换模式将从第 1 步开始/)).toBeInTheDocument();
  });

  it("恢复：逐行扫描第 3 步 → radio 选中且第 3 个非空行是当前步", async () => {
    const seeded = project();
    renderApp({
      route: `/assemble/${seeded.id}`,
      seed: {
        projects: [seeded],
        progress: [cursorFor(seeded, { mode: "row-by-row", stepIndex: 2, elapsedMs: 65_000 })],
      },
    });

    await screen.findByTestId("assemble-grid");
    expect(screen.getByRole("radio", { name: "逐行扫描" })).toBeChecked();
    // 史莱姆的非空行从 y=2 起，第 3 步就是 y=4。
    expect(currentRows()).toEqual([4]);
    expect(screen.getByTestId("assemble-clock")).toHaveTextContent("01:05");
  });
});

describe("完成态（T-ASM-12 / D-ASM-11）", () => {
  it("越界游标被夹到完成面板，不崩", async () => {
    const seeded = project();
    renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded], progress: [cursorFor(seeded, { stepIndex: 999 })] },
    });

    expect(await screen.findByRole("button", { name: "标记完工" })).toBeInTheDocument();
    expect(beads("done")).toHaveLength(300);
    expect(beads("current")).toHaveLength(0);
    expect(screen.getByRole("button", { name: "下一步" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "撤销" })).toBeEnabled();
  });

  it("「标记完工」把项目标 done 并回拼装台", async () => {
    const seeded = project();
    const { user, router, repository } = renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded], progress: [cursorFor(seeded, { stepIndex: 999 })] },
    });

    await user.click(await screen.findByRole("button", { name: "标记完工" }));
    await waitFor(() => expect(currentPath(router)).toBe("/workspace"));
    const stored = await repository.loadProjects();
    expect(stored[0]?.status).toBe("done");
  });
});

describe("计时（T-ASM-13 / D-ASM-8）", () => {
  it("页面可见时走秒，隐藏时暂停，复显续走", async () => {
    vi.useFakeTimers();
    const seeded = project();
    renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    const clock = screen.getByTestId("assemble-clock");
    expect(clock).toHaveTextContent("00:00");

    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(clock).toHaveTextContent("00:03");

    const hidden = vi.spyOn(document, "hidden", "get").mockReturnValue(true);
    await act(async () => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(5000);
    });
    expect(clock).toHaveTextContent("00:03");

    hidden.mockReturnValue(false);
    await act(async () => {
      document.dispatchEvent(new Event("visibilitychange"));
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
    });
    expect(clock).toHaveTextContent("00:05");
  });
});

describe("写盘时刻（§3.2 / D-ASM-8）", () => {
  it("秒针不落盘，卸载与 pagehide 才把用时刷进游标", async () => {
    vi.useFakeTimers();
    const seeded = project();
    const { repository, router } = renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded] },
    });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    await act(async () => {
      await vi.advanceTimersByTimeAsync(3000);
    });
    expect(screen.getByTestId("assemble-clock")).toHaveTextContent("00:03");
    // 每秒重写整包 localStorage 是无谓的：节拍不是写盘时刻。
    expect((await repository.loadProgress())[0]).toMatchObject({ elapsedMs: 0, stepIndex: 0 });

    await act(async () => {
      window.dispatchEvent(new Event("pagehide"));
    });
    expect((await repository.loadProgress())[0]).toMatchObject({ elapsedMs: 3000 });

    await act(async () => {
      await vi.advanceTimersByTimeAsync(2000);
      await router.navigate("/workspace");
    });
    expect((await repository.loadProgress())[0]).toMatchObject({ elapsedMs: 5000 });
  });
});

describe("播报（T-ASM-14 / D-ASM-9）", () => {
  it("单色模式完成一个色号会播报，其余模式不播报", async () => {
    const seeded = project();
    const { user } = renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    await screen.findByTestId("assemble-grid");
    await user.click(screen.getByRole("button", { name: "下一步" }));
    expect(screen.getByRole("status")).toHaveTextContent(/色号 C01 完成/);

    await user.click(screen.getByRole("radio", { name: "分块推进" }));
    expect(screen.getByRole("status")).toHaveTextContent("");
    await user.click(screen.getByRole("button", { name: "下一步" }));
    expect(screen.getByRole("status")).not.toHaveTextContent("色号");
  });

  it("里程碑向上穿越播报一次", async () => {
    const seeded = project();
    const { user } = renderApp({
      route: `/assemble/${seeded.id}`,
      seed: {
        projects: [seeded],
        progress: [cursorFor(seeded, { mode: "row-by-row", stepIndex: 4 })],
      },
    });

    await screen.findByTestId("assemble-grid");
    const status = screen.getByRole("status");
    expect(status).toHaveTextContent("");

    const steps = slimeSteps("row-by-row");
    for (let index = 4; index < steps.length; index += 1) {
      await user.click(screen.getByRole("button", { name: "下一步" }));
    }
    expect(status).toHaveTextContent("已完成 100%");
  });
});

describe("1:1 透光模式（T-ASM-15 / D-ASM-10）", () => {
  it("开启后画布标 physical 并常显免责声明，关闭后都撤掉", async () => {
    const seeded = project();
    const { user } = renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    const grid = await screen.findByTestId("assemble-grid");
    expect(grid).not.toHaveAttribute("data-scale");

    const toggle = screen.getByRole("checkbox", { name: /1:1 透光模式/ });
    await user.click(toggle);
    expect(screen.getByTestId("assemble-grid")).toHaveAttribute("data-scale", "physical");
    expect(screen.getByText(/显示器未校准/)).toBeInTheDocument();
    expect(screen.getByText(/140mm/)).toBeInTheDocument();

    await user.click(toggle);
    expect(screen.getByTestId("assemble-grid")).not.toHaveAttribute("data-scale");
    expect(screen.queryByText(/显示器未校准/)).not.toBeInTheDocument();
  });
});

describe("锁定当前行（T-ASM-16 / D-ASM-7）", () => {
  it("单色模式开启后当前高亮只剩一行", async () => {
    const seeded = project();
    const { user } = renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    await screen.findByTestId("assemble-grid");
    expect(currentRows().length).toBeGreaterThan(1);

    const lock = screen.getByRole("button", { name: "锁定当前行" });
    await user.click(lock);
    expect(lock).toHaveAttribute("aria-pressed", "true");
    const firstRow = currentRows();
    expect(firstRow).toHaveLength(1);

    await user.click(screen.getByRole("button", { name: "下一步" }));
    const secondRow = currentRows();
    expect(secondRow).toHaveLength(1);
    expect(secondRow[0]!).toBeGreaterThan(firstRow[0]!);
    // 步内已走过的行按已完成渲染，游标本身还没跨步。
    expect(beads("done").length).toBeGreaterThan(0);
  });

  it("逐行扫描模式下这个键是禁用的", async () => {
    const seeded = project();
    renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded], progress: [cursorFor(seeded, { mode: "row-by-row" })] },
    });

    const lock = await screen.findByRole("button", { name: "锁定当前行" });
    expect(lock).toBeDisabled();
    expect(lock).toHaveAttribute("title", "逐行模式本身即按行");
  });
});

describe("暂无网格态与框架不变量（T-ASM-17 / D-ASM-12）", () => {
  it.each([
    ["空白项目", null],
    ["立体拼豆图纸", asPatternId("gal-cakebox-03")],
  ])("%s：框架全在，但没有会话区", async (_label, sourcePatternId) => {
    const seeded = project({ sourcePatternId, title: "没有网格的项目" });
    renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    expect(
      await screen.findByRole("heading", { level: 1, name: "没有网格的项目" }),
    ).toBeInTheDocument();
    expect(screen.getByRole("link", { name: "退出拼装" })).toBeInTheDocument();
    expect(screen.getByRole("radio", { name: "黑色背景" })).toBeInTheDocument();
    expect(screen.getByTestId("assemble-backdrop")).toBeInTheDocument();
    expect(screen.getByText(/还没有豆图网格/)).toBeInTheDocument();

    expect(screen.queryByTestId("assemble-grid")).not.toBeInTheDocument();
    expect(screen.queryByRole("group", { name: "拼装控制" })).not.toBeInTheDocument();
    expect(screen.queryByTestId("assemble-clock")).not.toBeInTheDocument();
  });
});

describe("转换项目走同一条会话（T-UP-16 / T-UP-17）", () => {
  // generic-5mm 下标 5 是 G06 Black，14 是 G15 Red。两色 4×2，单色模式两步。
  const CONVERTED_CELLS = [5, 5, 14, 14, 5, null, 14, null];

  function converted(): { seeded: Project; doc: PatternDoc } {
    const seeded = project({ sourcePatternId: null, title: "上传转出来的图" });
    const doc = createPatternDoc(asProjectId(seeded.id), createGrid(4, 2, CONVERTED_CELLS), {
      kind: "PixelArt",
      ditherApplied: false,
    });
    return { seeded, doc };
  }

  /** `currentRows` is wired to the 28-wide fixture; this board is 4 wide. */
  function convertedCurrentRows(): number[] {
    const grid = screen.getByTestId("assemble-grid");
    const rows = new Set<number>();
    [...grid.children].forEach((cell, index) => {
      if (cell.getAttribute("data-cell-state") === "current") rows.add(Math.floor(index / 4));
    });
    return [...rows].sort((a, b) => a - b);
  }

  it("豆图读回来后渲染 assemble-grid，色号文本来自 generic-5mm", async () => {
    const { seeded, doc } = converted();
    renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded] },
      patterns: [doc],
    });

    const grid = await screen.findByTestId("assemble-grid");
    // AssembleCanvas 的 API 没变：同一个 {grid, palette} 适配器，同一套三态。
    expect(grid.getAttribute("aria-label")).toMatch(/第 1\/2 步 · 色号 G06 Black/);
    expect(beads("current")).toHaveLength(3);
    expect(beads("pending")).toHaveLength(3);
    expect(beads("done")).toHaveLength(0);
  });

  it("步进 / 模式 / 游标与画廊项目同一套行为", async () => {
    const { seeded, doc } = converted();
    const { user, repository } = renderApp({
      route: `/assemble/${seeded.id}`,
      seed: { projects: [seeded] },
      patterns: [doc],
    });

    await screen.findByTestId("assemble-grid");
    await user.click(screen.getByRole("button", { name: "下一步" }));
    expect(beads("done")).toHaveLength(3);
    expect(screen.getByTestId("assemble-grid").getAttribute("aria-label")).toMatch(
      /第 2\/2 步 · 色号 G15 Red/,
    );

    await user.click(screen.getByRole("radio", { name: "逐行扫描" }));
    expect(beads("done")).toHaveLength(0);
    expect(convertedCurrentRows()).toEqual([0]);
    await waitFor(async () => {
      const stored = await repository.loadProgress();
      expect(stored[0]).toMatchObject({ projectId: seeded.id, mode: "row-by-row", stepIndex: 0 });
    });
  });

  it("种进去的游标照样恢复，四个模式都在", async () => {
    const { seeded, doc } = converted();
    renderApp({
      route: `/assemble/${seeded.id}`,
      seed: {
        projects: [seeded],
        progress: [cursorFor(seeded, { mode: "row-by-row", stepIndex: 1 })],
      },
      patterns: [doc],
    });

    await screen.findByTestId("assemble-grid");
    expect(screen.getByRole("radio", { name: "逐行扫描" })).toBeChecked();
    expect(convertedCurrentRows()).toEqual([1]);
    expect(screen.getAllByRole("radio", { name: /推进|扫描|填充/ })).toHaveLength(4);
  });

  it("T-UP-17：仓里没有豆图 → 新版 NO_GRID_NOTE，且不提上传", async () => {
    const seeded = project({ sourcePatternId: null, title: "没有豆图的转换项目" });
    renderApp({ route: `/assemble/${seeded.id}`, seed: { projects: [seeded] } });

    expect(await screen.findByText(/还没有豆图网格/)).toBeInTheDocument();
    expect(screen.getByText(/还没有豆图网格/)).toHaveTextContent(NO_GRID_NOTE);
    expect(screen.getByText(/还没有豆图网格/)).not.toHaveTextContent("WP-B03");
    expect(screen.queryByTestId("assemble-grid")).not.toBeInTheDocument();
  });

  it("T-UP-17：读取窗口是显式加载态，不是「没有网格」", async () => {
    const seeded = project({ sourcePatternId: null, title: "读取中的转换项目" });
    const repository: Repository = {
      ...createInMemoryRepository({ projects: [seeded] }),
      loadPatternDoc: () => new Promise(() => {}),
    };
    renderApp({ route: `/assemble/${seeded.id}`, repository });

    const note = await screen.findByText(LOADING_GRID_NOTE);
    expect(note).toHaveAttribute("aria-busy", "true");
    expect(screen.queryByText(/还没有豆图网格/)).not.toBeInTheDocument();
  });
});

describe("DATA-1 不回归（T-ASM-18）", () => {
  it("存储写不进去时横幅常显，步进照样能走（内存态）", async () => {
    const storage = new FlakyStorage();
    const seeded = project();
    storage.setItem("bead.state", JSON.stringify({ projects: [seeded] }));
    storage.full = true;

    const repository = createRepository(storage);
    const { user } = renderApp({ route: `/assemble/${seeded.id}`, repository });

    expect(await screen.findByRole("alert")).toHaveTextContent("本地存储不可用，本次更改不会保存。");
    await screen.findByTestId("assemble-grid");

    await user.click(screen.getByRole("button", { name: "下一步" }));
    expect(beads("done").length).toBeGreaterThan(0);
    expect(screen.getByRole("alert")).toBeInTheDocument();
    // 会话内仍读得到：仓库降级成内存副本而不是静默丢弃。
    expect((await repository.loadProgress())[0]).toMatchObject({ stepIndex: 1 });
  });
});

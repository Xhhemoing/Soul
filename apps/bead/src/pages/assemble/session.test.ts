import { describe, expect, it } from "vitest";

import {
  beadsPerMinute,
  cellsBeforeSlice,
  createSessionState,
  describeStep,
  formatDuration,
  reachedMilestones,
  sessionReduce,
  sessionStats,
  stepRows,
  type SessionAction,
  type SessionContext,
  type SessionState,
} from "./session.ts";
import { createGrid, type Cell } from "../../algo/grid.ts";
import { splitSteps, type SplitMode } from "../../algo/steps.ts";

// 3×3：色 0 是最左一竖（3 颗，跨 3 行），色 1 是右边两竖（6 颗）。
// 单色模式升序 ⇒ 第 0 步是色 0，第 1 步是色 1。
const CELLS: Cell[] = [0, 1, 1, 0, 1, 1, 0, 1, 1];
const GRID = createGrid(3, 3, CELLS);
const PALETTE = ["H02", "B05"];

function contextFor(mode: SplitMode): SessionContext {
  return { steps: splitSteps(GRID, mode), paletteCodes: PALETTE };
}

function run(
  state: SessionState,
  context: SessionContext,
  actions: readonly SessionAction[],
): SessionState {
  return actions.reduce((current, action) => sessionReduce(current, action, context), state);
}

const COLOR = contextFor("color-by-color");
const ROWS = contextFor("row-by-row");

function start(context: SessionContext): SessionState {
  return createSessionState(undefined, context.steps);
}

describe("sessionStats（T-ASM-2）", () => {
  it("按已完成步累计颗数、总数与占比", () => {
    expect(sessionStats(COLOR.steps, 0)).toMatchObject({ totalCells: 9, placedCells: 0, percent: 0 });
    expect(sessionStats(COLOR.steps, 1)).toMatchObject({ placedCells: 3, percent: 33 });
    expect(sessionStats(COLOR.steps, 2)).toMatchObject({ placedCells: 9, percent: 100 });
    expect(sessionStats(COLOR.steps, 1).fraction).toBeCloseTo(1 / 3);
  });

  it("行窗口内已完成的行也算进颗数", () => {
    expect(sessionStats(COLOR.steps, 0, cellsBeforeSlice(COLOR.steps[0], 2)).placedCells).toBe(2);
  });

  it("空步数组返回全零，不做除零", () => {
    const empty = sessionStats([], 0);
    expect(empty).toEqual({ totalCells: 0, placedCells: 0, fraction: 0, percent: 0 });
    expect(Number.isNaN(empty.fraction)).toBe(false);
  });

  it("越界的 stepIndex 被夹取，不会算出超过总数的颗数", () => {
    expect(sessionStats(COLOR.steps, 999).placedCells).toBe(9);
    expect(sessionStats(COLOR.steps, -5).placedCells).toBe(0);
  });
});

describe("beadsPerMinute（T-ASM-3）", () => {
  it("正常值四舍五入到整数", () => {
    expect(beadsPerMinute(30, 60_000)).toBe(30);
    expect(beadsPerMinute(10, 45_000)).toBe(13);
  });

  it("还没放豆或还不到一秒时给 null 而不是 0", () => {
    expect(beadsPerMinute(0, 600_000)).toBeNull();
    expect(beadsPerMinute(5, 999)).toBeNull();
    expect(beadsPerMinute(5, 1000)).toBe(300);
  });
});

describe("会话 reducer（T-ASM-4）", () => {
  it("第 0 步不能再撤销", () => {
    const state = start(COLOR);
    expect(sessionReduce(state, { kind: "undo" }, COLOR)).toBe(state);
  });

  it("末步之后进入完成态，且不会越过 steps.length", () => {
    const done = run(start(COLOR), COLOR, [{ kind: "advance" }, { kind: "advance" }]);
    expect(done.stepIndex).toBe(COLOR.steps.length);
    expect(sessionReduce(done, { kind: "advance" }, COLOR)).toBe(done);
  });

  it("完成态还能撤销回最后一步", () => {
    const done = run(start(COLOR), COLOR, [{ kind: "advance" }, { kind: "advance" }]);
    expect(sessionReduce(done, { kind: "undo" }, COLOR).stepIndex).toBe(COLOR.steps.length - 1);
  });

  it("切换模式把游标归零、保留 elapsedMs（D-ASM-4）", () => {
    const moved = run(start(COLOR), COLOR, [
      { kind: "tick", deltaMs: 90_000 },
      { kind: "advance" },
    ]);
    expect(moved.stepIndex).toBe(1);

    const switched = sessionReduce(moved, { kind: "switchMode", mode: "row-by-row" }, COLOR);
    expect(switched.mode).toBe("row-by-row");
    expect(switched.stepIndex).toBe(0);
    expect(switched.elapsedMs).toBe(90_000);
  });

  it("切到同一个模式是空操作", () => {
    const state = start(COLOR);
    expect(sessionReduce(state, { kind: "switchMode", mode: "color-by-color" }, COLOR)).toBe(state);
  });

  it("行窗口：推进走行、走完跨步、撤销整步回退（D-ASM-7）", () => {
    expect(stepRows(COLOR.steps[0])).toEqual([0, 1, 2]);
    const locked = sessionReduce(start(COLOR), { kind: "toggleRowLock" }, COLOR);
    expect(locked.rowLock).toBe(true);

    const oneRow = sessionReduce(locked, { kind: "advance" }, COLOR);
    expect(oneRow).toMatchObject({ stepIndex: 0, rowSlice: 1 });

    const twoRows = sessionReduce(oneRow, { kind: "advance" }, COLOR);
    expect(twoRows).toMatchObject({ stepIndex: 0, rowSlice: 2 });

    const nextStep = sessionReduce(twoRows, { kind: "advance" }, COLOR);
    expect(nextStep).toMatchObject({ stepIndex: 1, rowSlice: 0 });

    // 步内回退走行，步首回退是整步。
    expect(sessionReduce(twoRows, { kind: "undo" }, COLOR)).toMatchObject({ rowSlice: 1 });
    expect(sessionReduce(nextStep, { kind: "undo" }, COLOR)).toMatchObject({
      stepIndex: 0,
      rowSlice: 0,
    });
  });

  it("逐行模式下行窗口不生效：一次推进就是一整步", () => {
    const locked = sessionReduce(start(ROWS), { kind: "toggleRowLock" }, ROWS);
    expect(sessionReduce(locked, { kind: "advance" }, ROWS)).toMatchObject({
      stepIndex: 1,
      rowSlice: 0,
    });
  });

  it("关掉行窗口或换模式时 slice 归零", () => {
    const inRow = run(start(COLOR), COLOR, [{ kind: "toggleRowLock" }, { kind: "advance" }]);
    expect(inRow.rowSlice).toBe(1);
    expect(sessionReduce(inRow, { kind: "toggleRowLock" }, COLOR).rowSlice).toBe(0);
    expect(
      sessionReduce(inRow, { kind: "switchMode", mode: "tile" }, COLOR).rowSlice,
    ).toBe(0);
  });

  it("tick 只加时间，不动游标", () => {
    const ticked = sessionReduce(start(COLOR), { kind: "tick", deltaMs: 1000 }, COLOR);
    expect(ticked).toMatchObject({ elapsedMs: 1000, stepIndex: 0 });
  });

  it("越界的游标在建会话时就被夹取（T-ASM-12 的纯逻辑面）", () => {
    const restored = createSessionState(
      { mode: "row-by-row", stepIndex: 999, elapsedMs: 4_000 },
      ROWS.steps,
    );
    expect(restored.stepIndex).toBe(ROWS.steps.length);
    expect(restored.elapsedMs).toBe(4_000);
  });
});

describe("里程碑与色号播报（T-ASM-5）", () => {
  it("向上穿越各报一次，撤销后再穿越不重报", () => {
    const first = sessionReduce(start(COLOR), { kind: "advance" }, COLOR);
    expect(first.announcement).toContain("已完成 25%");
    expect(first.announcedMilestones).toEqual([25]);

    const back = sessionReduce(first, { kind: "undo" }, COLOR);
    expect(back.announcement).toBe("");
    expect(back.announcedMilestones).toEqual([25]);

    const again = sessionReduce(back, { kind: "advance" }, COLOR);
    expect(again.announcement).not.toContain("25%");
  });

  it("一次跨过多个里程碑只播报最高的那个，但都记为已报", () => {
    const done = run(start(COLOR), COLOR, [{ kind: "advance" }, { kind: "advance" }]);
    expect(done.announcement).toContain("已完成 100%");
    expect(done.announcedMilestones).toEqual([25, 50, 75, 100]);
  });

  it("恢复进场把已达里程碑预标为已报", () => {
    const restored = createSessionState(
      { mode: "color-by-color", stepIndex: 1, elapsedMs: 0 },
      COLOR.steps,
    );
    expect(restored.announcedMilestones).toEqual([25]);
    expect(restored.announcement).toBe("");
    expect(sessionReduce(restored, { kind: "advance" }, COLOR).announcement).not.toContain("25%");
  });

  it("只有单色模式播报「色号完成」（D-ASM-9）", () => {
    expect(sessionReduce(start(COLOR), { kind: "advance" }, COLOR).announcement).toContain(
      "色号 H02 完成",
    );
    const tile = contextFor("tile");
    expect(sessionReduce(start(tile), { kind: "advance" }, tile).announcement).not.toContain("色号");
  });

  it("reachedMilestones 只回已达到的", () => {
    expect(reachedMilestones(0)).toEqual([]);
    expect(reachedMilestones(74)).toEqual([25, 50]);
    expect(reachedMilestones(100)).toEqual([25, 50, 75, 100]);
  });
});

describe("展示用文案", () => {
  it("计时 1 小时以内是 mm:ss，超过转 h:mm:ss", () => {
    expect(formatDuration(0)).toBe("00:00");
    expect(formatDuration(65_000)).toBe("01:05");
    expect(formatDuration(3_725_000)).toBe("1:02:05");
  });

  it("每种模式都给出颜色以外的文字线索（a11y）", () => {
    expect(describeStep(COLOR.steps[0], PALETTE)).toBe("色号 H02");
    expect(describeStep(ROWS.steps[1], PALETTE)).toBe("第 2 行");
    expect(describeStep(contextFor("tile").steps[0], PALETTE)).toBe("第 1 块");
    expect(describeStep(contextFor("outline-infill").steps[0], PALETTE)).toContain("描边");
    expect(describeStep(undefined, PALETTE)).toBe("全部步骤已完成");
  });
});

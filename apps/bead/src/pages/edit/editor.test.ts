import { describe, expect, it } from "vitest";

import { createGrid, type Cell, type Grid } from "../../algo/grid.ts";
import {
  DEFAULT_COLOR,
  MAX_HISTORY,
  applyChanges,
  createEditorState,
  editorReduce,
  floodChanges,
  linePoints,
  replaceChanges,
  symmetryPoints,
  type EditorAction,
  type EditorState,
  type Point,
} from "./editor.ts";

/** T-ED-1…5: the editing operations, with no DOM and no storage in the room. */

function grid(width: number, height: number, cells?: readonly Cell[]): Grid {
  return createGrid(width, height, cells);
}

function run(state: EditorState, ...actions: readonly EditorAction[]): EditorState {
  return actions.reduce(editorReduce, state);
}

function stroke(state: EditorState, ...points: readonly Point[]): EditorState {
  const [first, ...rest] = points;
  if (first === undefined) return state;
  return run(
    state,
    { kind: "strokeBegin", point: first },
    ...(rest.length === 0 ? [] : [{ kind: "strokeTo" as const, points: rest }]),
    { kind: "strokeEnd" },
  );
}

function painted(state: EditorState): (Cell | undefined)[] {
  return [...state.grid.cells];
}

describe("T-ED-1 对称落点（D-ED-9）", () => {
  const board = grid(4, 4);

  it("关：一笔一格", () => {
    expect(symmetryPoints("none", board, { x: 1, y: 2 })).toEqual([{ x: 1, y: 2 }]);
  });

  it("左右镜像用 width-1-x，上下镜像用 height-1-y", () => {
    expect(symmetryPoints("x", board, { x: 0, y: 1 })).toEqual([
      { x: 0, y: 1 },
      { x: 3, y: 1 },
    ]);
    expect(symmetryPoints("y", board, { x: 1, y: 0 })).toEqual([
      { x: 1, y: 0 },
      { x: 1, y: 3 },
    ]);
  });

  it("四向给四格，且互不重复", () => {
    expect(symmetryPoints("quad", board, { x: 0, y: 0 })).toEqual([
      { x: 0, y: 0 },
      { x: 3, y: 0 },
      { x: 0, y: 3 },
      { x: 3, y: 3 },
    ]);
  });

  it("28 与 56 都是偶数宽：镜像轴在列间，没有自映射格", () => {
    for (const size of [28, 56]) {
      const board = grid(size, size);
      for (let x = 0; x < size; x += 1) {
        expect(symmetryPoints("x", board, { x, y: 0 })).toHaveLength(2);
      }
    }
  });

  it("奇数宽的中列会自映射，落点集合去重后只剩一格", () => {
    expect(symmetryPoints("quad", grid(3, 3), { x: 1, y: 1 })).toEqual([{ x: 1, y: 1 }]);
  });

  it("笔画跨格时用直线补齐，不留空洞", () => {
    expect(linePoints({ x: 0, y: 0 }, { x: 3, y: 0 })).toEqual([
      { x: 0, y: 0 },
      { x: 1, y: 0 },
      { x: 2, y: 0 },
      { x: 3, y: 0 },
    ]);
    expect(linePoints({ x: 0, y: 0 }, { x: 2, y: 2 })).toEqual([
      { x: 0, y: 0 },
      { x: 1, y: 1 },
      { x: 2, y: 2 },
    ]);
  });

  it("四向对称的一笔真的落四格", () => {
    const state = run(createEditorState(grid(4, 4)), {
      kind: "selectSymmetry",
      symmetry: "quad",
    });
    const after = stroke(state, { x: 0, y: 0 });
    expect(painted(after).filter((cell) => cell !== null)).toHaveLength(4);
    expect(after.past).toHaveLength(1);
  });
});

describe("T-ED-2 油漆桶（D-ED-10）", () => {
  // 对角相连、四连通不相连：中心 0 的上下左右都是空。
  const diagonal = grid(3, 3, [0, null, 0, null, 0, null, 0, null, 0]);

  it("4-连通不对角渗漏", () => {
    expect(floodChanges(diagonal, { x: 1, y: 1 }, 7)).toHaveLength(1);
  });

  it("空区可灌：整块空板一次灌满", () => {
    expect(floodChanges(grid(2, 2), { x: 0, y: 0 }, 3)).toHaveLength(4);
  });

  it("「空」作为填充值就是区域清除", () => {
    const filled = grid(2, 2, [4, 4, 4, 4]);
    const changes = floodChanges(filled, { x: 1, y: 1 }, null);
    expect(changes).toHaveLength(4);
    expect(applyChanges(filled, changes).cells.every((cell) => cell === null)).toBe(true);
  });

  it("种子值 == 活动色 ⇒ no-op，且不产生历史条目", () => {
    const state = createEditorState(grid(2, 2, [DEFAULT_COLOR, null, null, null]));
    const after = editorReduce(state, { kind: "fill", point: { x: 0, y: 0 } });
    expect(after).toBe(state);
    expect(after.past).toHaveLength(0);
  });

  it("板边就是边界，灌注不越出网格", () => {
    const board = grid(3, 2, [null, 1, null, null, 1, null]);
    const changes = floodChanges(board, { x: 0, y: 0 }, 2);
    expect(changes.map((change) => change.index).sort((a, b) => a - b)).toEqual([0, 3]);
  });

  it("对称档不影响油漆桶（D-ED-9：区域语义与镜像语义正交）", () => {
    const state = run(createEditorState(grid(4, 1, [null, 5, null, null])), {
      kind: "selectSymmetry",
      symmetry: "x",
    });
    const after = editorReduce(state, { kind: "fill", point: { x: 0, y: 0 } });
    expect(painted(after)).toEqual([DEFAULT_COLOR, 5, null, null]);
  });
});

describe("T-ED-3 全局替换（D-ED-12）", () => {
  const board = grid(2, 2, [1, 2, 1, null]);

  it("改写颗数正确，其余码不动", () => {
    const changes = replaceChanges(board, 1, 9);
    expect(changes).toHaveLength(2);
    expect(applyChanges(board, changes).cells).toEqual([9, 2, 9, null]);
  });

  it("目标「空」就是擦除", () => {
    expect(applyChanges(board, replaceChanges(board, 1, null)).cells).toEqual([null, 2, null, null]);
  });

  it("from == to 被拒绝，不留历史", () => {
    const state = createEditorState(board);
    expect(replaceChanges(board, 1, 1)).toEqual([]);
    expect(editorReduce(state, { kind: "replace", from: 1, to: 1 })).toBe(state);
  });

  it("文档里没有的码替换 0 颗，也不留历史", () => {
    const state = createEditorState(board);
    expect(editorReduce(state, { kind: "replace", from: 40, to: 3 })).toBe(state);
  });
});

describe("T-ED-4 撤销 / 重做（D-ED-14）", () => {
  const board = grid(4, 4);

  it("整笔画是一条历史，undo/redo 往返回到逐格一致", () => {
    const before = painted(createEditorState(board));
    const drawn = stroke(createEditorState(board), { x: 0, y: 0 }, { x: 1, y: 0 }, { x: 2, y: 0 });
    expect(drawn.past).toHaveLength(1);
    expect(painted(drawn).filter((cell) => cell !== null)).toHaveLength(3);

    const undone = editorReduce(drawn, { kind: "undo" });
    expect(painted(undone)).toEqual(before);
    expect(painted(editorReduce(undone, { kind: "redo" }))).toEqual(painted(drawn));
  });

  it("同一格在一笔里画两次仍然只回退到落笔前", () => {
    const start = createEditorState(grid(2, 1, [3, null]));
    const drawn = run(
      start,
      { kind: "strokeBegin", point: { x: 0, y: 0 } },
      { kind: "strokeTo", points: [{ x: 1, y: 0 }, { x: 0, y: 0 }] },
      { kind: "strokeEnd" },
    );
    expect(painted(editorReduce(drawn, { kind: "undo" }))).toEqual([3, null]);
  });

  it("落笔后没有任何格子改变时不产生历史条目", () => {
    const start = createEditorState(grid(1, 1, [DEFAULT_COLOR]));
    const after = stroke(start, { x: 0, y: 0 });
    expect(after.past).toHaveLength(0);
    expect(after.stroke).toBeNull();
  });

  it("新操作清空 redo 栈", () => {
    const drawn = stroke(createEditorState(board), { x: 0, y: 0 });
    const undone = editorReduce(drawn, { kind: "undo" });
    expect(undone.future).toHaveLength(1);
    expect(stroke(undone, { x: 3, y: 3 }).future).toHaveLength(0);
  });

  it("第 101 条进栈时最旧的那条被丢掉", () => {
    let state = createEditorState(grid(12, 12));
    for (let i = 0; i <= MAX_HISTORY; i += 1) {
      state = stroke(state, { x: i % 12, y: Math.floor(i / 12) });
    }
    expect(state.past).toHaveLength(MAX_HISTORY);

    for (let i = 0; i < MAX_HISTORY; i += 1) state = editorReduce(state, { kind: "undo" });
    expect(state.past).toHaveLength(0);
    // 被丢掉的那一条再也回不来：第一笔留在板上。
    expect(state.grid.cells[0]).toBe(DEFAULT_COLOR);
    expect(painted(state).filter((cell) => cell !== null)).toHaveLength(1);
  });

  it("栈空时 undo / redo 都是 no-op", () => {
    const state = createEditorState(board);
    expect(editorReduce(state, { kind: "undo" })).toBe(state);
    expect(editorReduce(state, { kind: "redo" })).toBe(state);
  });

  it("油漆桶与替换各自是一条历史", () => {
    const state = createEditorState(grid(2, 2));
    const filled = editorReduce(state, { kind: "fill", point: { x: 0, y: 0 } });
    const replaced = editorReduce(filled, { kind: "replace", from: DEFAULT_COLOR, to: 1 });
    expect(replaced.past).toHaveLength(2);
    expect(painted(editorReduce(replaced, { kind: "undo" }))).toEqual(painted(filled));
  });
});

describe("T-ED-5 拾色器（D-ED-11）", () => {
  const state = run(createEditorState(grid(2, 1, [7, null])), {
    kind: "selectTool",
    tool: "eyedropper",
  });

  it("取到码：活动色更新且工具回画笔", () => {
    const after = editorReduce(state, { kind: "pick", point: { x: 0, y: 0 } });
    expect(after.activeColor).toBe(7);
    expect(after.tool).toBe("brush");
    expect(after.past).toHaveLength(0);
    expect(after.revision).toBe(0);
  });

  it("取到空格：活动色变「空」，顺手就是橡皮", () => {
    const after = editorReduce(state, { kind: "pick", point: { x: 1, y: 0 } });
    expect(after.activeColor).toBeNull();
    expect(after.tool).toBe("brush");
  });
});

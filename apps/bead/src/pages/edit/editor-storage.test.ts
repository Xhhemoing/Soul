import { describe, expect, it } from "vitest";
import { IDBFactory } from "fake-indexeddb";

import { createGrid } from "../../algo/grid.ts";
import { STORE, readStoreEntries } from "../../stores/bead-v1.ts";
import { mintProjectId } from "../../stores/ids.ts";
import { createPatternDoc, decodeCells, toPatternDoc } from "../../stores/patterns.ts";
import { createProjectFromBlank } from "../../stores/projects.ts";
import { createRepository } from "../../stores/repository.ts";
import { STORAGE_KEY } from "../../stores/persisted.ts";
import { FlakyStorage } from "../../test/flaky-storage.ts";
import { createEditorState, editorReduce, type EditorState } from "./editor.ts";

/**
 * T-ED-6 / T-ED-7: the editor's side of the `bead-v1` contract. The document it
 * writes is the same document B03 writes — same codec, same read-back check —
 * and an editing round adds no second place for a grid to live.
 *
 * Every case brings its own `IDBFactory` so the databases cannot bleed into one
 * another, and `FlakyStorage` stands in for the legacy localStorage source so a
 * migration cannot eat another case's key (the `repository.test.ts` pattern).
 */

function beadV1() {
  const factory = new IDBFactory();
  return { factory, repo: createRepository(new FlakyStorage(), factory) };
}

/** One editing round: fill the board, erase a corner, undo the erase. */
function edited(state: EditorState): EditorState {
  const filled = editorReduce(state, { kind: "fill", point: { x: 0, y: 0 } });
  const erased = editorReduce(
    { ...filled, activeColor: null },
    { kind: "fill", point: { x: 0, y: 0 } },
  );
  return editorReduce(erased, { kind: "undo" });
}

describe("T-ED-6 空白文档往返（D-ED-4 / D-UP-12）", () => {
  it("全 -1、无 provenance 的空白文档过 bead-v1 往返不变，且过读回校验", async () => {
    const { repo } = beadV1();
    const projectId = mintProjectId();
    const doc = createPatternDoc(projectId, createGrid(28, 28));

    expect(doc.provenance).toBeUndefined();
    expect(doc.paletteId).toBe("generic-5mm");
    expect([...doc.cells].every((cell) => cell === -1)).toBe(true);
    expect(toPatternDoc(doc)).not.toBeNull();

    await repo.savePatternDoc(doc);
    const back = await repo.loadPatternDoc(projectId);

    expect(back).toEqual(doc);
    expect(back?.cells).toBeInstanceOf(Int16Array);
    expect(decodeCells(back!).cells.every((cell) => cell === null)).toBe(true);
  });

  it("编辑后的文档同样往返，重开编辑器读到的是同一张网格", async () => {
    const { repo } = beadV1();
    const projectId = mintProjectId();
    await repo.savePatternDoc(createPatternDoc(projectId, createGrid(6, 6)));

    const opened = createEditorState(decodeCells((await repo.loadPatternDoc(projectId))!));
    const after = edited(opened);
    await repo.savePatternDoc(createPatternDoc(projectId, after.grid));

    const reopened = createEditorState(decodeCells((await repo.loadPatternDoc(projectId))!));
    expect(reopened.grid.cells).toEqual(after.grid.cells);
    expect(reopened.past).toHaveLength(0); // D-ED-14：历史永不持久化
  });

  it("转换文档被编辑后 provenance 原样保留（D-ED-2 / R-ED-4）", async () => {
    const { repo } = beadV1();
    const projectId = mintProjectId();
    const provenance = { kind: "Photo", ditherApplied: true } as const;
    await repo.savePatternDoc(createPatternDoc(projectId, createGrid(4, 4), provenance));

    const opened = createEditorState(decodeCells((await repo.loadPatternDoc(projectId))!));
    await repo.savePatternDoc(createPatternDoc(projectId, edited(opened).grid, provenance));

    expect((await repo.loadPatternDoc(projectId))?.provenance).toEqual(provenance);
  });
});

describe("T-ED-7 存储卫生（BD19 / D-ED-14 / D-INV-13）", () => {
  it("编辑一轮后：网格只在 patterns 仓一份，撤销栈与 BOM 一个字节都没落盘", async () => {
    const legacy = new FlakyStorage();
    const factory = new IDBFactory();
    const repo = createRepository(legacy, factory);

    const projectId = mintProjectId();
    const project = createProjectFromBlank(projectId, "空白板");
    await repo.savePatternDoc(createPatternDoc(projectId, createGrid(8, 8)));
    await repo.saveProjects([project]);

    const opened = createEditorState(decodeCells((await repo.loadPatternDoc(projectId))!));
    await repo.savePatternDoc(createPatternDoc(projectId, edited(opened).grid));

    const patterns = await readStoreEntries(factory, STORE.patterns);
    expect(patterns).toHaveLength(1);
    expect(Object.keys(patterns[0]!.value as object).sort()).toEqual([
      "cells",
      "height",
      "paletteId",
      "projectId",
      "width",
    ]);

    // BD19/BD20: 没有第二份网格。state 仓只装元数据，progress 仓一行都没有。
    const state = await readStoreEntries(factory, STORE.state);
    const everything = JSON.stringify(state);
    for (const banned of ["cells", "grid", "steps", "bom", "past", "future", "stroke", "undo"]) {
      expect(everything).not.toContain(banned);
    }
    expect(await readStoreEntries(factory, STORE.progress)).toEqual([]);

    // localStorage 里既没有旧键，也没有任何网格（BD19 的红线）。
    expect(legacy.getItem(STORAGE_KEY)).toBeNull();
    expect(globalThis.localStorage.getItem(STORAGE_KEY)).toBeNull();
  });
});

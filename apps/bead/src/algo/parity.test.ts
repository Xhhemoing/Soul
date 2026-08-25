import { describe, expect, it } from "vitest";

import fixture from "./fixtures/parity.json";
import { createImage } from "./image.ts";
import { entryAt, GENERIC_5MM } from "./palette.ts";
import { imageToPattern, type PipelineOptions } from "./pipeline.ts";
import { splitSteps, type SplitMode } from "./steps.ts";

/**
 * `fixtures/parity.json` locks the TypeScript pipeline's own path: the
 * detector's verdict, the undone up-scale, aspect framing and the four step
 * modes, none of which the oracle expresses. Cross-language colour-code parity
 * lives in `oracle-parity.test.ts`, which replays
 * `crates/bead-core/fixtures/parity/*.json` unchanged (AT-1).
 *
 * Nothing here decodes a PNG either way, because a browser will apply an
 * embedded ICC profile and the Rust `image` crate will not (T-PAR-2).
 */
function decodeHex(hex: string): Uint8ClampedArray {
  const bytes = new Uint8ClampedArray(hex.length / 2);
  for (let i = 0; i < bytes.length; i += 1) {
    bytes[i] = Number.parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return bytes;
}

describe("T-PAR-1 TS 管线回归锁", () => {
  it("覆盖像素图、照片（抖动开/关）与含透明四条路径", () => {
    expect(fixture.palette).toBe(GENERIC_5MM.id);
    expect(fixture.cases.map((it) => it.name)).toEqual([
      "pixel-art-upscaled-4x",
      "photo-no-dither",
      "photo-dither",
      "transparent-hole",
    ]);
  });

  it.each(fixture.cases.map((it) => [it.name, it] as const))("%s 色号序列逐元素全等", (_name, item) => {
    const image = createImage(item.width, item.height, decodeHex(item.rgbaHex));
    const result = imageToPattern(image, item.options as PipelineOptions);

    expect(result.kind).toBe(item.expected.kind);
    expect(result.ditherApplied).toBe(item.expected.ditherApplied);
    expect(result.detectedGrid).toEqual(item.expected.detectedGrid);
    expect(result.grid.width).toBe(item.expected.gridWidth);
    expect(result.grid.height).toBe(item.expected.gridHeight);

    const codes = result.grid.cells.map((cell) =>
      cell === null ? null : entryAt(GENERIC_5MM, cell).id,
    );
    expect(codes).toEqual(item.expected.codes);
    expect(result.bom.map((row) => ({ code: row.code, count: row.count }))).toEqual(
      item.expected.bom,
    );
  });

  it.each(fixture.cases.map((it) => [it.name, it] as const))("%s 四模式步骤序列全等", (_name, item) => {
    const image = createImage(item.width, item.height, decodeHex(item.rgbaHex));
    const grid = imageToPattern(image, item.options as PipelineOptions).grid;

    for (const [mode, expected] of Object.entries(item.expected.steps)) {
      const actual = splitSteps(grid, mode as SplitMode).map((step) => ({
        part: step.part,
        group: step.group,
        color: step.color === null ? null : entryAt(GENERIC_5MM, step.color).id,
        count: step.cells.length,
        first: `${step.cells[0]!.x},${step.cells[0]!.y}`,
        last: `${step.cells[step.cells.length - 1]!.x},${step.cells[step.cells.length - 1]!.y}`,
      }));
      expect(actual).toEqual(expected);
    }
  });
});

describe("T-PAR-3 近平局哨兵", () => {
  it.each(fixture.cases.map((it) => [it.name, it] as const))(
    "%s 每个像素的次优 ΔE 余量都 > 1e-6",
    (_name, item) => {
      const image = createImage(item.width, item.height, decodeHex(item.rgbaHex));
      const result = imageToPattern(image, item.options as PipelineOptions);
      expect(result.minRunnerUpMargin).toBeGreaterThan(fixture.deltaEMarginFloor);
      // The recorded margin is part of the fixture: if it moves, the fixture is
      // no longer describing the pipeline that produced it.
      expect(result.minRunnerUpMargin).toBeCloseTo(item.minRunnerUpMargin, 10);
    },
  );

  it("fixture 自带的余量都在下限之上", () => {
    for (const item of fixture.cases) {
      expect(item.minRunnerUpMargin).toBeGreaterThan(fixture.deltaEMarginFloor);
    }
    expect(fixture.deltaEMarginFloor).toBe(1e-6);
  });
});

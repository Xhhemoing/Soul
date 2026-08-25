import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";

import { buildBom } from "./bom.ts";
import { quantize } from "./dither.ts";
import { planFixedBoards, renderFit, type BoardSpec, type Sampling } from "./framing.ts";
import { createImage } from "./image.ts";
import { entryAt, GENERIC_5MM } from "./palette.ts";
import { imageToPattern } from "./pipeline.ts";

/**
 * AT-1. `crates/bead-core/fixtures/parity/*.json` is the oracle's own output,
 * written by `bead_core::parity::build`. BD18/BD8 make `bead-core` the
 * algorithm oracle, so these files are read here as-is — not regenerated, not
 * translated — and the acceptance bar is that the same raw RGBA in gives the
 * same colour-code sequence out.
 *
 * This test deliberately bypasses `imageToPattern`: the oracle's generator runs
 * `plan` → `render` → `map_image_traced` with the fixture's own fit, sampling
 * and dither, and never consults the PixelArt/Photo detector. Going through the
 * detector here would compare a different pipeline.
 */
const ORACLE_FIXTURES = join(process.cwd(), "..", "..", "crates", "bead-core", "fixtures", "parity");

interface OracleFixture {
  readonly case: string;
  readonly palette: string;
  readonly fit: { readonly mode: string; readonly board: BoardSpec; readonly cols: number; readonly rows: number };
  readonly sampling: Sampling;
  readonly dither: "none" | "floyd-steinberg";
  readonly source: { readonly width: number; readonly height: number; readonly rgba: number[] };
  readonly expected: {
    readonly width: number;
    readonly height: number;
    /** Empty string is the oracle's spelling of an empty cell. */
    readonly codes: string[];
    readonly bom: { readonly code: string; readonly name: string; readonly count: number }[];
  };
}

const CASES = ["pixel-art", "photo-flat", "photo-dithered", "with-transparency"] as const;

function load(name: string): OracleFixture {
  return JSON.parse(readFileSync(join(ORACLE_FIXTURES, `${name}.json`), "utf8")) as OracleFixture;
}

function run(fixture: OracleFixture) {
  const image = createImage(fixture.source.width, fixture.source.height, fixture.source.rgba);
  const plan = planFixedBoards(
    image.width,
    image.height,
    fixture.fit.board,
    fixture.fit.cols,
    fixture.fit.rows,
  );
  const framed = renderFit(image, plan, fixture.sampling);
  const { grid } = quantize(framed, {
    palette: GENERIC_5MM,
    dither: fixture.dither === "floyd-steinberg",
  });
  return { grid, bom: buildBom(grid, GENERIC_5MM) };
}

describe("AT-1 与 bead-core 共用同一份 parity fixture", () => {
  it("色板逐条与 oracle 相同：48 色、同码同名同 RGB", () => {
    const source = readFileSync(
      join(process.cwd(), "..", "..", "crates", "bead-core", "src", "palette.rs"),
      "utf8",
    );
    const oracle = [...source.matchAll(/\("(G\d\d)", "([^"]+)", "#([0-9A-F]{6})"\)/g)].map(
      (match) => ({ id: match[1]!, displayName: match[2]!, hex: match[3]! }),
    );

    expect(oracle.length).toBe(48);
    expect(GENERIC_5MM.entries.length).toBe(oracle.length);
    expect(
      GENERIC_5MM.entries.map((item) => ({
        id: item.id,
        displayName: item.displayName,
        hex: [item.rgb.r, item.rgb.g, item.rgb.b]
          .map((c) => c.toString(16).padStart(2, "0"))
          .join("")
          .toUpperCase(),
      })),
    ).toEqual(oracle);
  });

  it.each(CASES)("%s 的每个 fixture 都声明 generic-5mm 与固定拼板", (name) => {
    const fixture = load(name);
    expect(fixture.palette).toBe(GENERIC_5MM.id);
    expect(fixture.fit.mode).toBe("fixed-boards");
  });

  it.each(CASES)("%s 色号序列逐格与 oracle 全等", (name) => {
    const fixture = load(name);
    const { grid } = run(fixture);

    expect(grid.width).toBe(fixture.expected.width);
    expect(grid.height).toBe(fixture.expected.height);
    expect(grid.cells.map((cell) => (cell === null ? "" : entryAt(GENERIC_5MM, cell).id))).toEqual(
      fixture.expected.codes,
    );
  });

  it.each(CASES)("%s 走完整 imageToPattern 也得到同一序列", (name) => {
    const fixture = load(name);
    const image = createImage(fixture.source.width, fixture.source.height, fixture.source.rgba);
    // `kind: "Photo"` is not a claim about the picture: it is how the caller
    // asks for the oracle's pipeline, which has no detector and never undoes an
    // up-scale. The fixture's own dither flag then survives to quantisation.
    const result = imageToPattern(image, {
      framing: {
        mode: "fixed-boards",
        board: fixture.fit.board,
        cols: fixture.fit.cols,
        rows: fixture.fit.rows,
        sampling: fixture.sampling,
      },
      kind: "Photo",
      dither: fixture.dither === "floyd-steinberg",
    });

    expect(result.ditherApplied).toBe(fixture.dither === "floyd-steinberg");
    expect(
      result.grid.cells.map((cell) => (cell === null ? "" : entryAt(GENERIC_5MM, cell).id)),
    ).toEqual(fixture.expected.codes);
  });

  it.each(CASES)("%s BOM 的码、名与颗数也全等", (name) => {
    const fixture = load(name);
    const { bom } = run(fixture);

    expect(
      bom.map((row) => ({ code: row.code, name: row.displayName, count: row.count })),
    ).toEqual(fixture.expected.bom);
  });
});

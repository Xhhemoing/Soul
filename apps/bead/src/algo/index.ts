/**
 * WP-B03 — the browser side of the bead pipeline.
 *
 * `crates/bead-core` (WP-B01) is the oracle (BD18/BD8): the acceptance bar is
 * that the same fixture yields the same *colour-code* sequence on both sides,
 * compared as codes and never as raw ΔE floats, and a disagreement is settled
 * by moving this side. `oracle-parity.test.ts` replays the oracle's own
 * fixtures to hold that line. Every choice this port had to make, and every
 * semantic still unconverged, is written down in `contract.md`, keyed to gaps
 * G1–G8 of `docs/bead/reviews/round1-algorithms.md`.
 */

export {
  ciede2000,
  ciede2000Rgb,
  clamp,
  linearToSrgb,
  rgbToLab,
  roundHalfUp,
  srgbToLinear,
  SRGB_TO_XYZ_D65,
  WHITE_POINT_D65,
  type Lab,
  type Rgb,
} from "./color.ts";

export {
  AlgoError,
  createImage,
  imageFromPixels,
  isOpaque,
  OPAQUE_ALPHA_THRESHOLD,
  readAlpha,
  readRgb,
  type AlgoErrorCode,
  type RgbaImage,
} from "./image.ts";

export {
  entryAt,
  GENERIC_5MM,
  nearestEntry,
  nearestIndex,
  preparePalette,
  type ColorIndex,
  type NearestMatch,
  type Palette,
  type PaletteEntry,
  type PreparedPalette,
} from "./palette.ts";

export {
  cellAt,
  createGrid,
  isEmptyGrid,
  occupiedCount,
  rowMajorCells,
  type Cell,
  type CellRef,
  type Grid,
} from "./grid.ts";

export { quantize, type QuantizeOptions, type QuantizeResult } from "./dither.ts";

export {
  classificationFeatures,
  classifyImage,
  detectGrid,
  DEGENERATE_CONFIDENCE,
  MAX_DETECTED_CELL,
  PIXEL_ART_COLOR_CEILING,
  PIXEL_ART_THRESHOLD,
  type Classification,
  type ClassificationFeatures,
  type GridGeometry,
  type ImageKind,
} from "./classify.ts";

export {
  applyFraming,
  aspectTarget,
  BOARD_28,
  BOARD_56,
  collapseLattice,
  cropImage,
  DEFAULT_MAX_SIDE,
  latticeCells,
  planFixedBoards,
  renderFit,
  resampleBox,
  resampleNearest,
  type BoardSpec,
  type CellLattice,
  type CropRect,
  type FitPlan,
  type Framing,
  type Sampling,
  type SourceRect,
} from "./framing.ts";

export {
  colorByColor,
  outlineInfill,
  rowByRow,
  splitSteps,
  SPLIT_MODES,
  tileSplit,
  type ColorByColorOptions,
  type SplitMode,
  type SplitOptions,
  type Step,
  type StepPart,
} from "./steps.ts";

export { buildBom, totalBeads, type BomRow } from "./bom.ts";

export {
  findSubstitutes,
  substitutesForLab,
  SUBSTITUTE_MAX_DELTA_E,
  type InventoryColor,
  type SubstituteCandidate,
  type SubstituteOptions,
} from "./substitutes.ts";

export {
  imageToPattern,
  type PipelineOptions,
  type PipelineResult,
} from "./pipeline.ts";

export { decodeImage, DECODE_OPTIONS } from "./decode.ts";

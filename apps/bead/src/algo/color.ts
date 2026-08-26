/**
 * sRGB → Lab (D65) → CIEDE2000.
 *
 * Contract notes (see `contract.md`, gaps G2 and G5):
 * - The sRGB→XYZ matrix is pinned to the 7-decimal D65 form whose rows sum to
 *   the white point, so pure white lands on a* = b* = 0 without a special case.
 *   A Rust port must copy these digits verbatim; a matrix re-derived from the
 *   primaries diverges in the 4th decimal and flips near-tie nearest-colour
 *   lookups.
 * - ΔE00 follows Sharma, Wu & Dalal (2005) including the h' = 0 convention when
 *   a' = b = 0 and the ±180° hue wrap.
 */

export interface Rgb {
  /** 0–255 sRGB code value. */
  readonly r: number;
  readonly g: number;
  readonly b: number;
}

export interface Lab {
  readonly l: number;
  readonly a: number;
  readonly b: number;
}

/** IEC 61966-2-1 sRGB primaries, chromatically adapted to D65 (rows sum to the white point). */
export const SRGB_TO_XYZ_D65 = [
  [0.4124564, 0.3575761, 0.1804375],
  [0.2126729, 0.7151522, 0.072175],
  [0.0193339, 0.119192, 0.9503041],
] as const;

export const WHITE_POINT_D65 = { x: 0.95047, y: 1.0, z: 1.08883 } as const;

/** CIE 1976 ε = 216/24389 and κ = 24389/27, written as the exact rationals. */
const EPSILON = 216 / 24389;
const KAPPA = 24389 / 27;

const DEG = Math.PI / 180;
const POW25_7 = 6103515625; // 25^7, spelled out so both ports share the literal.

/**
 * Ties go away from zero. Every rounding in the pipeline routes through here so
 * the Rust port has exactly one rule to copy (`(x + 0.5).floor()` for x ≥ 0).
 */
export function roundHalfUp(value: number): number {
  return Math.floor(value + 0.5);
}

export function clamp(value: number, min: number, max: number): number {
  if (value < min) return min;
  if (value > max) return max;
  return value;
}

/** sRGB code value (0–255) → linear-light [0,1]. */
export function srgbToLinear(code: number): number {
  const c = clamp(code, 0, 255) / 255;
  return c <= 0.04045 ? c / 12.92 : Math.pow((c + 0.055) / 1.055, 2.4);
}

/** Linear-light [0,1] → sRGB code value (0–255, rounded half-up). */
export function linearToSrgb(linear: number): number {
  const l = clamp(linear, 0, 1);
  const encoded = l <= 0.0031308 ? 12.92 * l : 1.055 * Math.pow(l, 1 / 2.4) - 0.055;
  return clamp(roundHalfUp(encoded * 255), 0, 255);
}

function pivot(t: number): number {
  return t > EPSILON ? Math.cbrt(t) : (KAPPA * t + 16) / 116;
}

export function rgbToLab(rgb: Rgb): Lab {
  const r = srgbToLinear(rgb.r);
  const g = srgbToLinear(rgb.g);
  const b = srgbToLinear(rgb.b);

  const [mx, my, mz] = SRGB_TO_XYZ_D65;
  const x = (mx[0] * r + mx[1] * g + mx[2] * b) / WHITE_POINT_D65.x;
  const y = (my[0] * r + my[1] * g + my[2] * b) / WHITE_POINT_D65.y;
  const z = (mz[0] * r + mz[1] * g + mz[2] * b) / WHITE_POINT_D65.z;

  const fx = pivot(x);
  const fy = pivot(y);
  const fz = pivot(z);

  return { l: 116 * fy - 16, a: 500 * (fx - fy), b: 200 * (fy - fz) };
}

/** Hue angle in degrees, [0,360); a' = b = 0 collapses to 0 (Sharma's convention). */
function hueAngle(a: number, b: number): number {
  if (a === 0 && b === 0) return 0;
  const deg = Math.atan2(b, a) / DEG;
  return deg < 0 ? deg + 360 : deg;
}

export function ciede2000(first: Lab, second: Lab): number {
  const c1 = Math.sqrt(first.a * first.a + first.b * first.b);
  const c2 = Math.sqrt(second.a * second.a + second.b * second.b);
  const cBar = (c1 + c2) / 2;
  const cBar7 = Math.pow(cBar, 7);
  const gFactor = 0.5 * (1 - Math.sqrt(cBar7 / (cBar7 + POW25_7)));

  // Only a* is scaled; b* is untouched. Getting this wrong is the classic bug.
  const a1p = (1 + gFactor) * first.a;
  const a2p = (1 + gFactor) * second.a;
  const c1p = Math.sqrt(a1p * a1p + first.b * first.b);
  const c2p = Math.sqrt(a2p * a2p + second.b * second.b);
  const h1p = hueAngle(a1p, first.b);
  const h2p = hueAngle(a2p, second.b);

  const dLp = second.l - first.l;
  const dCp = c2p - c1p;

  let dhp = 0;
  if (c1p * c2p !== 0) {
    const raw = h2p - h1p;
    if (raw > 180) dhp = raw - 360;
    else if (raw < -180) dhp = raw + 360;
    else dhp = raw;
  }
  const dHp = 2 * Math.sqrt(c1p * c2p) * Math.sin((dhp / 2) * DEG);

  const lBarP = (first.l + second.l) / 2;
  const cBarP = (c1p + c2p) / 2;

  let hBarP: number;
  if (c1p * c2p === 0) {
    hBarP = h1p + h2p;
  } else {
    const sum = h1p + h2p;
    if (Math.abs(h1p - h2p) <= 180) hBarP = sum / 2;
    else if (sum < 360) hBarP = (sum + 360) / 2;
    else hBarP = (sum - 360) / 2;
  }

  const t =
    1 -
    0.17 * Math.cos((hBarP - 30) * DEG) +
    0.24 * Math.cos(2 * hBarP * DEG) +
    0.32 * Math.cos((3 * hBarP + 6) * DEG) -
    0.2 * Math.cos((4 * hBarP - 63) * DEG);

  const dTheta = 30 * Math.exp(-Math.pow((hBarP - 275) / 25, 2));
  const cBarP7 = Math.pow(cBarP, 7);
  const rc = 2 * Math.sqrt(cBarP7 / (cBarP7 + POW25_7));

  const lBarP50 = (lBarP - 50) * (lBarP - 50);
  const sl = 1 + (0.015 * lBarP50) / Math.sqrt(20 + lBarP50);
  const sc = 1 + 0.045 * cBarP;
  const sh = 1 + 0.015 * cBarP * t;
  const rt = -Math.sin(2 * dTheta * DEG) * rc;

  const termL = dLp / sl;
  const termC = dCp / sc;
  const termH = dHp / sh;

  return Math.sqrt(termL * termL + termC * termC + termH * termH + rt * termC * termH);
}

/** Convenience wrapper: ΔE00 between two sRGB code triples. */
export function ciede2000Rgb(first: Rgb, second: Rgb): number {
  return ciede2000(rgbToLab(first), rgbToLab(second));
}

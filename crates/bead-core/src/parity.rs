//! The shared fixture the browser port is checked against (T-PAR-1 to T-PAR-3).
//!
//! WP-B03 reimplements this crate in TypeScript, and its acceptance test is
//! that both produce the same colour codes for the same input. That only works
//! if "the same input" is unambiguous, so a parity case is raw RGBA bytes and
//! nothing else — no PNG, no JPEG. A browser applies an ICC profile on decode
//! and Rust's decoders do not, which would break the comparison before either
//! implementation had run (T-PAR-2).
//!
//! ## The near-tie sentinel
//!
//! `powf`, `cbrt` and `atan2` may differ in the last place between Rust's libm
//! and JavaScript's `Math`. That is harmless until a pixel's best and
//! second-best beads are separated by less than the error — then the two
//! implementations disagree about which bead it is, and a fixture built on that
//! pixel is a coin toss dressed as a contract. [`ParityFixture::near_ties`]
//! lists any such pixel, and the generator refuses to emit a case that has one
//! (T-PAR-3, G2).
//!
//! ## The format
//!
//! Deliberately plain JSON with no floating-point numbers in it, so that
//! `JSON.parse` and this module's hand-written emitter cannot disagree about
//! rounding either. The emitter lives here rather than behind a dependency
//! because the whole package is dependency-free and the schema is fixed.

use std::fmt::Write as _;

use crate::bom::Bom;
use crate::color::Rgba;
use crate::detect::ImageKind;
use crate::fit::{plan, render, FitError, FitMode, Sampling};
use crate::grid::Cell;
use crate::image::Image;
use crate::palette::Palette;
use crate::quantize::{map_image_traced, Dither, MapOptions, PatternGrid};
use crate::steps::{plan_steps, ColorOrder, StepMode, StepPlan};

/// A decision closer than this could flip between Rust and JavaScript.
pub const NEAR_TIE_MARGIN: f64 = 1e-6;

/// One case: an image, and everything needed to convert it.
#[derive(Debug, Clone, PartialEq)]
pub struct ParityCase {
    pub name: String,
    pub image: Image,
    pub fit: FitMode,
    pub sampling: Sampling,
    pub dither: Dither,
}

/// A pixel whose two best beads are too close to call.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NearTie {
    pub cell: Cell,
    pub margin: f64,
}

/// A built case: the input, the answer, and the sentinel's findings.
#[derive(Debug, Clone, PartialEq)]
pub struct ParityFixture {
    pub name: String,
    pub palette: String,
    pub source: Image,
    pub fit: FitMode,
    pub sampling: Sampling,
    pub dither: Dither,
    pub grid: PatternGrid,
    pub bom: Bom,
    pub plans: Vec<StepPlan>,
    /// Empty in any fixture fit to be committed.
    pub near_ties: Vec<NearTie>,
}

/// The four step modes every fixture records, in a fixed order.
pub fn parity_step_modes() -> Vec<StepMode> {
    vec![
        StepMode::ColorByColor {
            order: ColorOrder::AccentFirst,
        },
        StepMode::Tile {
            board: crate::fit::BoardSpec::square_28(),
        },
        StepMode::OutlineInfill,
        StepMode::RowByRow,
    ]
}

/// Run one case through the pipeline and collect everything the fixture pins.
pub fn build(case: &ParityCase, palette: &Palette) -> Result<ParityFixture, FitError> {
    let fit_plan = plan(case.image.width(), case.image.height(), &case.fit)?;
    let sampled = render(&case.image, &fit_plan, case.sampling);
    let traced = map_image_traced(&sampled, palette, MapOptions::with_dither(case.dither));

    let near_ties = traced
        .margins
        .iter()
        .enumerate()
        .filter_map(|(index, margin)| {
            let margin = (*margin)?;
            (margin <= NEAR_TIE_MARGIN).then(|| NearTie {
                cell: Cell::new(
                    (index as u32) % traced.grid.width(),
                    (index as u32) / traced.grid.width(),
                ),
                margin,
            })
        })
        .collect();

    let bom = Bom::from_grid(&traced.grid, palette);
    let plans = parity_step_modes()
        .iter()
        .map(|mode| plan_steps(&traced.grid, palette, mode))
        .collect();

    Ok(ParityFixture {
        name: case.name.clone(),
        palette: palette.namespace().to_owned(),
        source: case.image.clone(),
        fit: case.fit.clone(),
        sampling: case.sampling,
        dither: case.dither,
        grid: traced.grid,
        bom,
        plans,
        near_ties,
    })
}

impl ParityFixture {
    /// The committed form. Stable across runs and platforms: integers, strings
    /// and arrays only, two-space indent, trailing newline.
    pub fn to_json(&self, palette: &Palette) -> String {
        let mut out = String::new();
        out.push_str("{\n");
        write_field(&mut out, 1, "case", &json_string(&self.name), true);
        write_field(&mut out, 1, "palette", &json_string(&self.palette), true);
        write_field(&mut out, 1, "fit", &self.fit_json(), true);
        write_field(
            &mut out,
            1,
            "sampling",
            &json_string(sampling_slug(self.sampling)),
            true,
        );
        write_field(
            &mut out,
            1,
            "dither",
            &json_string(dither_slug(self.dither)),
            true,
        );
        write_field(&mut out, 1, "source", &self.source_json(), true);
        write_field(&mut out, 1, "expected", &self.expected_json(palette), false);
        out.push_str("}\n");
        out
    }

    fn fit_json(&self) -> String {
        match &self.fit {
            FitMode::FixedBoards { board, cols, rows } => format!(
                "{{ \"mode\": \"fixed-boards\", \"board\": {}, \"cols\": {cols}, \"rows\": {rows} }}",
                board_json(board)
            ),
            FitMode::AspectFit {
                board,
                max_cols,
                max_rows,
            } => format!(
                "{{ \"mode\": \"aspect-fit\", \"board\": {}, \"maxCols\": {max_cols}, \"maxRows\": {max_rows} }}",
                board_json(board)
            ),
            FitMode::ScaleCrop { .. } => {
                // Left out on purpose: this mode carries floating-point
                // parameters, and a fixture whose input needs float parsing to
                // agree is a worse contract than one that does not.
                unreachable!("parity cases do not use scale-crop")
            }
        }
    }

    fn source_json(&self) -> String {
        let mut bytes = String::new();
        for (index, pixel) in self.source.as_slice().iter().enumerate() {
            if index > 0 {
                bytes.push_str(", ");
            }
            let _ = write!(bytes, "{}, {}, {}, {}", pixel.r, pixel.g, pixel.b, pixel.a);
        }
        format!(
            "{{\n    \"width\": {},\n    \"height\": {},\n    \"rgba\": [{bytes}]\n  }}",
            self.source.width(),
            self.source.height()
        )
    }

    fn expected_json(&self, palette: &Palette) -> String {
        let codes: Vec<String> = self
            .grid
            .as_slice()
            .iter()
            .map(|slot| match slot {
                Some(id) => json_string(&palette.color(*id).code),
                None => json_string(""),
            })
            .collect();

        let bom: Vec<String> = self
            .bom
            .lines
            .iter()
            .map(|line| {
                format!(
                    "\n      {{ \"code\": {}, \"name\": {}, \"count\": {} }}",
                    json_string(&line.code),
                    json_string(&line.name),
                    line.count
                )
            })
            .collect();

        let steps: Vec<String> = self
            .plans
            .iter()
            .map(|plan| {
                let groups: Vec<String> = plan
                    .groups
                    .iter()
                    .map(|group| {
                        let cells: Vec<String> = group
                            .cells
                            .iter()
                            .map(|c| format!("[{}, {}]", c.x, c.y))
                            .collect();
                        format!(
                            "\n        {{ \"label\": {}, \"cells\": [{}] }}",
                            json_string(&group.label),
                            cells.join(", ")
                        )
                    })
                    .collect();
                format!(
                    "\n      {{ \"mode\": {}, \"groups\": [{}\n      ] }}",
                    json_string(plan.mode),
                    groups.join(",")
                )
            })
            .collect();

        format!(
            "{{\n    \"width\": {},\n    \"height\": {},\n    \"codes\": [{}],\n    \"bom\": [{}\n    ],\n    \"steps\": [{}\n    ]\n  }}",
            self.grid.width(),
            self.grid.height(),
            codes.join(", "),
            bom.join(","),
            steps.join(",")
        )
    }
}

fn board_json(board: &crate::fit::BoardSpec) -> String {
    format!(
        "{{ \"name\": {}, \"width\": {}, \"height\": {} }}",
        json_string(&board.name),
        board.width,
        board.height
    )
}

fn sampling_slug(sampling: Sampling) -> &'static str {
    match sampling {
        Sampling::Nearest => "nearest",
        Sampling::BoxAverage => "box-average",
    }
}

fn dither_slug(dither: Dither) -> &'static str {
    match dither {
        Dither::None => "none",
        Dither::FloydSteinberg => "floyd-steinberg",
    }
}

fn write_field(out: &mut String, depth: usize, key: &str, value: &str, comma: bool) {
    let indent = "  ".repeat(depth);
    let _ = write!(out, "{indent}\"{key}\": {value}");
    out.push_str(if comma { ",\n" } else { "\n" });
}

fn json_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The parity cases, in the order their files are named.
///
/// Four, covering what T-PAR-1 asks for: the pixel-art path, the photograph
/// path with dithering off and on, and an image with a hole in it.
pub fn cases() -> Vec<ParityCase> {
    vec![
        pixel_art_case(),
        photo_case("photo-flat", Dither::None),
        photo_case("photo-dithered", Dither::FloydSteinberg),
        transparent_case(),
    ]
}

/// A 4×4 drawing exported at 4×, read back onto a 4×4 board.
fn pixel_art_case() -> ParityCase {
    let sprite = [
        [
            (0xFF, 0xD4, 0x00),
            (0xE4, 0x03, 0x2E),
            (0x0B, 0x61, 0xA4),
            (0x00, 0x00, 0x00),
        ],
        [
            (0xE4, 0x03, 0x2E),
            (0xFF, 0xFF, 0xFF),
            (0x2E, 0x9E, 0x45),
            (0x0B, 0x61, 0xA4),
        ],
        [
            (0x0B, 0x61, 0xA4),
            (0x2E, 0x9E, 0x45),
            (0xFF, 0xFF, 0xFF),
            (0xE4, 0x03, 0x2E),
        ],
        [
            (0x00, 0x00, 0x00),
            (0x0B, 0x61, 0xA4),
            (0xE4, 0x03, 0x2E),
            (0xFF, 0xD4, 0x00),
        ],
    ];
    let mut pixels = Vec::with_capacity(16 * 16);
    for y in 0..16u32 {
        for x in 0..16u32 {
            let (r, g, b) = sprite[(y / 4) as usize][(x / 4) as usize];
            pixels.push(Rgba::new(r, g, b, 255));
        }
    }
    ParityCase {
        name: "pixel-art".to_owned(),
        image: Image::from_pixels(16, 16, pixels).expect("16x16"),
        fit: FitMode::FixedBoards {
            board: crate::fit::BoardSpec::new("4x4", 4, 4),
            cols: 1,
            rows: 1,
        },
        sampling: Sampling::Nearest,
        dither: Dither::None,
    }
}

/// A 12×12 two-axis ramp read down onto a 6×6 board.
fn photo_case(name: &str, dither: Dither) -> ParityCase {
    let mut pixels = Vec::with_capacity(12 * 12);
    for y in 0..12u32 {
        for x in 0..12u32 {
            pixels.push(Rgba::new(
                (20 + x * 19) as u8,
                (40 + y * 17) as u8,
                (200 - x * 7 - y * 5) as u8,
                255,
            ));
        }
    }
    ParityCase {
        name: name.to_owned(),
        image: Image::from_pixels(12, 12, pixels).expect("12x12"),
        fit: FitMode::FixedBoards {
            board: crate::fit::BoardSpec::new("6x6", 6, 6),
            cols: 1,
            rows: 1,
        },
        sampling: Sampling::BoxAverage,
        dither,
    }
}

/// A ring with a hole in the middle, so the fixture pins what an empty cell
/// does to the bill and to all four step orders.
fn transparent_case() -> ParityCase {
    let mut pixels = Vec::with_capacity(12 * 12);
    for y in 0..12u32 {
        for x in 0..12u32 {
            let inside_hole = (4..8).contains(&x) && (4..8).contains(&y);
            pixels.push(if inside_hole {
                Rgba::TRANSPARENT
            } else {
                Rgba::new((30 + x * 18) as u8, 90, (240 - y * 15) as u8, 255)
            });
        }
    }
    ParityCase {
        name: "with-transparency".to_owned(),
        image: Image::from_pixels(12, 12, pixels).expect("12x12"),
        fit: FitMode::FixedBoards {
            board: crate::fit::BoardSpec::new("6x6", 6, 6),
            cols: 1,
            rows: 1,
        },
        sampling: Sampling::Nearest,
        dither: Dither::None,
    }
}

/// The detector's verdict on a case, for the fixtures that claim a path.
pub fn detected_kind(case: &ParityCase) -> ImageKind {
    crate::detect::analyze(&case.image).kind
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn strings_are_escaped() {
        assert_eq!(json_string("a\"b\\c"), "\"a\\\"b\\\\c\"");
        assert_eq!(json_string("tab\there"), "\"tab\\there\"");
        assert_eq!(json_string("×"), "\"×\"");
    }

    #[test]
    fn every_case_has_a_distinct_name() {
        let mut names: Vec<String> = cases().into_iter().map(|c| c.name).collect();
        names.sort();
        let count = names.len();
        names.dedup();
        assert_eq!(names.len(), count);
    }
}

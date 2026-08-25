//! The whole conversion in one call, for callers who want the default answer.
//!
//! Detect, frame, sample, map, count. Each step is public on its own; this only
//! wires them together and picks the defaults that follow from the detection —
//! pixel art is read with nearest sampling and no dithering, a photograph is
//! box-averaged and dithered.

use crate::bom::Bom;
use crate::detect::{analyze, ImageAnalysis, ImageKind};
use crate::fit::{plan, render, FitError, FitMode, FitPlan, Sampling};
use crate::image::Image;
use crate::palette::Palette;
use crate::quantize::{map_image, Dither, MapOptions, PatternGrid};

/// How to convert one image.
#[derive(Debug, Clone, PartialEq)]
pub struct PatternOptions {
    pub fit: FitMode,
    /// `None` asks for the detector's choice: no dithering for pixel art,
    /// Floyd–Steinberg for photographs.
    pub dither: Option<Dither>,
    /// `None` asks for the detector's choice: nearest for pixel art, box
    /// average for photographs.
    pub sampling: Option<Sampling>,
}

impl PatternOptions {
    pub fn new(fit: FitMode) -> Self {
        Self {
            fit,
            dither: None,
            sampling: None,
        }
    }

    pub fn with_dither(mut self, dither: Dither) -> Self {
        self.dither = Some(dither);
        self
    }

    pub fn with_sampling(mut self, sampling: Sampling) -> Self {
        self.sampling = Some(sampling);
        self
    }
}

/// A converted pattern and everything that was decided along the way.
#[derive(Debug, Clone, PartialEq)]
pub struct Pattern {
    pub analysis: ImageAnalysis,
    pub plan: FitPlan,
    pub sampling: Sampling,
    pub dither: Dither,
    pub grid: PatternGrid,
    pub bom: Bom,
}

/// Convert an image into a bead pattern.
pub fn to_pattern(
    image: &Image,
    palette: &Palette,
    options: &PatternOptions,
) -> Result<Pattern, FitError> {
    let analysis = analyze(image);
    let fit_plan = plan(image.width(), image.height(), &options.fit)?;

    let pixel_art = matches!(analysis.kind, ImageKind::PixelArt { .. });
    let sampling = options.sampling.unwrap_or(if pixel_art {
        Sampling::Nearest
    } else {
        Sampling::BoxAverage
    });
    let dither = options.dither.unwrap_or(if pixel_art {
        Dither::None
    } else {
        Dither::FloydSteinberg
    });

    let sampled = render(image, &fit_plan, sampling);
    let grid = map_image(&sampled, palette, MapOptions::with_dither(dither));
    let bom = Bom::from_grid(&grid, palette);

    Ok(Pattern {
        analysis,
        plan: fit_plan,
        sampling,
        dither,
        grid,
        bom,
    })
}

//! Bead palettes and nearest-colour lookup.
//!
//! A palette is an ordered list of bead colours; a [`ColorId`] is an index into
//! one. Ids are only meaningful next to the palette they came from, which is
//! why every consumer in this crate takes both.
//!
//! The shipped fixture is `generic-5mm`: a documented, vendor-neutral 48-colour
//! set for 5 mm fuse beads. It is deliberately *not* a brand's colour card. No
//! manufacturer publishes machine-readable Lab values, so a brand palette would
//! be a transcription of somebody else's swatches with our error bars on it.
//! `generic-5mm` is a fixture the tests can pin exactly, and real brand
//! palettes are meant to arrive later through [`Palette::new`].

use std::collections::BTreeSet;
use std::fmt;

use crate::color::{ciede2000, within_delta_e, Lab, Rgb, Rgba};

/// An index into a [`Palette`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ColorId(pub u16);

impl ColorId {
    pub fn index(self) -> usize {
        usize::from(self.0)
    }
}

impl fmt::Display for ColorId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// One bead colour: a code to order by, a name to show, and the sRGB swatch
/// its Lab value is derived from.
#[derive(Debug, Clone, PartialEq)]
pub struct BeadColor {
    pub code: String,
    pub name: String,
    pub rgb: Rgb,
    pub lab: Lab,
}

impl BeadColor {
    pub fn new(code: impl Into<String>, name: impl Into<String>, rgb: Rgb) -> Self {
        Self {
            code: code.into(),
            name: name.into(),
            rgb,
            lab: rgb.to_lab(),
        }
    }
}

/// A named, ordered set of bead colours.
#[derive(Debug, Clone, PartialEq)]
pub struct Palette {
    namespace: String,
    colors: Vec<BeadColor>,
}

/// A palette that could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteError {
    /// Nothing to map onto.
    Empty,
    /// Two entries share a code, so a bill of materials could not name one.
    DuplicateCode(String),
    /// More colours than a [`ColorId`] can address.
    TooManyColors(usize),
}

impl fmt::Display for PaletteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaletteError::Empty => f.write_str("a palette needs at least one colour"),
            PaletteError::DuplicateCode(code) => write!(f, "colour code `{code}` appears twice"),
            PaletteError::TooManyColors(count) => {
                write!(f, "{count} colours exceeds the {} id limit", u16::MAX)
            }
        }
    }
}

impl std::error::Error for PaletteError {}

/// The nearest palette entry to some colour, and how far away it was.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteMatch {
    pub id: ColorId,
    pub delta_e: f64,
}

impl Palette {
    pub fn new(namespace: impl Into<String>, colors: Vec<BeadColor>) -> Result<Self, PaletteError> {
        if colors.is_empty() {
            return Err(PaletteError::Empty);
        }
        if colors.len() > usize::from(u16::MAX) {
            return Err(PaletteError::TooManyColors(colors.len()));
        }
        let mut seen = BTreeSet::new();
        for color in &colors {
            if !seen.insert(color.code.clone()) {
                return Err(PaletteError::DuplicateCode(color.code.clone()));
            }
        }
        Ok(Self {
            namespace: namespace.into(),
            colors,
        })
    }

    /// The documented vendor-neutral 5 mm fixture. See the module docs for why
    /// this is not a brand colour card.
    pub fn generic_5mm() -> Self {
        let colors = GENERIC_5MM
            .iter()
            .map(|(code, name, hex)| {
                let rgb = Rgb::from_hex(hex).expect("the built-in fixture is valid hex");
                BeadColor::new(*code, *name, rgb)
            })
            .collect();
        Self::new("generic-5mm", colors).expect("the built-in fixture is a valid palette")
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub fn len(&self) -> usize {
        self.colors.len()
    }

    /// Always false: [`Palette::new`] rejects an empty colour list.
    pub fn is_empty(&self) -> bool {
        self.colors.is_empty()
    }

    pub fn get(&self, id: ColorId) -> Option<&BeadColor> {
        self.colors.get(id.index())
    }

    /// Panics if `id` is not from this palette. Use [`Palette::get`] when the
    /// id might have come from somewhere else.
    pub fn color(&self, id: ColorId) -> &BeadColor {
        self.get(id)
            .unwrap_or_else(|| panic!("{id} is not an id in palette `{}`", self.namespace))
    }

    pub fn iter(&self) -> impl Iterator<Item = (ColorId, &BeadColor)> + '_ {
        self.colors
            .iter()
            .enumerate()
            .map(|(i, c)| (ColorId(i as u16), c))
    }

    pub fn find_code(&self, code: &str) -> Option<ColorId> {
        self.iter()
            .find(|(_, color)| color.code == code)
            .map(|(id, _)| id)
    }

    /// The entry closest to `lab` by CIEDE2000. Ties go to the lower id, so the
    /// result only depends on palette order and never on iteration luck.
    pub fn nearest_lab(&self, lab: Lab) -> PaletteMatch {
        let mut best = PaletteMatch {
            id: ColorId(0),
            delta_e: f64::INFINITY,
        };
        for (id, color) in self.iter() {
            let delta_e = ciede2000(lab, color.lab);
            if delta_e < best.delta_e {
                best = PaletteMatch { id, delta_e };
            }
        }
        best
    }

    /// The entry closest to an sRGB pixel, via Lab.
    pub fn nearest(&self, rgb: Rgb) -> PaletteMatch {
        self.nearest_lab(rgb.to_lab())
    }

    /// The nearest entry and the runner-up.
    ///
    /// The gap between them is what T-PAR-3 guards: `powf`, `cbrt` and `atan2`
    /// can differ in the last place between Rust's libm and JavaScript's
    /// `Math`, so a pixel whose best and second-best beads are a hair apart can
    /// flip between the two implementations. A parity fixture containing one is
    /// not a fixture, it is a coin toss, and this is how the generator finds
    /// them.
    pub fn nearest_two(&self, lab: Lab) -> (PaletteMatch, Option<PaletteMatch>) {
        let mut best: Option<PaletteMatch> = None;
        let mut second: Option<PaletteMatch> = None;
        for (id, color) in self.iter() {
            let candidate = PaletteMatch {
                id,
                delta_e: ciede2000(lab, color.lab),
            };
            match best {
                Some(current) if candidate.delta_e < current.delta_e => {
                    second = Some(current);
                    best = Some(candidate);
                }
                Some(_) => {
                    if second.is_none_or(|s| candidate.delta_e < s.delta_e) {
                        second = Some(candidate);
                    }
                }
                None => best = Some(candidate),
            }
        }
        (
            best.unwrap_or(PaletteMatch {
                id: ColorId(0),
                delta_e: f64::INFINITY,
            }),
            second,
        )
    }

    /// How much closer the nearest entry is than the runner-up. Infinite for a
    /// one-colour palette, where there is nothing to flip to.
    pub fn decision_margin(&self, lab: Lab) -> f64 {
        match self.nearest_two(lab) {
            (best, Some(second)) => second.delta_e - best.delta_e,
            (_, None) => f64::INFINITY,
        }
    }

    /// Every entry strictly within `max_delta_e` of `lab`, nearest first. Ties
    /// break on the colour index, so the order is total (G6).
    pub fn within(&self, lab: Lab, max_delta_e: f64) -> Vec<PaletteMatch> {
        let mut found: Vec<PaletteMatch> = self
            .iter()
            .filter(|(_, color)| within_delta_e(lab, color.lab, max_delta_e))
            .map(|(id, color)| PaletteMatch {
                id,
                delta_e: ciede2000(lab, color.lab),
            })
            .collect();
        found.sort_by(|a, b| {
            a.delta_e
                .partial_cmp(&b.delta_e)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.id.cmp(&b.id))
        });
        found
    }

    /// The nearest entry to a pixel, or `None` when the pixel is not a bead.
    pub fn nearest_rgba(&self, rgba: Rgba) -> Option<PaletteMatch> {
        rgba.rgb().map(|rgb| self.nearest(rgb))
    }
}

/// `generic-5mm`: 48 vendor-neutral 5 mm fuse-bead colours.
///
/// Chosen to cover the hue circle at three lightness steps plus the neutrals,
/// skin tones and browns that pixel portraits actually need. The hex values are
/// the fixture — they define the palette rather than approximating anything, so
/// changing one is a breaking change to every pinned nearest-colour test.
const GENERIC_5MM: &[(&str, &str, &str)] = &[
    ("G01", "White", "#FFFFFF"),
    ("G02", "Cream", "#F5EFE0"),
    ("G03", "Light Grey", "#D3D3D3"),
    ("G04", "Grey", "#9E9E9E"),
    ("G05", "Dark Grey", "#5C5C5C"),
    ("G06", "Black", "#000000"),
    ("G07", "Silver", "#B7BFC6"),
    ("G08", "Slate", "#6B7A85"),
    ("G09", "Charcoal", "#2B2F33"),
    ("G10", "Pale Pink", "#FFD9E2"),
    ("G11", "Pink", "#FF9EC4"),
    ("G12", "Rose", "#F0559A"),
    ("G13", "Magenta", "#E0218A"),
    ("G14", "Light Red", "#FF6F61"),
    ("G15", "Red", "#E4032E"),
    ("G16", "Dark Red", "#9B1B24"),
    ("G17", "Salmon", "#FFA48A"),
    ("G18", "Orange", "#F5821F"),
    ("G19", "Dark Orange", "#D2601A"),
    ("G20", "Peach", "#FFCBA4"),
    ("G21", "Light Yellow", "#FFF3A1"),
    ("G22", "Yellow", "#FFD400"),
    ("G23", "Gold", "#E0A526"),
    ("G24", "Lime", "#C6DE41"),
    ("G25", "Light Green", "#8CC63F"),
    ("G26", "Green", "#2E9E45"),
    ("G27", "Dark Green", "#14602D"),
    ("G28", "Mint", "#A8E6CF"),
    ("G29", "Teal", "#009B9F"),
    ("G30", "Dark Teal", "#00666B"),
    ("G31", "Sky Blue", "#8FD3F4"),
    ("G32", "Light Blue", "#4FA3E3"),
    ("G33", "Blue", "#0B61A4"),
    ("G34", "Dark Blue", "#123A6B"),
    ("G35", "Navy", "#0B1E3C"),
    ("G36", "Periwinkle", "#9FA8DA"),
    ("G37", "Violet", "#7A5CC4"),
    ("G38", "Purple", "#59259E"),
    ("G39", "Dark Purple", "#3B1660"),
    ("G40", "Lavender", "#D6C7EA"),
    ("G41", "Beige", "#E3C79A"),
    ("G42", "Tan", "#C9A06A"),
    ("G43", "Light Brown", "#A9713F"),
    ("G44", "Brown", "#7B4B25"),
    ("G45", "Dark Brown", "#4A2B14"),
    ("G46", "Skin Light", "#FFE0C4"),
    ("G47", "Skin Medium", "#E8B48A"),
    ("G48", "Skin Deep", "#A96A46"),
];

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn the_fixture_is_well_formed() {
        let palette = Palette::generic_5mm();
        assert_eq!(palette.namespace(), "generic-5mm");
        assert_eq!(palette.len(), 48);
        let codes: BTreeSet<&str> = palette.iter().map(|(_, c)| c.code.as_str()).collect();
        assert_eq!(codes.len(), 48);
    }

    #[test]
    fn duplicate_codes_are_rejected() {
        let colors = vec![
            BeadColor::new("A", "one", Rgb::new(0, 0, 0)),
            BeadColor::new("A", "two", Rgb::new(255, 255, 255)),
        ];
        assert_eq!(
            Palette::new("dupe", colors).unwrap_err(),
            PaletteError::DuplicateCode("A".to_owned())
        );
    }

    #[test]
    fn an_exact_swatch_matches_itself_at_zero() {
        let palette = Palette::generic_5mm();
        for (id, color) in palette.iter() {
            let found = palette.nearest(color.rgb);
            assert_eq!(found.id, id, "{} did not find itself", color.code);
            assert!(found.delta_e < 1e-9);
        }
    }
}

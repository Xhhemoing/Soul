//! What to buy, what is missing, and what could stand in for it.
//!
//! A [`Bom`] is a count per colour. Checking it against an [`Inventory`] gives
//! a list of [`Shortage`]s, each carrying the in-stock colours that are close
//! enough to substitute.
//!
//! "Close enough" is CIEDE2000 below [`SUBSTITUTE_MAX_DELTA_E`] — a difference
//! of 3 is roughly where a side-by-side comparison stops being obvious to most
//! people. It is a threshold, not a promise: the candidates are ranked by
//! distance and the number is shown, so the decision stays with whoever is
//! looking at the beads.

use std::collections::BTreeMap;

use crate::color::{ciede2000, within_delta_e};
use crate::palette::{ColorId, Palette};
use crate::quantize::PatternGrid;

/// Substitutes must be nearer than this in CIEDE2000.
pub const SUBSTITUTE_MAX_DELTA_E: f64 = 3.0;

/// One colour's worth of a bill of materials.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BomLine {
    pub id: ColorId,
    pub code: String,
    pub name: String,
    pub count: usize,
}

/// Every colour a pattern needs, commonest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bom {
    pub palette: String,
    pub lines: Vec<BomLine>,
}

impl Bom {
    /// Count the colours in a pattern.
    ///
    /// Empty cells are not beads and are not counted, so `total_beads` equals
    /// the number of filled cells — the same invariant the step planner's
    /// partition rests on. Ids that are not in `palette` are reported under
    /// their numeric id so a mismatch is visible rather than silently dropped.
    /// Commonest first, ties on the colour index (G6).
    pub fn from_grid(grid: &PatternGrid, palette: &Palette) -> Self {
        let mut counts: BTreeMap<ColorId, usize> = BTreeMap::new();
        for (_, slot) in grid.iter() {
            if let Some(id) = slot {
                *counts.entry(*id).or_insert(0) += 1;
            }
        }

        let mut lines: Vec<BomLine> = counts
            .into_iter()
            .map(|(id, count)| match palette.get(id) {
                Some(color) => BomLine {
                    id,
                    code: color.code.clone(),
                    name: color.name.clone(),
                    count,
                },
                None => BomLine {
                    id,
                    code: id.to_string(),
                    name: "unknown".to_owned(),
                    count,
                },
            })
            .collect();
        lines.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.id.cmp(&b.id)));

        Self {
            palette: palette.namespace().to_owned(),
            lines,
        }
    }

    pub fn total_beads(&self) -> usize {
        self.lines.iter().map(|l| l.count).sum()
    }

    pub fn distinct_colors(&self) -> usize {
        self.lines.len()
    }

    pub fn count_of(&self, code: &str) -> usize {
        self.lines
            .iter()
            .find(|l| l.code == code)
            .map_or(0, |l| l.count)
    }
}

/// How many beads of each colour are on hand, keyed by palette code.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Inventory {
    stock: BTreeMap<String, usize>,
}

impl Inventory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_pairs<I, S>(pairs: I) -> Self
    where
        I: IntoIterator<Item = (S, usize)>,
        S: Into<String>,
    {
        let mut inventory = Self::new();
        for (code, count) in pairs {
            inventory.set(code, count);
        }
        inventory
    }

    pub fn set(&mut self, code: impl Into<String>, count: usize) {
        self.stock.insert(code.into(), count);
    }

    pub fn add(&mut self, code: impl Into<String>, count: usize) {
        *self.stock.entry(code.into()).or_insert(0) += count;
    }

    pub fn on_hand(&self, code: &str) -> usize {
        self.stock.get(code).copied().unwrap_or(0)
    }

    pub fn codes(&self) -> impl Iterator<Item = &str> {
        self.stock.keys().map(String::as_str)
    }

    pub fn is_empty(&self) -> bool {
        self.stock.is_empty()
    }
}

/// An in-stock colour offered in place of one that is missing.
#[derive(Debug, Clone, PartialEq)]
pub struct Substitute {
    pub id: ColorId,
    pub code: String,
    pub name: String,
    pub delta_e: f64,
    /// Beads of this colour not already claimed by the rest of the pattern.
    pub available: usize,
}

/// One colour the pattern needs and the inventory cannot fully supply.
#[derive(Debug, Clone, PartialEq)]
pub struct Shortage {
    pub id: ColorId,
    pub code: String,
    pub name: String,
    pub needed: usize,
    pub on_hand: usize,
    pub missing: usize,
    /// Nearest first, then by code. Empty when nothing in stock is close.
    pub substitutes: Vec<Substitute>,
}

/// The result of holding a bill of materials against an inventory.
#[derive(Debug, Clone, PartialEq)]
pub struct StockCheck {
    pub shortages: Vec<Shortage>,
    /// Total beads the inventory cannot cover.
    pub missing_beads: usize,
}

impl StockCheck {
    pub fn is_satisfied(&self) -> bool {
        self.shortages.is_empty()
    }

    pub fn shortage_of(&self, code: &str) -> Option<&Shortage> {
        self.shortages.iter().find(|s| s.code == code)
    }
}

/// Compare a bill of materials with what is on hand.
///
/// A colour is only offered as a substitute if there are beads of it left over
/// once the pattern's own use of that colour is set aside — suggesting a colour
/// the same pattern is about to consume would just move the shortage.
pub fn check_stock(bom: &Bom, palette: &Palette, inventory: &Inventory) -> StockCheck {
    check_stock_within(bom, palette, inventory, SUBSTITUTE_MAX_DELTA_E)
}

/// [`check_stock`] with the substitute threshold spelled out.
pub fn check_stock_within(
    bom: &Bom,
    palette: &Palette,
    inventory: &Inventory,
    max_delta_e: f64,
) -> StockCheck {
    let spare: BTreeMap<&str, usize> = inventory
        .codes()
        .map(|code| {
            let claimed = bom.count_of(code).min(inventory.on_hand(code));
            (code, inventory.on_hand(code) - claimed)
        })
        .collect();

    let mut shortages = Vec::new();
    let mut missing_beads = 0usize;

    for line in &bom.lines {
        let on_hand = inventory.on_hand(&line.code);
        if on_hand >= line.count {
            continue;
        }
        let missing = line.count - on_hand;
        missing_beads += missing;

        let wanted = palette.get(line.id).map(|c| c.lab);
        let mut substitutes: Vec<Substitute> = Vec::new();
        if let Some(wanted) = wanted {
            for (code, available) in &spare {
                if *available == 0 || *code == line.code {
                    continue;
                }
                let Some(id) = palette.find_code(code) else {
                    continue;
                };
                let color = palette.color(id);
                if !within_delta_e(wanted, color.lab, max_delta_e) {
                    continue;
                }
                let delta_e = ciede2000(wanted, color.lab);
                substitutes.push(Substitute {
                    id,
                    code: color.code.clone(),
                    name: color.name.clone(),
                    delta_e,
                    available: *available,
                });
            }
        }
        substitutes.sort_by(|a, b| {
            a.delta_e
                .partial_cmp(&b.delta_e)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });

        shortages.push(Shortage {
            id: line.id,
            code: line.code.clone(),
            name: line.name.clone(),
            needed: line.count,
            on_hand,
            missing,
            substitutes,
        });
    }

    StockCheck {
        shortages,
        missing_beads,
    }
}

#[cfg(test)]
mod unit {
    use super::*;

    #[test]
    fn inventory_reports_zero_for_an_unknown_code() {
        let inventory = Inventory::from_pairs([("G01", 5usize)]);
        assert_eq!(inventory.on_hand("G01"), 5);
        assert_eq!(inventory.on_hand("G99"), 0);
    }

    #[test]
    fn adding_accumulates() {
        let mut inventory = Inventory::new();
        inventory.add("G02", 3);
        inventory.add("G02", 4);
        assert_eq!(inventory.on_hand("G02"), 7);
    }
}

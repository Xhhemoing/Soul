//! BeadFlow's algorithm oracle.
//!
//! Everything between "here is a picture" and "here is the order to place the
//! beads in" belongs in this crate, as plain arithmetic over std collections.
//! The colour half is here first:
//!
//! * [`color`] — sRGB to Lab under D65, and CIEDE2000;
//! * [`palette`] — bead colours and nearest-colour lookup, with the
//!   `generic-5mm` fixture;
//! * [`grid`] — the rectangular cell array everything else is phrased in;
//! * [`image`] — the decoded raster this crate takes as input;
//! * [`quantize`] — mapping pixels to beads, with optional Floyd–Steinberg.
//!
//! ## Scope
//!
//! This package is deliberately outside the repository's cargo workspace, has
//! no dependencies, opens no socket and touches no file. It is the reference
//! the browser implementation in WP-B03 is checked against, so its public
//! results have to be reproducible: every ordering in here is total, and ties
//! break on palette code rather than on hash iteration order.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod color;
pub mod grid;
pub mod image;
pub mod palette;
pub mod quantize;

pub use color::{ciede2000, srgb_to_lab, Lab, Rgb};
pub use grid::{Cell, Grid};
pub use palette::{ColorId, Palette};

//! Write the shared parity fixtures to `fixtures/parity/`.
//!
//! The committed JSON is what WP-B03's `vitest` suite reads, and
//! `tests/parity.rs` asserts that this crate still produces it byte for byte.
//! Run this after any deliberate change to the algorithms:
//!
//! ```text
//! cargo run --example emit-parity-fixtures --manifest-path crates/bead-core/Cargo.toml
//! ```
//!
//! A regenerated file with an unexplained diff is a broken contract, not a
//! refreshed fixture — the browser port is pinned to these bytes.

use std::fs;
use std::path::Path;

use bead_core::palette::Palette;
use bead_core::parity::{build, cases};

fn main() {
    let palette = Palette::generic_5mm();
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/parity");
    fs::create_dir_all(&directory).expect("the fixture directory");

    for case in cases() {
        let fixture = build(&case, &palette).expect("every parity case has a valid framing");
        assert!(
            fixture.near_ties.is_empty(),
            "`{}` has {} pixel(s) too close to call: {:?}",
            fixture.name,
            fixture.near_ties.len(),
            fixture.near_ties
        );
        let path = directory.join(format!("{}.json", fixture.name));
        fs::write(&path, fixture.to_json(&palette)).expect("writing the fixture");
        println!("wrote {}", path.display());
    }
}

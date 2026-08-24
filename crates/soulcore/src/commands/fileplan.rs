//! WP11 command surface: read-only scan and plan preview.
//!
//! Empty, and honestly so. ST-00 declared the module and registered
//! `soul-fileplan` as a dependency; ST-02 fills this file with the
//! orchestration WP11 needs — the `ScanDirectory` and `PlanFiles` action
//! checks, and the `FilePlan` audit entry on both the allowed and the refused
//! path. Until then there is nothing here to call, which is the accurate state
//! of the work package.

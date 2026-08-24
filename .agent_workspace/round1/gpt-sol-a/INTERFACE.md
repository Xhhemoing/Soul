# Benchkit interface

The package is an independent Rust 2021 crate named `soul-t0-benchkit`; its
library import name is `soul_t0_benchkit`. It has no dependencies.

## Core shapes

```rust
pub const MODERATE_MIN_INTERACTIONS: u64 = 3;
pub const STRONG_MIN_INTERACTIONS: u64 = 10;
pub const STRONG_MIN_ACTIVE_DAYS: u64 = 3;
pub const FIXTURE_NOW_UTC: &str = "2026-08-24T14:00:00Z";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Direction {
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Venue {
    Direct,
    Group,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Band {
    Weak,
    Moderate,
    Strong,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interaction {
    pub peer_id: String,
    pub direction: Direction,
    pub venue: Venue,
    pub conversation_ref: String,
    pub occurred_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct T0Score {
    pub band: Band,
    pub interaction_count: u64,
    pub outgoing_count: u64,
    pub incoming_count: u64,
    pub conversation_count: u64,
    pub active_day_count: u64,
    pub first_contact_utc: Option<String>,
    pub last_contact_utc: Option<String>,
    pub has_direct: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    pub id: &'static str,
    pub now_utc: &'static str,
    pub primary_peer: &'static str,
    pub interactions: Vec<Interaction>,
    pub expected_t0_band: Band,
    pub notes: &'static str,
}
```

`has_direct` is the independent equivalent of Goal 1's `any_direct`. Group
versus direct is retained in the interaction and summary shapes, but T0 does
not use venue when selecting a band.

## Exact public function signatures

```rust
impl Band {
    pub const fn as_str(self) -> &'static str;
}

impl Interaction {
    pub fn new(
        peer_id: impl Into<String>,
        direction: Direction,
        venue: Venue,
        conversation_ref: impl Into<String>,
        occurred_at_utc: impl Into<String>,
    ) -> Self;
}

pub fn score_t0(interactions: &[Interaction], peer_id: &str) -> T0Score;

pub fn score_all_t0(
    interactions: &[Interaction],
) -> std::collections::BTreeMap<String, T0Score>;

pub fn fixture_lilei() -> Fixture;
pub fn fixture_burst() -> Fixture;
pub fn fixture_old() -> Fixture;
pub fn fixture_group() -> Fixture;
pub fn fixture_oneside() -> Fixture;
pub fn fixture_empty() -> Fixture;
pub fn fixture_scale() -> Fixture;
pub fn all_fixtures() -> Vec<Fixture>;

pub fn fixture_metadata_jsonl(fixtures: &[Fixture]) -> String;
```

`score_t0` returns an all-zero `Weak` score if `peer_id` has no observations.
`score_all_t0` emits only observed peers. UTC active days are the first 10
characters of the normalized RFC 3339 `occurred_at_utc`, matching Goal 1.

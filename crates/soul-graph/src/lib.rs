//! Soul's people graph.
//!
//! PRODUCT_LOCK: *nodes are people; edges carry interaction strength,
//! relationship type, last contact and evidence; third-party data does not
//! leave the machine.* That sentence is the whole specification, and this
//! crate is five small pieces of it:
//!
//! * [`interaction`] — the contract an importer writes: one observed exchange,
//!   recorded as evidence that points at a sealed event rather than at prose;
//! * [`build`] — one pass over that evidence produces one edge per person and
//!   one inference per edge, both carrying the ids that support them;
//! * [`model`] — what the UI reads. There is no field on a node that holds a
//!   name; labels are sealed pointers and identifiers are hashes;
//! * [`view`] — reading it back, and resolving an edge's evidence so that
//!   "there is evidence" can be checked rather than claimed;
//! * [`correct`] — the user overruling a band, and the way back out of it,
//!   because PRODUCT_LOCK says this graph is one the user can correct.
//!
//! v0.1 builds an ego network. Soul only ever saw conversations the user was
//! in, so an edge between two other people would be an inference with nothing
//! behind it, and inferences here have to have something behind them.
//!
//! Nothing in this crate opens a socket or writes a file. Every third-party
//! node and every edge is `local_only`, which is the graph's half of the
//! promise the redactor and the research preview keep on their side.

#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]

pub mod build;
pub mod correct;
pub mod error;
pub mod interaction;
pub mod model;
pub mod t4d_adapt;
pub mod view;

pub use build::{rebuild, GraphBuild};
pub use correct::{correct_tie, corrected_relationship, release_tie, TieCorrection};
pub use error::{GraphError, GraphResult};
pub use interaction::{conversation_ref, Direction, InteractionRef, Venue};
pub use model::{PersonNode, SoulGraph, TieEdge, TieStrength, TieType};
pub use t4d_adapt::{AdaptError, InteractionInterner};
pub use view::{load, resolve_evidence};

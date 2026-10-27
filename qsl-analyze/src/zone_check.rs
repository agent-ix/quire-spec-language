// SPDX-License-Identifier: AGPL-3.0-or-later
//! The native zone engine EN-6 (ADR-026 §8).
//!
//! [`Dbm`] is its zone representation (EZ-2, FR-238): a difference-bound
//! matrix over arbitrary-precision integer bounds, after every constant of
//! the model and the claim has been scaled once to an integer by
//! [`scale_constants`]. [`check_timed_model`] is its search (FR-239), with
//! finite counterexamples concretised to exact delays (FR-241).

mod certificate;
mod concretise;
mod dbm;
mod divergence;
mod scale;
mod search;
mod settle;
mod window;

pub use certificate::{
    CertEdge, CertNode, DivergenceWitness, DivergentTarget, LuBounds, ZoneCertificate,
};
pub use dbm::{Bound, Dbm, DbmError};
pub use scale::{scale_constants, ScaledConstants};
pub use search::{
    check_timed_model, ZoneCheckItem, ZoneCheckLimit, ZoneCheckLimits, ZoneCheckOutcome,
    ZoneCheckRefusal, ZoneCheckRequest, ZoneStats, ZoneStop, ZoneUndecided,
};
pub use settle::{
    settle_zone_check, ZoneCertificateCheck, ZoneCheckRecord, ZoneItem, ZONE_CERTIFIED,
};
pub use window::bounded_window_verdict;

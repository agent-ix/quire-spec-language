// SPDX-License-Identifier: AGPL-3.0-or-later
//! The native zone engine EN-6 (ADR-026 §8).
//!
//! [`Dbm`] is its zone representation (EZ-2, FR-238): a difference-bound
//! matrix over arbitrary-precision integer bounds, after every constant of
//! the model and the claim has been scaled once to an integer by
//! [`scale_constants`]. [`check_timed_model`] is its search (FR-239), with
//! FR-240's liveness phase on a symbolic product, and counterexamples,
//! finite or lassos, concretised to exact delays (FR-241). A proof
//! carries `qsl_replay`'s [`qsl_replay::ZoneCertificate`] (FR-244), which
//! the core checks with its own code (FR-245).

mod certificate;
mod concretise;
mod dbm;
mod divergence;
mod lasso;
mod liveness;
mod scale;
mod search;
mod settle;
mod time_lock;

pub use certificate::certificate_item;
pub use dbm::{Bound, Dbm, DbmError};
pub use scale::{scale_constants, ScaledConstants};
pub use search::{
    check_timed_model, ZoneCheckItem, ZoneCheckLimit, ZoneCheckLimits, ZoneCheckOutcome,
    ZoneCheckRefusal, ZoneCheckRequest, ZoneStats, ZoneStop, ZoneUndecided,
};
pub use settle::{settle_zone_check, ZoneCheckRecord, ZoneItem, ZONE_CERTIFIED};

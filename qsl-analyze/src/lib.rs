// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-analyze`: the ADR-011 §6.1 layer **A** crate (ADR-011 §7.3 X-12,
//! ADR-029 CB-3) -- the in-process analysis engines, above the qualified
//! core. Nothing in layers K to 6, R or tool depends on it.
//!
//! [`zone_check`] holds the native zone engine EN-6's difference-bound
//! matrices (ADR-026 EZ-2, FR-238). The zone certificate checker in
//! `qsl-replay` keeps its own DBM code and never calls this one (ADR-026
//! CF-6).
//!
//! [`statistical`] holds EN-4, statistical model checking of probabilistic
//! claims (ADR-024 ST-1, FR-189 to FR-191).
//!
//! [`exact_probabilistic`] holds EN-5's probabilistic product (FR-196).
//!
//! [`schedulability`] holds EN-7, the closed-form schedulability engine
//! (ADR-026 §12, FR-247).

#![forbid(unsafe_code)]

pub mod exact_probabilistic;
pub mod model_check;
pub mod schedulability;
pub mod statistical;
pub mod zone_check;

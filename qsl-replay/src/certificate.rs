// SPDX-License-Identifier: AGPL-3.0-or-later
//! The in-core certificate checkers' own arithmetic (ADR-029 CB-2).
//!
//! [`zone`] holds the zone certificate checker's difference-bound matrices
//! (ADR-026 CF-3, CF-6; FR-245): separate code from the zone engine's DBM in
//! `qsl-analyze`, so a fault in one cannot hide a fault in the other.

pub mod zone;

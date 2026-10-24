// SPDX-License-Identifier: AGPL-3.0-or-later
//! The closed-form schedulability checker's own code (ADR-026 RT-7,
//! FR-248; ADR-029 CB-2): the task set and claim an analysis is about
//! ([`task_set`]), the outcome and evidence EN-7 returns ([`outcome`]), the
//! recomputation of that evidence in exact rationals ([`recompute`]) and the
//! verdict it settles as ([`settle`]). It names nothing of `qsl-analyze`,
//! which depends on these types and not the reverse.

pub mod outcome;
pub mod recompute;
pub mod settle;
pub mod task_set;

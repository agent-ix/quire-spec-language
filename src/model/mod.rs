// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-150-153: the `quire.model.complete/v1` model graph — original/effective
//! declaration provenance, inheritance and conformance, systems-model binding
//! and closed model environments.
//!
//! QSL does not yet receive a `quire.model.complete/v1` bundle from a live
//! Semantic IR 2.0.0 intake (`agent-ix/quire-specification#131`'s intake half,
//! blocked on `agent-ix/filament-core-data#173`, unmerged as of this rung).
//! [`bundle::Bundle`] is the producer interface `1.3.0` bundle shape that
//! intake will eventually supply; everything else in this module is a pure
//! function of that value, so wiring a real intake later replaces only how a
//! [`bundle::Bundle`] is constructed, never how it is normalized, queried or
//! dispatched against.

pub mod accounting;
pub mod bundle;
pub mod conformance;
pub mod dispatch;
pub mod key;
pub mod normalize;
pub mod population;
pub mod refusal;
pub mod systems;

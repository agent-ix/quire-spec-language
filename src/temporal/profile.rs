// SPDX-License-Identifier: AGPL-3.0-or-later
//! The registered identities of the three v1 temporal profiles, which the
//! strict `/2` reader compares against exactly.

/// Exact registered identity of the event-position profile.
pub const EVENT_POSITION: &str = "quire.temporal.event-position.false-extension/v1";
/// Exact registered identity of the fixed-sample profile.
pub const FIXED_SAMPLE: &str = "quire.temporal.fixed-sample.false-extension/v1";
/// Exact registered identity of the timestamped finite-window profile.
pub const TIMESTAMPED_WINDOW: &str = "quire.temporal.timestamped-event.finite-window/v1";

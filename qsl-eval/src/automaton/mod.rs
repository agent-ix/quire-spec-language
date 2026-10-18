// SPDX-License-Identifier: AGPL-3.0-or-later
//! The property-automaton kernels (ADR-018 SM-6, §12; ADR-011 §6.1 layer 5):
//! what `qsl-analyze`'s model checker and the layer-6 certificate checkers
//! both build on, so neither depends on the other.
//!
//! This module owns the automaton data and the algorithms over it, never a
//! formula: translating a checked temporal clause into these automata is the
//! property-automaton translation that builds on this module.
//!
//! - [`Gba`] is a generalized Büchi automaton over [`Letter`]s: state 0 is
//!   the automaton's initial state, a transition carries a [`Guard`], and
//!   each acceptance set is a set of states. With no acceptance set it is a
//!   safety automaton.
//! - [`Monitor`] is the deterministic bad-prefix monitor of a safety
//!   automaton (ADR-018 SM-6): the automaton trimmed to the states from which
//!   some infinite run exists, then determinized by subset construction as
//!   the caller reaches states, whose one rejecting state is the empty
//!   subset. A deterministic automaton gives the bounded profile's finite
//!   monitor through the same type.
//! - [`Numbering`] numbers states in the order the caller reaches them, under
//!   the `max_automaton_states` bound (ADR-018 IV-6), with checked
//!   arithmetic. The number is storage local to one explored graph and never
//!   a state's identity (ADR-018 §12).
//! - The counters ([`DeadlineCounter`], [`ExtentCounter`], [`PastAgeCounter`],
//!   [`ShiftCounter`], [`EventuallyWindows`], [`AlwaysOffsets`]) are ADR-018
//!   IV-3's counter construction for interval operators: one counter per
//!   occurrence for a lower bound of 0, and sets of open windows or offsets
//!   otherwise.

mod counters;
mod error;
mod gba;
mod letter;
mod monitor;
mod numbering;
mod state;

pub use counters::{
    AlwaysOffsets, DeadlineCounter, EventuallyWindows, Expired, ExtentCounter, PastAgeCounter,
    ShiftCounter, Violated,
};
pub use error::AutomatonError;
pub use gba::{Gba, GbaBuilder, Transition};
pub use letter::{Atom, Guard, Letter, MAX_ATOMS};
pub use monitor::Monitor;
pub use numbering::{Interned, Numbering};
pub use state::StateId;

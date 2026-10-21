// SPDX-License-Identifier: AGPL-3.0-or-later
//! The property-automaton kernels (ADR-018 SM-6, §12; ADR-011 §6.1 layer 5):
//! what `qsl-analyze`'s model checker and the layer-6 certificate checkers
//! both build on, so neither depends on the other.
//!
//! This module owns the automaton data, the algorithms over it and the
//! translation of a checked temporal clause into these automata.
//!
//! - [`Gba`] is a generalized Büchi automaton over [`Letter`]s: state 0 is
//!   the automaton's initial state, a transition carries a [`Guard`], and
//!   each acceptance set is a set of states. With no acceptance set it is a
//!   safety automaton.
//! - [`Monitor`] is the deterministic bad-prefix monitor of a safety
//!   automaton (ADR-018 SM-6), read lazily through [`SafetyAutomaton`]: the
//!   subset construction over the states from which some infinite run
//!   exists, each state's viability decided when a step first meets it, whose
//!   one rejecting state is the empty subset. A deterministic automaton gives
//!   the bounded profile's finite monitor through the same type.
//! - [`Numbering`] numbers states in the order the caller reaches them, under
//!   the `max_automaton_states` bound (ADR-018 IV-6), with checked
//!   arithmetic. The number is storage local to one explored graph and never
//!   a state's identity (ADR-018 §12).
//! - [`translate`] builds a [`PropertyAutomaton`] from a checked temporal
//!   clause (ADR-018 SM-6): a deterministic finite monitor for a bounded
//!   clause, the automaton of an infinite-trace safety formula for the
//!   bad-prefix monitor, and a [`Gba`] of the negation of a liveness formula.
//!   Past operators are tracked as automaton state, and an interval operator
//!   keeps the counters below (IV-3).
//!   [`translate_counted`] reads the steps of a protocol subject by kind
//!   (ADR-027 PB-3): the letter gains an atom for a step that does not count,
//!   and the counters and shifts advance only on counted steps.
//! - The counters ([`DeadlineCounter`], [`ExtentCounter`], [`PastAgeCounter`],
//!   [`ShiftCounter`], [`EventuallyWindows`], [`AlwaysOffsets`]) are ADR-018
//!   IV-3's counter construction for interval operators: one counter per
//!   occurrence for a lower bound of 0, and sets of open windows or offsets
//!   otherwise.
//! - [`SafetyTranslation`] and [`BoundedTranslation`] are FR-338's canonical
//!   translations of the safety items and the bounded single-interval clauses
//!   EN-1 decides, whose states have the order-independent
//!   [`AutomatonStateKey`] the closure certificate carries.

mod bounded;
mod counters;
mod dnf;
mod error;
mod formula;
mod gba;
mod key;
mod letter;
mod monitor;
mod numbering;
mod past;
mod state;
mod translate;

pub use bounded::{bounded_shape, BoundedKind, BoundedState, BoundedTranslation};
pub use counters::{
    AlwaysOffsets, DeadlineCounter, EventuallyWindows, Expired, ExtentCounter, PastAgeCounter,
    ShiftCounter, Violated,
};
pub use error::AutomatonError;
pub use gba::{Gba, GbaBuilder, Transition};
pub use key::{
    invariant_atom, AutomatonStateKey, ContextActivation, ContextBinding, ContextRoot, KeyRefusal,
    NotTranslated, SafetyItem, SafetyState, SafetyTranslation, StateContext, TranslationRefusal,
    AUTOMATON_STATE_KEY_V1,
};
pub use letter::{Atom, Guard, Letter, MAX_ATOMS};
pub use monitor::{Monitor, SafetyAutomaton};
pub use numbering::{Interned, Numbering};
pub use state::StateId;
pub use translate::{
    translate, translate_body, translate_counted, translate_negation, AtomSymmetry, AutomatonKind,
    PropertyAutomaton,
};

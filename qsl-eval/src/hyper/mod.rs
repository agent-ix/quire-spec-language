// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176 (ADR-023 HC-1, HC-4; ADR-011 §6.1 layer 5): the self-composition
//! product of a hyper clause's subjects, which `qsl-analyze`'s EN-1 explores
//! and the layer-6 product-closure checker (FR-163) expands with the same code
//! (ADR-018 LA-2).
//!
//! [`HyperProduct`] is a FR-101 [`TransitionSystem`](crate::simulation::TransitionSystem)
//! whose state is a tuple of component model states with the state of the
//! generalized Büchi automaton of the negated body. The clause's `holds`
//! atoms and `μ_U` are read through the seams [`HyperLetters`] and
//! [`StepMatch`], the callers' seam to the one clause evaluator (FR-107).

mod product;
mod seam;

pub use product::{
    cartesian, ComponentStep, Halt, HyperFinding, HyperInputs, HyperKey, HyperProduct, HyperState,
    HyperTransition, Property,
};
pub use seam::{HyperLetters, MatchReading, ParameterBinding, StepLabel, StepMatch};

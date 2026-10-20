// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-176, FR-177 (ADR-023 HC-1, HC-2, HC-4; ADR-011 §6.1 layer 5): the
//! self-composition product of a hyper clause's subjects and the witness-set
//! product of a forall-exists safety clause, which `qsl-analyze`'s EN-1
//! explores and the layer-6 checkers (FR-163, FR-183) expand with the same
//! code (ADR-018 LA-2).
//!
//! [`HyperProduct`] is a FR-101 [`TransitionSystem`](crate::simulation::TransitionSystem)
//! whose state is a tuple of component model states with the state of the
//! generalized Büchi automaton of the negated body. The clause's `holds`
//! atoms and `μ_U` are read through the seams [`HyperLetters`] and
//! [`StepMatch`], the callers' seam to the one clause evaluator (FR-107).
//! [`WitnessProduct`] is the same for HP-3: its state is a tuple of universal
//! component states with the set of existential tuples still matched.

mod joint;
mod product;
mod seam;
mod witness;

pub use joint::cartesian;
pub use product::{
    ComponentStep, Halt, HyperFinding, HyperInputs, HyperKey, HyperProduct, HyperState,
    HyperTransition, Property,
};
pub use seam::{HyperLetters, MatchReading, ParameterBinding, StepLabel, StepMatch};
pub use witness::{Member, Partner, PartnerStep, WitnessInputs, WitnessProduct, WitnessSet};

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-174, FR-176, FR-177, FR-178 (ADR-023 HC-1, HC-2, HC-4, PA-4; ADR-011
//! §6.1 layer 5): the self-composition product of a hyper clause's subjects,
//! the witness-set product of a forall-exists safety clause and the projected
//! product of a projection-aligned one, which `qsl-analyze`'s EN-1
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
//! [`ProjectedProduct`] is the same for HP-6: its state is a tuple of component
//! states with the state of the body's safety monitor, and a skipped step moves
//! one component alone (FR-178).
//!
//! A [`HyperProduct`] over a clause whose copies of one model can be swapped
//! also offers the symmetry reduction of that swap (FR-174, ADR-023 CS-2): the
//! canonical form of a state is the least member of its orbit.

mod copy_swap;
mod joint;
mod product;
mod projected;
mod seam;
mod witness;

pub use copy_swap::{copy_population, copy_symmetry, CopySwapInputs};
pub use joint::cartesian;
pub use product::{
    BodySense, ComponentStep, Halt, HyperFinding, HyperInputs, HyperKey, HyperProduct, HyperState,
    HyperTransition, Property,
};
pub use projected::{ProjectedInputs, ProjectedProduct};
pub use seam::{HyperLetters, MatchReading, ParameterBinding, StepLabel, StepMatch};
pub use witness::{Member, Partner, PartnerStep, WitnessInputs, WitnessProduct, WitnessSet};

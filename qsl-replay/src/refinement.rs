// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-020 §2: the layer-6 decisions of a refinement between two state
//! models. [`map_state`] (FR-140) is the one function that computes the
//! abstract state a concrete state stands for; [`check_initial`] and
//! [`check_step`] (FR-141) decide each initial state and step with it. The
//! product, the step check and replay all call these, so a verdict and its
//! replay never disagree about what the abstract model saw.

mod failure;
mod map;
mod outcome;
mod step;

pub use failure::{RefinementFailure, StepRejection, StepVerdict, Taken, Undetermined};
pub use map::{
    map_state, subject_models, EvalFailure, HiddenSlot, MappedState, MappingFailure, MappingRowRef,
    MappingUndetermined, ObjectKey, StepArgument,
};
pub use outcome::{RefinementOutcome, SafetyOutcome, SafetyUndecided};
pub use step::{check_initial, check_step, ConcreteStep, RefinementPosition};

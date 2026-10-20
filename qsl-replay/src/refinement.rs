// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-020 §2: the layer-6 decisions of a refinement between two state
//! models. [`map_state`] (FR-140) is the one function that computes the
//! abstract state a concrete state stands for; [`check_initial`] and
//! [`check_step`] (FR-141) decide each initial state and step with it. The
//! product, the step check and replay all call these, so a verdict and its
//! replay never disagree about what the abstract model saw.

mod failure;
mod history;
mod liveness;
mod map;
mod outcome;
mod step;
mod subject;

pub use failure::{RefinementFailure, StepRejection, StepResult, StepVerdict, Taken, Undetermined};
pub use history::{initial_history, step_history, HistoryValues};
pub use liveness::{abstract_classes, AbstractClass};
pub use map::{
    map_state, EvalFailure, HiddenSlot, MappedState, MappingFailure, MappingRowRef,
    MappingUndetermined, ObjectKey, StepArgument,
};
pub use outcome::{LivenessHalf, RefinementOutcome, SafetyOutcome, SafetyUndecided};
pub use step::{check_initial, check_step, ConcreteStep, RefinementPosition};
pub use subject::{open_systems, RefinementSystems, SubjectRefusal};

pub(crate) use step::stop_verdict;

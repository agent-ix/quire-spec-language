// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-020 §2: the layer-6 decisions of a refinement between two state
//! models. [`map_state`] (FR-140) is the one function that computes the
//! abstract state a concrete state stands for; the step check, the product
//! and replay all call it, so a verdict and its replay never disagree about
//! what the abstract model saw.

mod map;

pub use map::{
    map_state, EvalFailure, HiddenSlot, MappedState, MappingFailure, MappingFeature, MappingRowRef,
    MappingUndetermined, ObjectKey,
};

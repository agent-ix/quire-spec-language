// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-143 "Enabled and taken through the mapping" (ADR-018 FA-1, ADR-020
//! CO-2): the classes of the abstract fairness set `F_A`, and when a class is
//! enabled at a mapped state or taken by a step.
//!
//! The engine's second phase and replay's lasso check both read the classes
//! here, so a lasso replay never disagrees with the engine about which
//! abstract operation a loop leaves undelivered.

use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::InternalFault;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity};
use quire_semantic_value::model_transition::ModelTransition;

use crate::refinement::failure::Taken;

/// One class of `F_A`: a `whole` constraint's operation, or one transition
/// identity of an `each` constraint's operation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AbstractClass {
    constraint: usize,
    operation: String,
    identity: Option<ModelTransition>,
}

impl AbstractClass {
    /// The index of the class's constraint in `F_A`.
    pub fn constraint(&self) -> usize {
        self.constraint
    }

    /// Whether `transition` is of the class.
    pub fn covers(&self, transition: &ModelTransition) -> bool {
        transition.operation() == self.operation
            && self
                .identity
                .as_ref()
                .is_none_or(|identity| identity == transition)
    }

    /// Whether the abstract subject enables the class at a mapped state whose
    /// enabled transition identities are `enabled`.
    pub fn is_enabled_in(&self, enabled: &[ModelTransition]) -> bool {
        enabled.iter().any(|transition| self.covers(transition))
    }

    /// Whether a step that took `taken` takes the class. A `stutter` step
    /// takes none.
    pub fn is_taken_by(&self, taken: &Taken) -> bool {
        match taken {
            Taken::Stutter => false,
            Taken::Abstract(transitions) => {
                transitions.iter().any(|transition| self.covers(transition))
            }
        }
    }
}

/// The classes of `fairness`, in its source order and, within an `each`
/// constraint, in the order of `identities`, which are the abstract
/// transition identities the behaviour enables or takes.
///
/// # Errors
///
/// [`InternalFault`] when a constraint names an operation the abstract
/// subject does not declare, which the checked refinement rules out.
pub fn abstract_classes(
    system: &ModelSystem<'_>,
    fairness: &[FairnessConstraint],
    identities: &[ModelTransition],
) -> Result<Vec<AbstractClass>, InternalFault> {
    let mut classes = Vec::new();
    for (index, constraint) in fairness.iter().enumerate() {
        let operation = constraint.operation();
        let text = system
            .operation_text(operation.declaring_type(), operation.name())
            .ok_or_else(|| {
                InternalFault::new(
                    "refinement-liveness",
                    "an-abstract-fairness-names-a-declared-operation",
                )
            })?;
        match constraint.granularity() {
            FairnessGranularity::Whole => classes.push(AbstractClass {
                constraint: index,
                operation: text.to_owned(),
                identity: None,
            }),
            FairnessGranularity::Each => classes.extend(
                identities
                    .iter()
                    .filter(|transition| transition.operation() == text)
                    .map(|transition| AbstractClass {
                        constraint: index,
                        operation: text.to_owned(),
                        identity: Some(transition.clone()),
                    }),
            ),
        }
    }
    Ok(classes)
}

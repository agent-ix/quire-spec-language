// SPDX-License-Identifier: AGPL-3.0-or-later
//! Lasso fairness: whether the loop of a given lasso is fair (ADR-018 FA-3,
//! CX-3), as replay reads it.

use std::collections::BTreeSet;

use crate::fairness::constraint::{Constraint, Obligation, Strength};
use crate::fairness::error::FairnessError;
use crate::fairness::graph::{EdgeLabel, Identity};

/// One step of a lasso's loop: the identities enabled at the state the step
/// leaves and what the step takes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoopStep {
    enabled: BTreeSet<Identity>,
    label: EdgeLabel,
}

impl LoopStep {
    /// A step taking `label` from a state where exactly the identities in
    /// `enabled` are enabled.
    pub fn new(enabled: impl IntoIterator<Item = Identity>, label: EdgeLabel) -> Self {
        Self {
            enabled: enabled.into_iter().collect(),
            label,
        }
    }
}

/// Whether a loop is fair.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LassoFairness {
    /// The loop is fair under every constraint.
    Fair,
    /// The loop is unfair under the constraint at `constraint`, the first
    /// such in the fairness set.
    Unfair {
        /// The constraint's index in the fairness set.
        constraint: usize,
    },
}

/// Whether `steps`, the loop of a lasso, is fair under `constraints`. A weak
/// constraint is fair on the loop when it is taken in it or disabled
/// somewhere in it; a strong constraint when it is taken in it or enabled
/// nowhere in it. A visibility constraint (ADR-025 MF-2) is fair on the
/// loop when the loop loads its pair from no message, or loads it from the
/// mo-last message somewhere.
///
/// # Errors
///
/// [`FairnessError::EmptyLoop`] when `steps` is empty.
pub fn check_loop(
    steps: &[LoopStep],
    constraints: &[Constraint],
) -> Result<LassoFairness, FairnessError> {
    if steps.is_empty() {
        return Err(FairnessError::EmptyLoop);
    }
    for (index, constraint) in constraints.iter().enumerate() {
        let taken = steps.iter().any(|step| constraint.is_taken_by(step.label));
        let enabled_somewhere = steps
            .iter()
            .any(|step| constraint.is_enabled_in(&step.enabled));
        let disabled_somewhere = steps
            .iter()
            .any(|step| !constraint.is_enabled_in(&step.enabled));
        let fair = match (constraint.strength(), constraint.obligation()) {
            (Strength::Weak, Obligation::Enablement) => taken || disabled_somewhere,
            (Strength::Weak, Obligation::Visibility { .. }) => {
                taken || !steps.iter().any(|step| constraint.is_owed_by(step.label))
            }
            (Strength::Strong, Obligation::Enablement | Obligation::Visibility { .. }) => {
                taken || !enabled_somewhere
            }
        };
        if !fair {
            return Ok(LassoFairness::Unfair { constraint: index });
        }
    }
    Ok(LassoFairness::Fair)
}

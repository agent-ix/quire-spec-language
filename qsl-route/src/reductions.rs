// SPDX-License-Identifier: AGPL-3.0-or-later
//! The state-space reductions a request selects (FR-159 Inputs), which
//! negotiation routes by (FR-159 "Capability") and which, but for the
//! constraint, enter each item's obligation identity (FR-159 "Identity",
//! ADR-013 O-09). The engines read the same selection.

use qsl_foundation::ReductionKind;
use quire_checked::key::DeclarationKey;

/// One symmetry declaration of a request (FR-151): classes of
/// interchangeable keys of `population`'s universe. A key in no class, or in
/// a class of one, is fixed.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SymmetryDeclaration {
    /// The population.
    pub population: DeclarationKey,
    /// The classes, each a list of keys of the population's universe.
    pub classes: Vec<Vec<String>>,
}

/// The reductions a request selects (FR-159 Inputs). The default selects
/// none.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Reductions {
    /// The symmetry declarations; empty selects no symmetry.
    pub symmetry: Vec<SymmetryDeclaration>,
    /// Whether partial-order reduction is selected.
    pub partial_order: bool,
    /// The state constraint's clause name, when one is selected.
    pub constraint: Option<String>,
}

impl Reductions {
    /// Whether the request selects no reduction.
    pub fn is_empty(&self) -> bool {
        self.symmetry.is_empty() && !self.partial_order && self.constraint.is_none()
    }

    /// The selected reductions, in the order symmetry, partial-order,
    /// constraint.
    pub fn selected(&self) -> Vec<ReductionKind> {
        let mut selected = Vec::new();
        if !self.symmetry.is_empty() {
            selected.push(ReductionKind::Symmetry);
        }
        if self.partial_order {
            selected.push(ReductionKind::PartialOrder);
        }
        if self.constraint.is_some() {
            selected.push(ReductionKind::StateConstraint);
        }
        selected
    }

    /// The part of the selection that enters an item's obligation
    /// identity (FR-159 "Identity"): the symmetry declarations, classes in
    /// request order, and the partial-order selection. The constraint is a
    /// method parameter and stays out (FR-158).
    pub fn obligation_identity(&self) -> ObligationReductions {
        ObligationReductions {
            symmetry: self.symmetry.clone(),
            partial_order: self.partial_order,
        }
    }
}

/// The reductions in an item's obligation identity (FR-159 "Identity"):
/// none for an unreduced request.
#[derive(Clone, Debug, Default, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObligationReductions {
    /// The symmetry declarations, in request order.
    pub symmetry: Vec<SymmetryDeclaration>,
    /// Whether partial-order reduction is selected.
    pub partial_order: bool,
}

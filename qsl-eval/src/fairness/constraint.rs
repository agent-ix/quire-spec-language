// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fairness constraints and their membership (ADR-018 FA-1 to FA-4).

use std::collections::BTreeSet;

use crate::fairness::graph::{EdgeLabel, Identity};

/// How strongly a constraint binds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Strength {
    /// Weak fairness (ADR-018 FA-3): from no position onward is the
    /// constraint enabled at every position while none of its transitions is
    /// taken.
    Weak,
    /// Strong fairness (ADR-019 SF): a constraint enabled infinitely often
    /// is taken infinitely often.
    Strong,
}

impl From<quire_semantic_value::fairness::FairnessKind> for Strength {
    fn from(kind: quire_semantic_value::fairness::FairnessKind) -> Self {
        match kind {
            quire_semantic_value::fairness::FairnessKind::Weak => Self::Weak,
            quire_semantic_value::fairness::FairnessKind::Strong => Self::Strong,
        }
    }
}

/// One fairness constraint: its strength and the transition identities it
/// covers. A `whole` constraint covers every identity of its operation, an
/// `each` constraint one identity. The order of constraints in a fairness
/// set is the order the kernels report witnesses in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Constraint {
    strength: Strength,
    members: BTreeSet<Identity>,
}

impl Constraint {
    /// A constraint of `strength` covering `members`.
    pub fn new(strength: Strength, members: impl IntoIterator<Item = Identity>) -> Self {
        Self {
            strength,
            members: members.into_iter().collect(),
        }
    }

    /// A weak constraint covering `members`.
    pub fn weak(members: impl IntoIterator<Item = Identity>) -> Self {
        Self::new(Strength::Weak, members)
    }

    /// A strong constraint covering `members`.
    pub fn strong(members: impl IntoIterator<Item = Identity>) -> Self {
        Self::new(Strength::Strong, members)
    }

    /// The constraint's strength.
    pub const fn strength(&self) -> Strength {
        self.strength
    }

    /// Whether the constraint covers `identity`.
    pub fn covers(&self, identity: Identity) -> bool {
        self.members.contains(&identity)
    }

    /// Whether an edge labelled `label` takes the constraint. The terminal
    /// stutter step belongs to no constraint.
    pub fn is_taken_by(&self, label: EdgeLabel) -> bool {
        match label {
            EdgeLabel::Transition(identity) => self.covers(identity),
            EdgeLabel::Stutter => false,
        }
    }

    /// Whether the constraint is enabled where exactly the identities in
    /// `enabled` are: a `whole` constraint is enabled when any of its
    /// identities is (ADR-018 FA-2).
    pub fn is_enabled_in(&self, enabled: &BTreeSet<Identity>) -> bool {
        self.members.iter().any(|member| enabled.contains(member))
    }
}

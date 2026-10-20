// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fairness constraints and their membership (ADR-018 FA-1 to FA-5; ADR-025
//! MF-2).

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

/// What owes a constraint on a cycle, and so which filter rule reads it
/// (ADR-018 FA-5: a new kind adds a variant and its filter rule).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Obligation {
    /// ADR-018 FA-2 to FA-4 and ADR-019: owed where the constraint is
    /// enabled, read from each vertex's enabled set, weakly or strongly.
    Enablement(Strength),
    /// ADR-025 MF-2's visibility fairness for one (thread, location) pair,
    /// an edge Streett pair: a cycle that takes one of `loads`, the pair's
    /// load identities at every message depth, also takes one of the
    /// constraint's members, the loads of the location's mo-last message
    /// (depth 0). It is weak (MF-5) and has no enabledness.
    Visibility {
        /// Every load identity of the pair, the fresh loads among them.
        loads: BTreeSet<Identity>,
    },
}

/// One fairness constraint: the transition identities that take it and what
/// owes it. A `whole` constraint covers every identity of its operation, an
/// `each` constraint one identity. The order of constraints in a fairness
/// set is the order the kernels report witnesses in.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Constraint {
    members: BTreeSet<Identity>,
    obligation: Obligation,
}

impl Constraint {
    /// A constraint of `strength` covering `members`, owed where enabled.
    pub fn new(strength: Strength, members: impl IntoIterator<Item = Identity>) -> Self {
        Self {
            members: members.into_iter().collect(),
            obligation: Obligation::Enablement(strength),
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

    /// ADR-025 MF-2's visibility constraint for one (thread, location) pair:
    /// a cycle that takes any of `loads` must take one of `fresh`, the loads
    /// of the location's mo-last message. A fresh load is itself a load, so
    /// `fresh` joins `loads`.
    pub fn visibility(
        loads: impl IntoIterator<Item = Identity>,
        fresh: impl IntoIterator<Item = Identity>,
    ) -> Self {
        let members: BTreeSet<Identity> = fresh.into_iter().collect();
        let mut loads: BTreeSet<Identity> = loads.into_iter().collect();
        loads.extend(members.iter().copied());
        Self {
            members,
            obligation: Obligation::Visibility { loads },
        }
    }

    /// The constraint's strength: its enablement strength, or weak for a
    /// visibility constraint (ADR-025 MF-5).
    pub const fn strength(&self) -> Strength {
        match self.obligation {
            Obligation::Enablement(strength) => strength,
            Obligation::Visibility { .. } => Strength::Weak,
        }
    }

    /// What owes the constraint on a cycle.
    pub const fn obligation(&self) -> &Obligation {
        &self.obligation
    }

    /// Whether the constraint covers `identity`.
    pub fn covers(&self, identity: Identity) -> bool {
        self.members.contains(&identity)
    }

    /// Whether an edge labelled `label` takes the constraint, which
    /// discharges it. The terminal stutter step belongs to no constraint.
    pub fn is_taken_by(&self, label: EdgeLabel) -> bool {
        match label {
            EdgeLabel::Transition(identity) => self.covers(identity),
            EdgeLabel::Stutter => false,
        }
    }

    /// Whether an edge labelled `label` makes a cycle owe a
    /// [`Obligation::Visibility`] constraint: it loads the pair at any
    /// depth. An [`Obligation::Enablement`] constraint is owed by
    /// enabledness, never by an edge.
    pub fn is_owed_by(&self, label: EdgeLabel) -> bool {
        match (&self.obligation, label) {
            (Obligation::Visibility { loads }, EdgeLabel::Transition(identity)) => {
                loads.contains(&identity)
            }
            (Obligation::Visibility { .. }, EdgeLabel::Stutter)
            | (Obligation::Enablement(_), _) => false,
        }
    }

    /// Whether the constraint is enabled where exactly the identities in
    /// `enabled` are: a `whole` constraint is enabled when any of its
    /// identities is (ADR-018 FA-2). `None` for a
    /// [`Obligation::Visibility`] constraint, which has no enabledness.
    pub fn is_enabled_in(&self, enabled: &BTreeSet<Identity>) -> Option<bool> {
        match self.obligation {
            Obligation::Enablement(_) => {
                Some(self.members.iter().any(|member| enabled.contains(member)))
            }
            Obligation::Visibility { .. } => None,
        }
    }
}

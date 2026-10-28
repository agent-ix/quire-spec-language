// SPDX-License-Identifier: AGPL-3.0-or-later
//! The state-space reductions a request selects and a provider advertises
//! (QSL FR-159, FR-160; QSpec FR-382). The kind lives here, below both the
//! route registry, which negotiates by it, and the terminal records, which
//! name it.

/// A reduction a request selects (FR-160 `ReductionKind`).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReductionKind {
    /// Symmetry over population universes.
    Symmetry,
    /// Partial-order reduction.
    PartialOrder,
    /// A state constraint.
    StateConstraint,
}

impl ReductionKind {
    /// Every reduction, in the order symmetry, partial-order, constraint,
    /// the order in which a request's selections are judged (FR-159).
    pub const ALL: [Self; 3] = [Self::Symmetry, Self::PartialOrder, Self::StateConstraint];

    /// The reduction's one wire spelling wherever a reduction is named: the
    /// provider manifest's `reductions` member, the
    /// `reduction-not-preserving` cause and a proof basis's applied
    /// reductions (FR-159 "Capability", QSpec FR-382).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Symmetry => "symmetry",
            Self::PartialOrder => "partial-order",
            Self::StateConstraint => "constraint",
        }
    }

    /// The reduction `label` spells, if any.
    pub fn from_wire(label: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == label)
    }
}

#[cfg(test)]
mod tests {
    use super::ReductionKind;
    use ix_trace_rs::trace;

    /// FR-159 "Capability": `constraint` is the one spelling of the
    /// state-constraint reduction; `state-constraint` spells none.
    #[trace("TC-583", "FR-159-AC-5")]
    #[test]
    fn each_reduction_has_one_wire_spelling() {
        for kind in ReductionKind::ALL {
            assert_eq!(ReductionKind::from_wire(kind.as_str()), Some(kind));
        }
        assert_eq!(ReductionKind::StateConstraint.as_str(), "constraint");
        assert_eq!(ReductionKind::from_wire("state-constraint"), None);
    }
}

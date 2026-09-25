// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-014 §4 and §11 (QSL-140): the wire-level bound types that cross from
//! layer 3 to the O-20 request writer, to CG and to replay.
//!
//! - [`DomainKey`] names one unbounded domain of a requested item: the
//!   checked node that carries it, then the child-index path into that
//!   node's type.
//! - [`FiniteBound`] is the finite domain a caller substitutes for one
//!   unbounded domain. Each variant is valid by construction: an empty or
//!   inverted range cannot be built.
//! - [`ProofBound`] pairs the two. It is ADR-014 B-4, the proof bound: part
//!   of a bounded request's obligation identity, never a resource limit.
//! - [`IntervalKey`] is QSpec FR-255's key of one bounded temporal interval,
//!   in its wire form, so `qsl-replay` can name it.
//!
//! ADR-014 §1: no value of another bound kind converts into these types.
//! This module defines no `From`/`TryFrom` from an authored bound (B-1), an
//! accounting limit (B-2), a stage limit (B-3), a backend tool budget (B-5)
//! or a profile ceiling (B-6). A caller builds a [`FiniteBound`] explicitly.
//!
//! The module imports only K (`quire-exact`) and F's own `digest` and
//! `selection`: `selection` because an [`IntervalKey`] names its profile by
//! [`DefinitionRef`].

use std::fmt;
use std::num::NonZeroU64;

use quire_exact::{EmptyInterval, Integer, IntegerInterval};

use crate::digest::WireNodeId;
use crate::selection::DefinitionRef;

/// One unbounded domain of a requested item (ADR-014 §4 "Domain key").
///
/// `node` is the checked node that carries the domain: a parameter, a bound
/// variable, a loop or a temporal clause. `path` is the child-index path
/// into that node's type (element, field, variant payload); a loop or a
/// clause has an empty path. Keys order by node, then path, so a map keyed
/// by them iterates deterministically.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DomainKey {
    node: WireNodeId,
    path: Vec<u32>,
}

impl DomainKey {
    /// The domain at `path` inside `node`'s type.
    pub fn new(node: WireNodeId, path: Vec<u32>) -> Self {
        Self { node, path }
    }

    /// The checked node that carries the domain.
    pub fn node(&self) -> WireNodeId {
        self.node
    }

    /// The child-index path into the node's type. Empty for the node
    /// itself.
    pub fn path(&self) -> &[u32] {
        &self.path
    }
}

impl fmt::Display for DomainKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.node)?;
        self.path.iter().try_for_each(|index| write!(f, "/{index}"))
    }
}

/// Which [`FiniteBound`] variant a domain takes (ADR-014 §4's boundable
/// table, right column).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum FiniteBoundKind {
    /// A collection or population cardinality maximum.
    Cardinality,
    /// An inclusive integer range.
    IntegerRange,
    /// A recursive value type's depth maximum.
    Depth,
}

/// A finite domain for one unbounded domain (ADR-014 B-4, §4).
///
/// Every value is non-empty: `IntegerRange` holds a kernel
/// [`IntegerInterval`], which refuses `lower > upper`, and `Depth` holds a
/// non-zero maximum, since no value has depth zero. `Cardinality{maximum}`
/// admits the counts `0..=maximum`, which is never empty.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FiniteBound {
    /// At most `maximum` members (collection) or instances (population).
    Cardinality {
        /// The inclusive maximum.
        maximum: u64,
    },
    /// An integer in the inclusive range.
    IntegerRange(IntegerInterval),
    /// A recursive value at most `maximum` levels deep.
    Depth {
        /// The inclusive maximum depth.
        maximum: NonZeroU64,
    },
}

/// A [`FiniteBound`] constructor was given an empty or inverted range.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EmptyFiniteBound {
    /// `lower > upper` for an integer range.
    #[error("integer range lower bound exceeds its upper bound")]
    InvertedIntegerRange,
    /// A depth maximum of zero admits no value.
    #[error("depth maximum zero admits no value")]
    ZeroDepth,
}

impl From<EmptyInterval> for EmptyFiniteBound {
    fn from(_: EmptyInterval) -> Self {
        Self::InvertedIntegerRange
    }
}

impl FiniteBound {
    /// `Cardinality{maximum}`.
    pub fn cardinality(maximum: u64) -> Self {
        Self::Cardinality { maximum }
    }

    /// `IntegerRange[lower, upper]`, refusing `lower > upper`.
    pub fn integer_range(lower: Integer, upper: Integer) -> Result<Self, EmptyFiniteBound> {
        Ok(Self::IntegerRange(IntegerInterval::new(lower, upper)?))
    }

    /// `Depth{maximum}`, refusing zero.
    pub fn depth(maximum: u64) -> Result<Self, EmptyFiniteBound> {
        NonZeroU64::new(maximum)
            .map(|maximum| Self::Depth { maximum })
            .ok_or(EmptyFiniteBound::ZeroDepth)
    }

    /// Which variant this is.
    pub fn kind(&self) -> FiniteBoundKind {
        match self {
            Self::Cardinality { .. } => FiniteBoundKind::Cardinality,
            Self::IntegerRange(_) => FiniteBoundKind::IntegerRange,
            Self::Depth { .. } => FiniteBoundKind::Depth,
        }
    }
}

/// One caller-supplied proof bound (ADR-014 B-4): the finite domain
/// substituted for one unbounded domain of an item. It is part of the
/// bounded request's obligation identity (ADR-013 O-09), and a result
/// qualifies only over it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ProofBound {
    /// The unbounded domain this bound replaces.
    pub domain: DomainKey,
    /// The finite domain substituted for it.
    pub bound: FiniteBound,
}

/// QSpec FR-255's key of one bounded temporal interval, in wire form
/// (ADR-014 TR-3, §10 scenario 5): (`lower`, `upper`, selected profile
/// identity, clock binding). Equality is componentwise.
///
/// `clock_binding` is the wire node id of the checked clock-binding node the
/// interval's offsets count under. A checked node id is content-derived, so
/// two bindings with equal components have equal ids, which is the
/// componentwise equality QSpec FR-252 requires.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IntervalKey {
    lower: u64,
    upper: u64,
    profile: DefinitionRef,
    clock_binding: WireNodeId,
}

/// An [`IntervalKey`] with `lower > upper` (QSpec FR-091, FR-255).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("temporal interval [{lower}, {upper}] is inverted")]
pub struct InvertedInterval {
    /// The refused lower bound.
    pub lower: u64,
    /// The refused upper bound.
    pub upper: u64,
}

impl IntervalKey {
    /// The key of `[lower, upper]` under `profile` and `clock_binding`,
    /// refusing `lower > upper`.
    pub fn new(
        lower: u64,
        upper: u64,
        profile: DefinitionRef,
        clock_binding: WireNodeId,
    ) -> Result<Self, InvertedInterval> {
        if lower > upper {
            return Err(InvertedInterval { lower, upper });
        }
        Ok(Self {
            lower,
            upper,
            profile,
            clock_binding,
        })
    }

    /// The inclusive lower offset.
    pub fn lower(&self) -> u64 {
        self.lower
    }

    /// The inclusive upper offset.
    pub fn upper(&self) -> u64 {
        self.upper
    }

    /// The selected temporal profile.
    pub fn profile(&self) -> &DefinitionRef {
        &self.profile
    }

    /// The clock-binding node the offsets count under.
    pub fn clock_binding(&self) -> WireNodeId {
        self.clock_binding
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selection::DefinitionDigest;
    use crate::ByteDigest;
    use ix_trace_rs::trace;

    fn node(fill: u8) -> WireNodeId {
        WireNodeId::from_digest([fill; 32])
    }

    fn profile() -> DefinitionRef {
        DefinitionRef::new(
            "quire.temporal.event-position/v1",
            "1",
            DefinitionDigest::from_digest(ByteDigest::of(b"profile")),
        )
        .unwrap()
    }

    /// ADR-014 §4: an integer range's constructor refuses an inverted range,
    /// a depth's refuses zero, and every admitted bound reports its kind.
    #[trace("TC-436", "FR-097-AC-1")]
    #[test]
    fn finite_bounds_refuse_empty_ranges_and_report_their_kind() {
        assert_eq!(
            FiniteBound::integer_range(Integer::from(5_i64), Integer::from(4_i64)),
            Err(EmptyFiniteBound::InvertedIntegerRange)
        );
        assert_eq!(FiniteBound::depth(0), Err(EmptyFiniteBound::ZeroDepth));
        let one_point = FiniteBound::integer_range(Integer::from(4_i64), Integer::from(4_i64))
            .expect("a one-point range is not empty");
        assert_eq!(one_point.kind(), FiniteBoundKind::IntegerRange);
        assert_eq!(
            FiniteBound::cardinality(0).kind(),
            FiniteBoundKind::Cardinality
        );
        assert_eq!(
            FiniteBound::depth(1).unwrap().kind(),
            FiniteBoundKind::Depth
        );
    }

    /// ADR-014 §4: domain keys order by node, then path, and name the
    /// node's own position with an empty path.
    #[trace("TC-436", "FR-097-AC-1")]
    #[test]
    fn domain_keys_order_by_node_then_path() {
        let root = DomainKey::new(node(1), Vec::new());
        let element = DomainKey::new(node(1), vec![0]);
        let other = DomainKey::new(node(2), Vec::new());
        assert!(root < element);
        assert!(element < other);
        assert_ne!(root, element);
        assert_eq!(element.to_string(), format!("{}/0", node(1)));
    }

    /// ADR-014 TR-3: an interval key refuses `lower > upper` and compares
    /// componentwise, so a different clock binding is a different key.
    #[trace("TC-436", "FR-097-AC-1")]
    #[test]
    fn interval_keys_refuse_inversion_and_compare_componentwise() {
        assert_eq!(
            IntervalKey::new(3, 2, profile(), node(9)),
            Err(InvertedInterval { lower: 3, upper: 2 })
        );
        let key = IntervalKey::new(0, 4, profile(), node(9)).unwrap();
        assert_eq!(key, IntervalKey::new(0, 4, profile(), node(9)).unwrap());
        assert_ne!(key, IntervalKey::new(0, 4, profile(), node(8)).unwrap());
        assert_ne!(key, IntervalKey::new(1, 4, profile(), node(9)).unwrap());
        assert_eq!((key.lower(), key.upper()), (0, 4));
    }
}

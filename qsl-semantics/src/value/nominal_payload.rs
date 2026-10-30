// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facts a nominal declaration node carries beyond its structural key:
//! an enum's ordering and cases, a dimension's terms, a unit's dimension,
//! target and conversion, and a dimension equivalence's pair and scale
//! (FR-322 "Nominal nodes"). Plain data with borrowed accessors; a node key
//! cannot stand in for these facts.

use quire_exact::{Integer, NodeKey, Rational};

use super::enumeration::EnumDeclarationPreimage;
use super::unit::{DimensionEquivalencePreimage, DimensionPreimage, UnitPreimage};

/// The payload of one nominal node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NominalPayload {
    /// An enum declaration.
    Enum(EnumPayload),
    /// A dimension.
    Dimension(DimensionPayload),
    /// A declared unit.
    Unit(UnitPayload),
    /// A dimension equivalence.
    DimensionEquivalence(DimensionEquivalencePayload),
}

/// An enum declaration's ordering and cases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumPayload {
    ordered: bool,
    members: Vec<String>,
}

impl EnumPayload {
    /// An enum that is `ordered` or not, with `members` (the case texts in
    /// declaration order, or sorted for an unordered enum).
    pub fn new(ordered: bool, members: Vec<String>) -> Self {
        Self { ordered, members }
    }

    /// The payload of `preimage`.
    pub fn of(preimage: &EnumDeclarationPreimage) -> Self {
        Self::new(preimage.is_ordered(), preimage.members().to_vec())
    }

    /// Whether the enum selects ordered semantics.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// The case names, in the order the declaration holds them.
    pub fn members(&self) -> &[String] {
        &self.members
    }
}

/// A dimension: its `(base dimension node, exponent)` terms, empty for a
/// base dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionPayload {
    terms: Vec<(NodeKey, Integer)>,
}

impl DimensionPayload {
    /// A dimension over `terms`; empty terms declare a base dimension.
    pub fn new(terms: Vec<(NodeKey, Integer)>) -> Self {
        Self { terms }
    }

    /// The payload of `preimage`.
    pub fn of(preimage: &DimensionPreimage) -> Self {
        Self::new(preimage.terms().to_vec())
    }

    /// The `(dimension node, exponent)` terms, strictly ascending by key.
    pub fn terms(&self) -> &[(NodeKey, Integer)] {
        &self.terms
    }

    /// Whether this is a base dimension.
    pub fn is_base(&self) -> bool {
        self.terms.is_empty()
    }
}

/// A declared unit: its dimension, its conversion target and the exact
/// conversion `value_in_target = factor * value + offset`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitPayload {
    dimension: NodeKey,
    target: Option<NodeKey>,
    factor: Rational,
    offset: Rational,
}

impl UnitPayload {
    /// A unit of `dimension` converting to `target` (`None` for a root unit)
    /// by `factor` and `offset`.
    pub fn new(
        dimension: NodeKey,
        target: Option<NodeKey>,
        factor: Rational,
        offset: Rational,
    ) -> Self {
        Self {
            dimension,
            target,
            factor,
            offset,
        }
    }

    /// The payload of `preimage`, or `None` when a spelled rational is not a
    /// valid rational (a zero denominator).
    pub fn of(preimage: &UnitPreimage) -> Option<Self> {
        let (scale_n, scale_d) = preimage.scale();
        let (offset_n, offset_d) = preimage.offset();
        Some(Self::new(
            preimage.dimension().clone(),
            preimage.target().cloned(),
            Rational::new(scale_n.clone(), scale_d.clone()).ok()?,
            Rational::new(offset_n.clone(), offset_d.clone()).ok()?,
        ))
    }

    /// The dimension node of the unit.
    pub fn dimension(&self) -> &NodeKey {
        &self.dimension
    }

    /// The unit node this unit converts to, or `None` for a root unit.
    pub fn target(&self) -> Option<&NodeKey> {
        self.target.as_ref()
    }

    /// The exact scale factor.
    pub fn factor(&self) -> &Rational {
        &self.factor
    }

    /// The exact offset.
    pub fn offset(&self) -> &Rational {
        &self.offset
    }
}

/// A dimension equivalence: the two related dimension nodes in ascending key
/// order and the root ratio stated for the first.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionEquivalencePayload {
    dimensions: [NodeKey; 2],
    scale: Rational,
}

impl DimensionEquivalencePayload {
    /// An equivalence of `dimensions` with root `scale`.
    pub fn new(dimensions: [NodeKey; 2], scale: Rational) -> Self {
        Self { dimensions, scale }
    }

    /// The payload of `preimage`.
    pub fn of(preimage: &DimensionEquivalencePreimage) -> Self {
        Self::new(preimage.dimensions(), preimage.root_scale().clone())
    }

    /// The two dimension nodes, ascending.
    pub fn dimensions(&self) -> &[NodeKey; 2] {
        &self.dimensions
    }

    /// The root ratio stated for the first dimension.
    pub fn scale(&self) -> &Rational {
        &self.scale
    }
}

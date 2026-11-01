// SPDX-License-Identifier: AGPL-3.0-or-later
//! The facts a nominal declaration node carries beyond its structural key:
//! an enum's ordering and cases, a dimension's terms, a unit's dimension,
//! target and conversion, and a dimension equivalence's pair and scale
//! (FR-322 "Nominal nodes"). Plain data with borrowed accessors; a node key
//! cannot stand in for these facts.

use qsl_foundation::source::node_table::{NodeKeyDebug, NodeList};
use quire_exact::{Integer, NodeKey, Rational};

use super::enumeration::EnumDeclarationPreimage;
use super::semantic_node::NodeOwner;
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

impl NominalPayload {
    /// The declaring owner a preimage-read payload retains.
    pub fn owner(&self) -> Option<&NodeOwner> {
        match self {
            Self::Enum(payload) => payload.owner(),
            Self::Dimension(payload) => payload.owner(),
            Self::Unit(payload) => payload.owner(),
            Self::DimensionEquivalence(_) => None,
        }
    }
}

/// An enum declaration's ordering and cases.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumPayload {
    ordered: bool,
    members: Vec<String>,
    owner: Option<NodeOwner>,
}

impl EnumPayload {
    /// An enum that is `ordered` or not, with `members` (the case texts in
    /// declaration order, or sorted for an unordered enum).
    pub fn new(ordered: bool, members: Vec<String>) -> Self {
        Self {
            ordered,
            members,
            owner: None,
        }
    }

    /// The payload of `preimage`.
    pub fn of(preimage: &EnumDeclarationPreimage) -> Self {
        let mut payload = Self::new(preimage.is_ordered(), preimage.members().to_vec());
        payload.owner = Some(preimage.owner().clone());
        payload
    }

    /// Whether the enum selects ordered semantics.
    pub fn is_ordered(&self) -> bool {
        self.ordered
    }

    /// The case names, in the order the declaration holds them.
    pub fn members(&self) -> &[String] {
        &self.members
    }

    /// The declaring owner, when the payload was read from a preimage.
    pub fn owner(&self) -> Option<&NodeOwner> {
        self.owner.as_ref()
    }
}

/// A dimension: its `(base dimension node, exponent)` terms, empty for a
/// base dimension.
#[derive(Clone, Eq, PartialEq)]
pub struct DimensionPayload {
    terms: Vec<(NodeKey, Integer)>,
    owner: Option<NodeOwner>,
}

impl std::fmt::Debug for DimensionPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { terms, owner } = self;
        f.debug_struct("DimensionPayload")
            .field(
                "terms",
                &terms
                    .iter()
                    .map(|value| (NodeKeyDebug(&value.0), &value.1))
                    .collect::<Vec<_>>(),
            )
            .field("owner", &owner)
            .finish()
    }
}

impl DimensionPayload {
    /// A dimension over `terms`; empty terms declare a base dimension.
    pub fn new(terms: Vec<(NodeKey, Integer)>) -> Self {
        Self { terms, owner: None }
    }

    /// The payload of `preimage`.
    pub fn of(preimage: &DimensionPreimage) -> Self {
        let mut payload = Self::new(preimage.terms().to_vec());
        payload.owner = Some(preimage.owner().clone());
        payload
    }

    /// The `(dimension node, exponent)` terms, strictly ascending by key.
    pub fn terms(&self) -> &[(NodeKey, Integer)] {
        &self.terms
    }

    /// The declaring owner, when the payload was read from a preimage.
    pub fn owner(&self) -> Option<&NodeOwner> {
        self.owner.as_ref()
    }

    /// Whether this is a base dimension.
    pub fn is_base(&self) -> bool {
        self.terms.is_empty()
    }
}

/// A declared unit: its dimension, its conversion target and the exact
/// conversion `value_in_target = factor * value + offset`.
#[derive(Clone, Eq, PartialEq)]
pub struct UnitPayload {
    dimension: NodeKey,
    target: Option<NodeKey>,
    factor: Rational,
    offset: Rational,
    owner: Option<NodeOwner>,
}

impl std::fmt::Debug for UnitPayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self {
            dimension,
            target,
            factor,
            offset,
            owner,
        } = self;
        f.debug_struct("UnitPayload")
            .field("dimension", &NodeKeyDebug(dimension))
            .field("target", &target.as_ref().map(NodeKeyDebug))
            .field("factor", &factor)
            .field("offset", &offset)
            .field("owner", &owner)
            .finish()
    }
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
            owner: None,
        }
    }

    /// The payload of `preimage`, or `None` when a spelled rational is not a
    /// valid rational (a zero denominator).
    pub fn of(preimage: &UnitPreimage) -> Option<Self> {
        let (scale_n, scale_d) = preimage.scale();
        let (offset_n, offset_d) = preimage.offset();
        let mut payload = Self::new(
            preimage.dimension().clone(),
            preimage.target().cloned(),
            Rational::new(scale_n.clone(), scale_d.clone()).ok()?,
            Rational::new(offset_n.clone(), offset_d.clone()).ok()?,
        );
        payload.owner = Some(preimage.owner().clone());
        Some(payload)
    }

    /// The declaring owner, when the payload was read from a preimage.
    pub fn owner(&self) -> Option<&NodeOwner> {
        self.owner.as_ref()
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
#[derive(Clone, Eq, PartialEq)]
pub struct DimensionEquivalencePayload {
    dimensions: [NodeKey; 2],
    scale: Rational,
}

impl std::fmt::Debug for DimensionEquivalencePayload {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { dimensions, scale } = self;
        f.debug_struct("DimensionEquivalencePayload")
            .field("dimensions", &NodeList(dimensions.iter()))
            .field("scale", &scale)
            .finish()
    }
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

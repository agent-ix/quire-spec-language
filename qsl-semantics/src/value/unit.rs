// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 dimension, unit and compound-unit preimages: the compile-side half
//! of the unit graph. The runtime half (`Dimension`, `Unit`, `UnitEdge`,
//! `UnitGraph`'s topology, `CompoundUnit`) is `quire_semantic_value::unit`
//! (ADR-011 §6.1 layer SV).
//!
//! Base dimensions and units are I04 nominal nodes keyed by the structural
//! [`NodeKey`] the check stage carries for each declaration; nothing here
//! hashes or recomputes a key. This module reads the preimage documents and
//! admits the nodes ([`admit_unit_graph`]): semantic well-formedness, graph
//! topology and the owner join. A node id spelled in a document is a
//! checked-package node id read into a [`NodeKey`].
//!
//! A compound-unit preimage's kernel [`UnitId`] ([`CompoundUnitPreimage::id`])
//! is `quire_semantic_value::unit::compound_unit_id` of its spelled terms.

use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

use super::semantic_node::{
    is_qualified_name, refuse, NodeIdDocument, NodeOwner, NominalRefusal, OwnerSelection,
    RationalDocument,
};
use quire_exact::{Integer, InvalidNodeKey, NodeKey, Rational, UnitId};
use quire_semantic_value::semantic_node::{InvalidSemanticGraph, SemanticGraphCause};
use quire_semantic_value::unit::{
    CompoundUnitCause, DimensionNode, InvalidCompoundUnit, NominalDeclaration, UnitGraph, UnitNode,
};

const DIMENSION_VERSION: &str = "quire.dimension-node/v1";
const UNIT_VERSION: &str = "quire.unit-node/v1";
const COMPOUND_UNIT_VERSION: &str = "quire.value.compound-unit/v1";

// ---- dimensions --------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DimensionTermDocument {
    dimension_node_id: NodeIdDocument,
    exponent: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct DimensionDocument {
    version: String,
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    terms: Vec<DimensionTermDocument>,
}

/// Read schema terms: canonical node ids and integer spellings, in order.
fn read_terms<'a>(
    terms: impl IntoIterator<Item = (&'a NodeIdDocument, &'a str)>,
) -> Option<Vec<(NodeKey, Integer)>> {
    terms
        .into_iter()
        .map(|(id, exponent)| Some((id.key()?, exponent.parse().ok()?)))
        .collect()
}

/// A `quire.dimension-node/v1` preimage that satisfies the preimage schema.
/// Empty terms declare a base dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionPreimage {
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    terms: Vec<(NodeKey, Integer)>,
}

impl DimensionPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    #[ix_trace_rs::string_edge]
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let non_canonical = refuse(SemanticGraphCause::NonCanonicalPreimage);
        let document: DimensionDocument =
            serde_json::from_value(value).map_err(|_| non_canonical)?;
        let terms = read_terms(
            document
                .terms
                .iter()
                .map(|term| (&term.dimension_node_id, term.exponent.as_str())),
        )
        .filter(|_| {
            document.version == DIMENSION_VERSION
                && document.owner.is_well_formed()
                && is_qualified_name(&document.qualified_declaration)
        })
        .ok_or(non_canonical)?;
        Ok(Self {
            owner: document.owner,
            qualified_declaration: document.qualified_declaration,
            terms,
        })
    }

    /// Build a preimage from its parts, applying the schema checks
    /// [`Self::from_json`] applies (`NonCanonicalPreimage`). `terms` name
    /// base dimensions by node key, in the order the caller keys them:
    /// ascending by key (checked at [`admit_unit_graph`]). Empty terms
    /// declare a base dimension.
    pub fn new(
        owner: NodeOwner,
        qualified_declaration: Vec<String>,
        terms: Vec<(NodeKey, Integer)>,
    ) -> Result<Self, InvalidSemanticGraph> {
        if !owner.is_well_formed() || !is_qualified_name(&qualified_declaration) {
            return Err(refuse(SemanticGraphCause::NonCanonicalPreimage));
        }
        Ok(Self {
            owner,
            qualified_declaration,
            terms,
        })
    }

    /// The owner projection.
    pub fn owner(&self) -> &NodeOwner {
        &self.owner
    }

    /// Qualified declaration name segments.
    pub fn qualified_declaration(&self) -> &[String] {
        &self.qualified_declaration
    }

    /// Retained `(base dimension, exponent)` terms, as spelled.
    pub fn terms(&self) -> &[(NodeKey, Integer)] {
        &self.terms
    }

    /// Whether this declares a base dimension.
    pub fn is_base(&self) -> bool {
        self.terms.is_empty()
    }
}

impl DimensionPreimage {
    /// The dimension's structural node key: its owner and qualified name.
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey`] when the key cannot be built.
    pub fn key(&self) -> Result<NodeKey, InvalidNodeKey> {
        self.owner.declaration_key(&self.qualified_declaration)
    }

    /// The dimension's qualified name, which lowering builds its node from
    /// (FR-094).
    fn nominal_declaration(&self) -> NominalDeclaration {
        NominalDeclaration {
            qualified_declaration: self.qualified_declaration.clone(),
            ..NominalDeclaration::default()
        }
    }
}

// ---- dimension equivalences ---------------------------------------------------

/// A dimension equivalence (FR-091 "Dimension keys", QSpec FR-142 "Dimension
/// equivalence"): the two related dimension nodes in ascending structural key
/// order and the root ratio stated for the node listed first, so a pair
/// written in either order is one equivalence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionEquivalencePreimage {
    owner: NodeOwner,
    dimensions: [NodeKey; 2],
    root_scale: Rational,
}

impl DimensionEquivalencePreimage {
    /// The preimage of `first ~ second` whose canonical root of `first` is
    /// `root_ratio` canonical roots of `second`. The nodes are put in
    /// ascending key order, the ratio inverted when that swaps them.
    /// `None` for two equal nodes or a ratio that is not positive, which
    /// the assembler refuses first.
    pub fn new(
        owner: NodeOwner,
        first: NodeKey,
        second: NodeKey,
        root_ratio: &Rational,
    ) -> Option<Self> {
        if first == second
            || !owner.is_well_formed()
            || root_ratio.numerator().is_negative()
            || root_ratio.is_zero()
        {
            return None;
        }
        let (dimensions, root_scale) = if first < second {
            ([first, second], root_ratio.clone())
        } else {
            (
                [second, first],
                Rational::from_integer(Integer::one()).div(root_ratio)?,
            )
        };
        Some(Self {
            owner,
            dimensions,
            root_scale,
        })
    }

    /// The two related dimension nodes, in ascending key order.
    pub fn dimensions(&self) -> [NodeKey; 2] {
        self.dimensions.clone()
    }

    /// One canonical root of the first node in canonical roots of the
    /// second.
    pub fn root_scale(&self) -> &Rational {
        &self.root_scale
    }
}

// ---- units -------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UnitDocument {
    version: String,
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    dimension_node_id: NodeIdDocument,
    // A required member whose value may be `null`: `Option` would also accept
    // an absent member, which the schema refuses.
    target_unit_node_id: serde_json::Value,
    scale: RationalDocument,
    offset: RationalDocument,
}

/// A spelled rational: exact numerator and positive denominator.
type SpelledRational = (Integer, Integer);

/// A `quire.unit-node/v1` preimage that satisfies the preimage schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitPreimage {
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    dimension: NodeKey,
    target: Option<NodeKey>,
    scale: SpelledRational,
    offset: SpelledRational,
}

impl UnitPreimage {
    /// Build a preimage from its parts, applying the schema checks
    /// [`Self::from_json`] applies (`NonCanonicalPreimage`). `scale` and
    /// `offset` are `Rational`s, so they are reduced with a positive
    /// denominator; `target` is `None` for a canonical root.
    pub fn new(
        owner: NodeOwner,
        qualified_declaration: Vec<String>,
        dimension: NodeKey,
        target: Option<NodeKey>,
        scale: &Rational,
        offset: &Rational,
    ) -> Result<Self, InvalidSemanticGraph> {
        if !owner.is_well_formed() || !is_qualified_name(&qualified_declaration) {
            return Err(refuse(SemanticGraphCause::NonCanonicalPreimage));
        }
        Ok(Self {
            owner,
            qualified_declaration,
            dimension,
            target,
            scale: (scale.numerator().clone(), scale.denominator().clone()),
            offset: (offset.numerator().clone(), offset.denominator().clone()),
        })
    }

    /// Read a preimage object; any schema violation is non-canonical.
    #[ix_trace_rs::string_edge]
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let non_canonical = refuse(SemanticGraphCause::NonCanonicalPreimage);
        let document: UnitDocument = serde_json::from_value(value).map_err(|_| non_canonical)?;
        let target = match document.target_unit_node_id {
            serde_json::Value::Null => None,
            reference => Some(
                serde_json::from_value::<NodeIdDocument>(reference)
                    .ok()
                    .and_then(|id| id.key())
                    .ok_or(non_canonical)?,
            ),
        };
        let well_formed = document.version == UNIT_VERSION
            && document.owner.is_well_formed()
            && is_qualified_name(&document.qualified_declaration);
        match (
            document.dimension_node_id.key(),
            document.scale.parts(),
            document.offset.parts(),
        ) {
            (Some(dimension), Some(scale), Some(offset)) if well_formed => Ok(Self {
                owner: document.owner,
                qualified_declaration: document.qualified_declaration,
                dimension,
                target,
                scale,
                offset,
            }),
            _ => Err(non_canonical),
        }
    }

    /// The owner projection.
    pub fn owner(&self) -> &NodeOwner {
        &self.owner
    }

    /// Qualified declaration name segments.
    pub fn qualified_declaration(&self) -> &[String] {
        &self.qualified_declaration
    }

    /// The unit's dimension node id, as read.
    pub fn dimension(&self) -> &NodeKey {
        &self.dimension
    }

    /// The target unit node id as read, or `None` for a canonical root.
    pub fn target(&self) -> Option<&NodeKey> {
        self.target.as_ref()
    }

    /// The edge's scale as spelled: `(numerator, denominator)`.
    pub fn scale(&self) -> (&Integer, &Integer) {
        (&self.scale.0, &self.scale.1)
    }

    /// The edge's offset as spelled: `(numerator, denominator)`.
    pub fn offset(&self) -> (&Integer, &Integer) {
        (&self.offset.0, &self.offset.1)
    }

    /// The unit's qualified name, which lowering builds its node from
    /// (FR-094).
    fn nominal_declaration(&self) -> NominalDeclaration {
        NominalDeclaration {
            qualified_declaration: self.qualified_declaration.clone(),
            ..NominalDeclaration::default()
        }
    }
}

impl UnitPreimage {
    /// The unit's structural node key: its owner and qualified name.
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey`] when the key cannot be built.
    pub fn key(&self) -> Result<NodeKey, InvalidNodeKey> {
        self.owner.declaration_key(&self.qualified_declaration)
    }

    /// Refuse an unreduced rational, then a zero scale or a non-identity root
    /// ([`UnitNode::checked`]). The node keeps `declaration`.
    fn check_semantics(
        &self,
        declaration: NominalDeclaration,
    ) -> Result<UnitNode, SemanticGraphCause> {
        let reduced = |(numerator, denominator): &SpelledRational| {
            Rational::new(numerator.clone(), denominator.clone())
                .ok()
                .filter(|value| {
                    value.numerator() == numerator && value.denominator() == denominator
                })
                .ok_or(SemanticGraphCause::UnreducedRational)
        };
        let (scale, offset) = (reduced(&self.scale)?, reduced(&self.offset)?);
        UnitNode::checked(
            self.dimension.clone(),
            self.target.clone(),
            scale,
            offset,
            declaration,
        )
    }
}

// ---- admission ---------------------------------------------------------------

/// Admit dimension and unit nodes keyed by the paired structural keys.
///
/// Each node is first checked for semantic well-formedness and the node set
/// for repeated keys. The graph, referenced by key, is then checked for
/// unknown or derived dimension terms, unknown dimensions, unknown and
/// cross-dimension targets, per-dimension root count and target cycles
/// ([`UnitGraph::from_checked_nodes`]). Finally every owner must join the
/// selection. Every refusal is `invalid_semantic_graph`; the typed cause names
/// the first failed check.
pub fn admit_unit_graph(
    dimensions: impl IntoIterator<Item = (DimensionPreimage, NodeKey)>,
    units: impl IntoIterator<Item = (UnitPreimage, NodeKey)>,
    owners: &OwnerSelection,
) -> Result<UnitGraph, NominalRefusal> {
    let mut owners_seen = Vec::new();
    let mut keys = BTreeSet::new();
    let mut admitted_dimensions = BTreeMap::new();
    for (preimage, key) in dimensions {
        let node = DimensionNode::checked(preimage.terms.clone(), preimage.nominal_declaration())
            .map_err(refuse)?;
        if !keys.insert(key.clone()) {
            return Err(refuse(SemanticGraphCause::DuplicateNode).into());
        }
        owners_seen.push(preimage.owner);
        admitted_dimensions.insert(key, node);
    }
    let mut admitted_units = BTreeMap::new();
    for (preimage, key) in units {
        let node = preimage
            .check_semantics(preimage.nominal_declaration())
            .map_err(refuse)?;
        if !keys.insert(key.clone()) {
            return Err(refuse(SemanticGraphCause::DuplicateNode).into());
        }
        owners_seen.push(preimage.owner);
        admitted_units.insert(key, node);
    }
    let graph = UnitGraph::from_checked_nodes(&admitted_dimensions, &admitted_units)?;
    if owners_seen.iter().any(|owner| !owners.contains(owner)) {
        return Err(refuse(SemanticGraphCause::OwnerNotSelected).into());
    }
    Ok(graph)
}

// ---- compound units ----------------------------------------------------------

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompoundTermDocument {
    unit_node_id: NodeIdDocument,
    exponent: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CompoundDocument {
    version: String,
    terms: Vec<CompoundTermDocument>,
}

/// A schema-valid `quire.value.compound-unit/v1` preimage, as spelled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompoundUnitPreimage {
    terms: Vec<(NodeKey, Integer)>,
}

impl CompoundUnitPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidCompoundUnit> {
        let non_canonical = InvalidCompoundUnit {
            cause: CompoundUnitCause::NonCanonicalPreimage,
        };
        let document: CompoundDocument =
            serde_json::from_value(value).map_err(|_| non_canonical)?;
        let terms = read_terms(
            document
                .terms
                .iter()
                .map(|term| (&term.unit_node_id, term.exponent.as_str())),
        )
        .filter(|_| document.version == COMPOUND_UNIT_VERSION)
        .ok_or(non_canonical)?;
        Ok(Self { terms })
    }

    /// The spelled `(unit node id, exponent)` terms, for
    /// [`UnitGraph::compound_unit`].
    pub fn terms(&self) -> &[(NodeKey, Integer)] {
        &self.terms
    }

    /// The compound-arm [`UnitId`] of exactly these spelled terms
    /// (`UnitId::compound`), or `None` when a term key cannot be cloned.
    pub fn id(&self) -> Option<UnitId> {
        let terms = self
            .terms
            .iter()
            .map(|(id, exponent)| Some((id.try_clone().ok()?, exponent.clone())))
            .collect::<Option<Vec<_>>>()?;
        Some(UnitId::compound(terms))
    }
}

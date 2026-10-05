// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 dimension, unit and compound-unit preimages: the compile-side half
//! of the unit graph. The runtime half (`Dimension`, `Unit`, `UnitEdge`,
//! `UnitGraph`'s topology, `CompoundUnit`) is `quire_semantic_value::unit`
//! (ADR-011 §6.1 layer SV).
//!
//! Base dimensions and units are I04 nominal nodes whose keys hash
//! `quire.dimension-node/v1` and `quire.unit-node/v1` preimages. This module
//! reads those preimages, computes their digests ([`NodeIdentityPreimage`])
//! and compares each retained key's bytes with them at admission
//! ([`admit_unit_graph`]). It never constructs a `NodeKey`: only `check`
//! mints one (ADR-011 §6.1, ADR-013 O-04). A node id read from a preimage
//! document stays a [`WireNodeId`] until `quire_semantic_value`'s unit graph
//! resolves its bytes by lookup among the admitted nodes' retained keys.
//!
//! A compound-unit preimage's kernel [`UnitId`] ([`CompoundUnitPreimage::id`])
//! is `quire_semantic_value::unit::compound_unit_id` of its spelled terms,
//! the same digest the runtime `CompoundUnit::id` computes.

use std::collections::{BTreeMap, BTreeSet};

use quire_canonical::FixedShape;
use serde::{Deserialize, Serialize};

use super::semantic_node::{
    is_qualified_name, preimage_bytes, preimage_digest, refuse, retains, CanonicalOwner,
    CanonicalRational, NodeIdDocument, NodeIdentityPreimage, NodeOwner, NominalRefusal,
    OwnerSelection, RationalDocument,
};
use qsl_foundation::digest::WireNodeId;
use quire_exact::{Integer, NodeKey, Rational, UnitId, COMPOUND_UNIT_DOMAIN};
use quire_semantic_value::semantic_node::{
    CanonicalNodeId, IdentityRefusal, InvalidSemanticGraph, SemanticGraphCause, IDENTITY_LIMITS,
};
use quire_semantic_value::unit::{
    compound_unit_id, CompoundUnitCause, DimensionNode, InvalidCompoundUnit, NominalDeclaration,
    UnitGraph, UnitNode,
};

const DIMENSION_VERSION: &str = "quire.dimension-node/v1";
const UNIT_VERSION: &str = "quire.unit-node/v1";

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

// JCS preimages: fields are declared in ascending key order (see `semantic_node`).
#[derive(Serialize, FixedShape)]
struct CanonicalDimensionTerm {
    dimension_node_id: CanonicalNodeId,
    exponent: String,
}

#[derive(Serialize, FixedShape)]
struct CanonicalDimension<'a> {
    owner: CanonicalOwner<'a>,
    qualified_declaration: &'a [String],
    terms: Vec<CanonicalDimensionTerm>,
    version: &'static str,
}

/// Read schema terms: canonical node ids and integer spellings, in order.
fn read_terms<'a>(
    terms: impl IntoIterator<Item = (&'a NodeIdDocument, &'a str)>,
) -> Option<Vec<(WireNodeId, Integer)>> {
    terms
        .into_iter()
        .map(|(id, exponent)| Some((id.wire_id()?, exponent.parse().ok()?)))
        .collect()
}

/// A `quire.dimension-node/v1` preimage that satisfies the preimage schema.
/// Empty terms declare a base dimension.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionPreimage {
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    terms: Vec<(WireNodeId, Integer)>,
}

impl DimensionPreimage {
    /// Read a preimage object; any schema violation is non-canonical.
    #[qsl_attrs::string_edge]
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
            terms: terms
                .into_iter()
                .map(|(key, exponent)| (WireNodeId::from_digest(*key.as_bytes()), exponent))
                .collect(),
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
    pub fn terms(&self) -> &[(WireNodeId, Integer)] {
        &self.terms
    }

    /// Whether this declares a base dimension.
    pub fn is_base(&self) -> bool {
        self.terms.is_empty()
    }
}

impl DimensionPreimage {
    fn canonical(&self) -> CanonicalDimension<'_> {
        CanonicalDimension {
            owner: self.owner.canonical(),
            qualified_declaration: &self.qualified_declaration,
            terms: self
                .terms
                .iter()
                .map(|(id, exponent)| CanonicalDimensionTerm {
                    dimension_node_id: CanonicalNodeId::from(*id.as_bytes()),
                    exponent: exponent.to_string(),
                })
                .collect(),
            version: DIMENSION_VERSION,
        }
    }

    /// The RFC 8785 bytes whose SHA-256 is the dimension's node key.
    pub(crate) fn preimage_bytes(&self) -> Result<Vec<u8>, NominalRefusal> {
        preimage_bytes(&self.canonical())
    }

    /// The dimension's qualified name and preimage bytes, which lowering
    /// builds its node from (FR-094).
    fn nominal_declaration(&self) -> Result<NominalDeclaration, NominalRefusal> {
        Ok(NominalDeclaration {
            qualified_declaration: self.qualified_declaration.clone(),
            preimage: self.preimage_bytes()?,
        })
    }
}

impl NodeIdentityPreimage for DimensionPreimage {
    fn digest(&self) -> Result<[u8; 32], NominalRefusal> {
        preimage_digest(&self.canonical())
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

#[derive(Serialize, FixedShape)]
struct CanonicalUnit<'a> {
    dimension_node_id: CanonicalNodeId,
    offset: CanonicalRational,
    owner: CanonicalOwner<'a>,
    qualified_declaration: &'a [String],
    scale: CanonicalRational,
    target_unit_node_id: Option<CanonicalNodeId>,
    version: &'static str,
}

/// A spelled rational: exact numerator and positive denominator.
type SpelledRational = (Integer, Integer);

/// A `quire.unit-node/v1` preimage that satisfies the preimage schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitPreimage {
    owner: NodeOwner,
    qualified_declaration: Vec<String>,
    dimension: WireNodeId,
    target: Option<WireNodeId>,
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
        let wire = |key: NodeKey| WireNodeId::from_digest(*key.as_bytes());
        Ok(Self {
            owner,
            qualified_declaration,
            dimension: wire(dimension),
            target: target.map(wire),
            scale: (scale.numerator().clone(), scale.denominator().clone()),
            offset: (offset.numerator().clone(), offset.denominator().clone()),
        })
    }

    /// Read a preimage object; any schema violation is non-canonical.
    #[qsl_attrs::string_edge]
    pub fn from_json(value: serde_json::Value) -> Result<Self, InvalidSemanticGraph> {
        let non_canonical = refuse(SemanticGraphCause::NonCanonicalPreimage);
        let document: UnitDocument = serde_json::from_value(value).map_err(|_| non_canonical)?;
        let target = match document.target_unit_node_id {
            serde_json::Value::Null => None,
            reference => Some(
                serde_json::from_value::<NodeIdDocument>(reference)
                    .ok()
                    .and_then(|id| id.wire_id())
                    .ok_or(non_canonical)?,
            ),
        };
        let well_formed = document.version == UNIT_VERSION
            && document.owner.is_well_formed()
            && is_qualified_name(&document.qualified_declaration);
        match (
            document.dimension_node_id.wire_id(),
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
    pub fn dimension(&self) -> WireNodeId {
        self.dimension
    }

    /// The target unit node id as read, or `None` for a canonical root.
    pub fn target(&self) -> Option<WireNodeId> {
        self.target
    }

    fn canonical(&self) -> CanonicalUnit<'_> {
        CanonicalUnit {
            dimension_node_id: CanonicalNodeId::from(*self.dimension.as_bytes()),
            offset: CanonicalRational::new(&self.offset.0, &self.offset.1),
            owner: self.owner.canonical(),
            qualified_declaration: &self.qualified_declaration,
            scale: CanonicalRational::new(&self.scale.0, &self.scale.1),
            target_unit_node_id: self.target.map(|id| CanonicalNodeId::from(*id.as_bytes())),
            version: UNIT_VERSION,
        }
    }

    /// The edge's scale as spelled: `(numerator, denominator)`.
    pub fn scale(&self) -> (&Integer, &Integer) {
        (&self.scale.0, &self.scale.1)
    }

    /// The edge's offset as spelled: `(numerator, denominator)`.
    pub fn offset(&self) -> (&Integer, &Integer) {
        (&self.offset.0, &self.offset.1)
    }

    /// The RFC 8785 bytes whose SHA-256 is the unit's node key.
    pub(crate) fn preimage_bytes(&self) -> Result<Vec<u8>, NominalRefusal> {
        preimage_bytes(&self.canonical())
    }

    /// The unit's qualified name and preimage bytes, which lowering builds
    /// its node from (FR-094).
    fn nominal_declaration(&self) -> Result<NominalDeclaration, NominalRefusal> {
        Ok(NominalDeclaration {
            qualified_declaration: self.qualified_declaration.clone(),
            preimage: self.preimage_bytes()?,
        })
    }
}

impl NodeIdentityPreimage for UnitPreimage {
    fn digest(&self) -> Result<[u8; 32], NominalRefusal> {
        preimage_digest(&self.canonical())
    }
}

impl UnitPreimage {
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
            *self.dimension.as_bytes(),
            self.target.map(|target| *target.as_bytes()),
            scale,
            offset,
            declaration,
        )
    }
}

// ---- admission ---------------------------------------------------------------

/// The owner of a node and whether its retained key is the one its content
/// determines, checked after topology.
struct Provenance {
    owner: NodeOwner,
    key_matches: bool,
}

/// Admit dimension and unit nodes whose graph retains the paired keys.
///
/// Each node is first checked for semantic well-formedness and the node set
/// for repeated keys. The graph, referenced by retained keys, is then checked
/// for unknown or derived dimension terms, unknown dimensions, unknown and
/// cross-dimension targets, per-dimension root count and target cycles
/// ([`UnitGraph::from_checked_nodes`]). Finally every owner must join the
/// selection and every retained key must equal its recomputed key. Every
/// refusal is `invalid_semantic_graph`; the typed cause names the first
/// failed check.
pub fn admit_unit_graph(
    dimensions: impl IntoIterator<Item = (DimensionPreimage, NodeKey)>,
    units: impl IntoIterator<Item = (UnitPreimage, NodeKey)>,
    owners: &OwnerSelection,
) -> Result<UnitGraph, NominalRefusal> {
    let mut provenance = Vec::new();
    let mut keys = BTreeSet::new();
    let mut admitted_dimensions = BTreeMap::new();
    for (preimage, key) in dimensions {
        let node = DimensionNode::checked(
            preimage
                .terms
                .iter()
                .map(|(id, exponent)| (*id.as_bytes(), exponent.clone()))
                .collect(),
            preimage.nominal_declaration()?,
        )
        .map_err(refuse)?;
        if !keys.insert(key) {
            return Err(refuse(SemanticGraphCause::DuplicateNode).into());
        }
        provenance.push(Provenance {
            key_matches: retains(key, &preimage)?,
            owner: preimage.owner,
        });
        admitted_dimensions.insert(key, node);
    }
    let mut admitted_units = BTreeMap::new();
    for (preimage, key) in units {
        let node = preimage
            .check_semantics(preimage.nominal_declaration()?)
            .map_err(refuse)?;
        if !keys.insert(key) {
            return Err(refuse(SemanticGraphCause::DuplicateNode).into());
        }
        provenance.push(Provenance {
            key_matches: retains(key, &preimage)?,
            owner: preimage.owner,
        });
        admitted_units.insert(key, node);
    }
    let graph = UnitGraph::from_checked_nodes(&admitted_dimensions, &admitted_units)?;
    if provenance.iter().any(|node| !owners.contains(&node.owner)) {
        return Err(refuse(SemanticGraphCause::OwnerNotSelected).into());
    }
    if provenance.iter().any(|node| !node.key_matches) {
        return Err(refuse(SemanticGraphCause::StaleKey).into());
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
    terms: Vec<([u8; 32], Integer)>,
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
        .filter(|_| document.version == COMPOUND_UNIT_DOMAIN)
        .ok_or(non_canonical)?;
        Ok(Self {
            terms: terms
                .into_iter()
                .map(|(id, exponent)| (*id.as_bytes(), exponent))
                .collect(),
        })
    }

    /// The spelled `(unit node id, exponent)` terms, each id as its 32 digest
    /// bytes, for [`UnitGraph::compound_unit`].
    pub fn terms(&self) -> &[([u8; 32], Integer)] {
        &self.terms
    }

    /// The compound-arm [`UnitId`] of exactly these spelled terms, under
    /// [`IDENTITY_LIMITS`].
    ///
    /// # Errors
    ///
    /// As [`compound_unit_id`]: the preimage's bytes reached the identity
    /// byte limit, or a heap reservation failed.
    pub fn id(&self) -> Result<UnitId, IdentityRefusal> {
        compound_unit_id(
            self.terms.iter().map(|(id, exponent)| (*id, exponent)),
            IDENTITY_LIMITS,
        )
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 dimensions, declared units, their admitted unit graph and the
//! evaluator-owned compound-unit value identity.
//!
//! Base dimensions and units are I04 nominal nodes whose keys hash
//! `quire.dimension-node/v1` and `quire.unit-node/v1` preimages. A dimension is
//! a sorted map from base-dimension key to a nonzero mathematical exponent.
//! The units of one dimension node form an acyclic graph with exactly one
//! targetless canonical root; each edge maps
//! `target_value = scale × source_value + offset` exactly.
//!
//! This module computes the preimage digests ([`NodeIdentityPreimage`]) and
//! compares each retained key's bytes with them at admission. It never
//! constructs a `NodeKey`: only `check` mints one (ADR-011 §6.1, ADR-013
//! O-04). A node id read from a preimage document stays a
//! [`WireNodeId`] until `UnitGraph::admit` or `UnitGraph::compound_unit`
//! resolves it by lookup among the admitted nodes' retained keys.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::semantic_node::{
    check_terms, is_qualified_name, preimage_digest, refuse, resolve, retains, wire_index,
    CanonicalNodeId, CanonicalOwner, CanonicalRational, InvalidSemanticGraph, NodeIdDocument,
    NodeIdentityPreimage, NodeOwner, OwnerSelection, RationalDocument, SemanticGraphCause,
};
use qsl_foundation::digest::WireNodeId;
use quire_exact::{Integer, NodeKey, Rational};

const DIMENSION_VERSION: &str = "quire.dimension-node/v1";
const UNIT_VERSION: &str = "quire.unit-node/v1";

/// Domain and preimage version of a compound-unit value identity.
pub const COMPOUND_UNIT_DOMAIN: &str = "quire.value.compound-unit/v1";

// ---- dimensions --------------------------------------------------------------

/// A normalized dimension: base-dimension node keys to nonzero exponents.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Dimension(BTreeMap<NodeKey, Integer>);

impl Dimension {
    /// The empty (dimensionless) map.
    pub fn dimensionless() -> Self {
        Self::default()
    }

    /// Whether every exponent is absent.
    pub fn is_dimensionless(&self) -> bool {
        self.0.is_empty()
    }

    /// Ascending `(base dimension, exponent)` terms; no exponent is zero.
    pub fn exponents(&self) -> impl Iterator<Item = (NodeKey, &Integer)> {
        self.0.iter().map(|(key, exponent)| (*key, exponent))
    }

    /// Add exponents.
    pub fn multiply(&self, other: &Self) -> Self {
        Self(combine(&self.0, &other.0, Integer::add))
    }

    /// Subtract exponents.
    pub fn divide(&self, other: &Self) -> Self {
        Self(combine(&self.0, &other.0, Integer::sub))
    }

    /// Multiply every exponent by `exponent`.
    pub fn power(&self, exponent: &Integer) -> Self {
        Self(scale_exponents(&self.0, exponent))
    }
}

/// Combine two exponent maps key by key and drop zero exponents.
fn combine(
    left: &BTreeMap<NodeKey, Integer>,
    right: &BTreeMap<NodeKey, Integer>,
    operation: fn(&Integer, &Integer) -> Integer,
) -> BTreeMap<NodeKey, Integer> {
    let keys: BTreeSet<_> = left.keys().chain(right.keys()).copied().collect();
    let zero = Integer::zero();
    keys.into_iter()
        .map(|key| {
            let exponent = operation(
                left.get(&key).unwrap_or(&zero),
                right.get(&key).unwrap_or(&zero),
            );
            (key, exponent)
        })
        .filter(|(_, exponent)| !exponent.is_zero())
        .collect()
}

fn scale_exponents(
    terms: &BTreeMap<NodeKey, Integer>,
    exponent: &Integer,
) -> BTreeMap<NodeKey, Integer> {
    terms
        .iter()
        .map(|(key, value)| (*key, value.mul(exponent)))
        .filter(|(_, value)| !value.is_zero())
        .collect()
}

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
#[derive(Serialize)]
struct CanonicalDimensionTerm {
    dimension_node_id: CanonicalNodeId,
    exponent: String,
}

#[derive(Serialize)]
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

impl NodeIdentityPreimage for DimensionPreimage {
    fn digest(&self) -> Result<[u8; 32], InvalidSemanticGraph> {
        preimage_digest(&CanonicalDimension {
            owner: self.owner.canonical(),
            qualified_declaration: &self.qualified_declaration,
            terms: self
                .terms
                .iter()
                .map(|(id, exponent)| CanonicalDimensionTerm {
                    dimension_node_id: (*id).into(),
                    exponent: exponent.to_string(),
                })
                .collect(),
            version: DIMENSION_VERSION,
        })
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

#[derive(Serialize)]
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
    /// Read a preimage object; any schema violation is non-canonical.
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
}

impl NodeIdentityPreimage for UnitPreimage {
    fn digest(&self) -> Result<[u8; 32], InvalidSemanticGraph> {
        preimage_digest(&CanonicalUnit {
            dimension_node_id: self.dimension.into(),
            offset: CanonicalRational::new(&self.offset.0, &self.offset.1),
            owner: self.owner.canonical(),
            qualified_declaration: &self.qualified_declaration,
            scale: CanonicalRational::new(&self.scale.0, &self.scale.1),
            target_unit_node_id: self.target.map(Into::into),
            version: UNIT_VERSION,
        })
    }
}

impl UnitPreimage {
    /// Refuse an unreduced rational, a zero scale, then a non-identity root.
    fn check_semantics(&self) -> Result<(Rational, Rational), SemanticGraphCause> {
        let reduced = |(numerator, denominator): &SpelledRational| {
            Rational::new(numerator.clone(), denominator.clone())
                .ok()
                .filter(|value| {
                    value.numerator() == numerator && value.denominator() == denominator
                })
                .ok_or(SemanticGraphCause::UnreducedRational)
        };
        let (scale, offset) = (reduced(&self.scale)?, reduced(&self.offset)?);
        if scale.is_zero() {
            return Err(SemanticGraphCause::ZeroScale);
        }
        let identity = scale == Rational::from_integer(Integer::one()) && offset.is_zero();
        if self.target.is_none() && !identity {
            return Err(SemanticGraphCause::NonIdentityRoot);
        }
        Ok((scale, offset))
    }
}

/// One exact affine edge `target = scale × source + offset`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UnitEdge {
    scale: Rational,
    offset: Rational,
}

impl UnitEdge {
    /// Nonzero exact scale.
    pub fn scale(&self) -> &Rational {
        &self.scale
    }

    /// Exact offset.
    pub fn offset(&self) -> &Rational {
        &self.offset
    }
}

/// An admitted declared unit and its exact path to the canonical root.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Unit {
    key: NodeKey,
    dimension_node: NodeKey,
    dimension: Dimension,
    root: NodeKey,
    path: Vec<UnitEdge>,
    canonical: UnitEdge,
}

impl Unit {
    /// The unit node key.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The unit's dimension node key.
    pub fn dimension_node(&self) -> NodeKey {
        self.dimension_node
    }

    /// The normalized dimension map.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// The canonical root unit of the unit's dimension node.
    pub fn root(&self) -> NodeKey {
        self.root
    }

    /// Edges from this unit to the root, in source-to-root order.
    pub fn path(&self) -> &[UnitEdge] {
        &self.path
    }

    /// The composed exact mapping to the canonical root.
    pub fn canonical(&self) -> &UnitEdge {
        &self.canonical
    }

    /// Whether this unit is affine: a nonzero-offset point unit that admits
    /// only conversion and comparison. The composed canonical offset
    /// decides, so a zero-offset unit that targets an affine unit is affine.
    pub fn is_affine(&self) -> bool {
        !self.canonical.offset.is_zero()
    }
}

/// An admitted closed set of dimension and unit nodes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnitGraph {
    dimensions: BTreeMap<NodeKey, Dimension>,
    units: BTreeMap<NodeKey, Unit>,
    /// Resolves a wire unit node id to an admitted unit's key.
    unit_ids: BTreeMap<WireNodeId, NodeKey>,
}

/// A dimension node after its per-node semantic checks.
struct AdmittedDimension {
    terms: Vec<(WireNodeId, Integer)>,
}

/// A unit node after its per-node semantic checks, as read.
struct AdmittedUnit {
    dimension: WireNodeId,
    target: Option<WireNodeId>,
    edge: UnitEdge,
}

/// A unit node whose dimension and target resolved to admitted keys.
struct ResolvedUnit {
    dimension: NodeKey,
    target: Option<NodeKey>,
    edge: UnitEdge,
}

/// The owner of a node and whether its retained key is the one its content
/// determines, checked after topology.
struct Provenance {
    owner: NodeOwner,
    key_matches: bool,
}

impl UnitGraph {
    /// Admit dimension and unit nodes whose graph retains the paired keys.
    ///
    /// Each node is first checked for semantic well-formedness and the node
    /// set for repeated keys. The graph, referenced by retained keys, is then
    /// checked for unknown or derived dimension terms, unknown dimensions,
    /// unknown and cross-dimension targets, per-dimension root count and
    /// target cycles. Finally every owner must join the selection and every
    /// retained key must equal its recomputed key. Every refusal is
    /// `invalid_semantic_graph`; the typed cause names the first failed check.
    pub fn admit(
        dimensions: impl IntoIterator<Item = (DimensionPreimage, NodeKey)>,
        units: impl IntoIterator<Item = (UnitPreimage, NodeKey)>,
        owners: &OwnerSelection,
    ) -> Result<Self, InvalidSemanticGraph> {
        let mut provenance = Vec::new();
        let mut keys = BTreeSet::new();
        let mut admitted_dimensions = BTreeMap::new();
        for (preimage, key) in dimensions {
            check_terms(&preimage.terms).map_err(refuse)?;
            if !keys.insert(key) {
                return Err(refuse(SemanticGraphCause::DuplicateNode));
            }
            provenance.push(Provenance {
                key_matches: retains(key, &preimage)?,
                owner: preimage.owner,
            });
            admitted_dimensions.insert(
                key,
                AdmittedDimension {
                    terms: preimage.terms,
                },
            );
        }
        let mut admitted_units = BTreeMap::new();
        for (preimage, key) in units {
            let (scale, offset) = preimage.check_semantics().map_err(refuse)?;
            if !keys.insert(key) {
                return Err(refuse(SemanticGraphCause::DuplicateNode));
            }
            provenance.push(Provenance {
                key_matches: retains(key, &preimage)?,
                owner: preimage.owner,
            });
            admitted_units.insert(
                key,
                AdmittedUnit {
                    dimension: preimage.dimension,
                    target: preimage.target,
                    edge: UnitEdge { scale, offset },
                },
            );
        }
        let dimension_ids = wire_index(admitted_dimensions.keys().copied());
        let dimensions = dimension_maps(&admitted_dimensions, &dimension_ids)?;
        let unit_ids = wire_index(admitted_units.keys().copied());
        let units = unit_paths(
            &resolve_units(&admitted_units, &dimension_ids, &unit_ids)?,
            &dimensions,
        )?;
        if provenance.iter().any(|node| !owners.contains(&node.owner)) {
            return Err(refuse(SemanticGraphCause::OwnerNotSelected));
        }
        if provenance.iter().any(|node| !node.key_matches) {
            return Err(refuse(SemanticGraphCause::StaleKey));
        }
        Ok(Self {
            dimensions,
            units,
            unit_ids,
        })
    }

    /// The normalized map of an admitted dimension node.
    pub fn dimension(&self, key: NodeKey) -> Option<&Dimension> {
        self.dimensions.get(&key)
    }

    /// An admitted unit.
    pub fn unit(&self, key: NodeKey) -> Option<&Unit> {
        self.units.get(&key)
    }

    /// Construct a compound unit from a schema-valid preimage: every term must
    /// name an admitted canonical root unit with a nonzero exponent, strictly
    /// ascending by key.
    pub fn compound_unit(
        &self,
        preimage: &CompoundUnitPreimage,
    ) -> Result<CompoundUnit, InvalidCompoundUnit> {
        check_terms(&preimage.terms).map_err(|cause| InvalidCompoundUnit {
            cause: match cause {
                SemanticGraphCause::ZeroExponent => CompoundUnitCause::ZeroExponent,
                SemanticGraphCause::DuplicateTerm => CompoundUnitCause::DuplicateTerm,
                _ => CompoundUnitCause::UnsortedTerms,
            },
        })?;
        let mut dimension = Dimension::dimensionless();
        let mut terms = BTreeMap::new();
        for (id, exponent) in &preimage.terms {
            let (key, root) = resolve(&self.unit_ids, *id)
                .and_then(|key| Some((key, self.units.get(&key)?)))
                .filter(|(key, unit)| unit.root == *key)
                .ok_or(InvalidCompoundUnit {
                    cause: CompoundUnitCause::NotRootUnit,
                })?;
            dimension = dimension.multiply(&root.dimension.power(exponent));
            terms.insert(key, exponent.clone());
        }
        Ok(CompoundUnit { terms, dimension })
    }
}

/// A base dimension maps to itself; a derived dimension to its base terms,
/// each resolved by lookup among the admitted dimension keys.
fn dimension_maps(
    dimensions: &BTreeMap<NodeKey, AdmittedDimension>,
    ids: &BTreeMap<WireNodeId, NodeKey>,
) -> Result<BTreeMap<NodeKey, Dimension>, InvalidSemanticGraph> {
    dimensions
        .iter()
        .map(|(key, node)| {
            if node.terms.is_empty() {
                return Ok((*key, Dimension([(*key, Integer::one())].into())));
            }
            let mut terms = BTreeMap::new();
            for (term, exponent) in &node.terms {
                let base =
                    resolve(ids, *term).ok_or(refuse(SemanticGraphCause::UnknownDimension))?;
                if dimensions
                    .get(&base)
                    .is_some_and(|base| !base.terms.is_empty())
                {
                    return Err(refuse(SemanticGraphCause::NonBaseDimensionTerm));
                }
                terms.insert(base, exponent.clone());
            }
            Ok((*key, Dimension(terms)))
        })
        .collect()
}

/// Resolve every unit's dimension, then every unit's target, by lookup among
/// the admitted keys.
fn resolve_units(
    units: &BTreeMap<NodeKey, AdmittedUnit>,
    dimension_ids: &BTreeMap<WireNodeId, NodeKey>,
    unit_ids: &BTreeMap<WireNodeId, NodeKey>,
) -> Result<BTreeMap<NodeKey, ResolvedUnit>, InvalidSemanticGraph> {
    let resolved_dimensions = units
        .values()
        .map(|unit| resolve(dimension_ids, unit.dimension))
        .collect::<Option<Vec<_>>>()
        .ok_or(refuse(SemanticGraphCause::UnknownDimension))?;
    units
        .iter()
        .zip(resolved_dimensions)
        .map(|((key, unit), dimension)| {
            let target = unit
                .target
                .map(|id| resolve(unit_ids, id).ok_or(refuse(SemanticGraphCause::UnknownTarget)))
                .transpose()?;
            Ok((
                *key,
                ResolvedUnit {
                    dimension,
                    target,
                    edge: unit.edge.clone(),
                },
            ))
        })
        .collect()
}

/// Check unit graph topology and compose each unit's exact path to its root.
fn unit_paths(
    units: &BTreeMap<NodeKey, ResolvedUnit>,
    dimensions: &BTreeMap<NodeKey, Dimension>,
) -> Result<BTreeMap<NodeKey, Unit>, InvalidSemanticGraph> {
    let targets = || units.values().filter_map(|unit| Some((unit, unit.target?)));
    if targets().any(|(unit, target)| {
        units
            .get(&target)
            .is_some_and(|target| target.dimension != unit.dimension)
    }) {
        return Err(refuse(SemanticGraphCause::CrossDimensionTarget));
    }
    let mut roots: BTreeMap<NodeKey, Vec<NodeKey>> = BTreeMap::new();
    for (key, unit) in units {
        let slot = roots.entry(unit.dimension).or_default();
        if unit.target.is_none() {
            slot.push(*key);
        }
    }
    if roots.values().any(Vec::is_empty) {
        return Err(refuse(SemanticGraphCause::MissingRoot));
    }
    if roots.values().any(|found| found.len() > 1) {
        return Err(refuse(SemanticGraphCause::DuplicateRoot));
    }
    units
        .iter()
        .map(|(key, unit)| {
            let mut path = Vec::new();
            let mut canonical = UnitEdge {
                scale: Rational::from_integer(Integer::one()),
                offset: Rational::from_integer(Integer::zero()),
            };
            let mut current = (*key, unit);
            while let Some(target) = current.1.target {
                if path.len() >= units.len() {
                    return Err(refuse(SemanticGraphCause::TargetCycle));
                }
                let edge = current.1.edge.clone();
                canonical = UnitEdge {
                    scale: edge.scale.mul(&canonical.scale),
                    offset: edge.scale.mul(&canonical.offset).add(&edge.offset),
                };
                path.push(edge);
                let next = units
                    .get(&target)
                    .ok_or(refuse(SemanticGraphCause::UnknownTarget))?;
                current = (target, next);
            }
            let dimension = dimensions
                .get(&unit.dimension)
                .cloned()
                .ok_or(refuse(SemanticGraphCause::UnknownDimension))?;
            Ok((
                *key,
                Unit {
                    key: *key,
                    dimension_node: unit.dimension,
                    dimension,
                    root: current.0,
                    path,
                    canonical,
                },
            ))
        })
        .collect()
}

// ---- compound units ----------------------------------------------------------

/// An evaluator-owned `quire.value.compound-unit/v1` identity digest. It is not
/// an I04 semantic node key.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct CompoundUnitIdentity([u8; 32]);

impl CompoundUnitIdentity {
    /// The raw digest.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Display for CompoundUnitIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.iter().try_for_each(|byte| write!(f, "{byte:02x}"))
    }
}

impl fmt::Debug for CompoundUnitIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CompoundUnitIdentity({self})")
    }
}

/// Why a compound-unit preimage was refused at construction.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("invalid compound unit: {cause:?}")]
pub struct InvalidCompoundUnit {
    /// The typed reason.
    pub cause: CompoundUnitCause,
}

/// The check that refused a compound-unit preimage.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CompoundUnitCause {
    /// The preimage does not satisfy `value-compound-unit.schema.json`.
    NonCanonicalPreimage,
    /// A term exponent is zero.
    ZeroExponent,
    /// A root unit appears twice.
    DuplicateTerm,
    /// Terms are not strictly ascending by node key.
    UnsortedTerms,
    /// A term does not name an admitted canonical root unit.
    NotRootUnit,
}

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

#[derive(Serialize)]
struct CanonicalCompoundTerm {
    exponent: String,
    unit_node_id: CanonicalNodeId,
}

#[derive(Serialize)]
struct CanonicalCompound {
    terms: Vec<CanonicalCompoundTerm>,
    version: &'static str,
}

fn compound_identity<'a, K: Into<CanonicalNodeId>>(
    terms: impl IntoIterator<Item = (K, &'a Integer)>,
) -> CompoundUnitIdentity {
    let preimage = CanonicalCompound {
        terms: terms
            .into_iter()
            .map(|(unit, exponent)| CanonicalCompoundTerm {
                exponent: exponent.to_string(),
                unit_node_id: unit.into(),
            })
            .collect(),
        version: COMPOUND_UNIT_DOMAIN,
    };
    let bytes = serde_json::to_vec(&preimage)
        .expect("a struct of strings, vectors and a constant always serializes to JSON");
    CompoundUnitIdentity(Sha256::digest(bytes).into())
}

/// A schema-valid `quire.value.compound-unit/v1` preimage, as spelled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompoundUnitPreimage {
    terms: Vec<(WireNodeId, Integer)>,
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
        Ok(Self { terms })
    }

    /// The identity of exactly these spelled terms.
    pub fn identity(&self) -> CompoundUnitIdentity {
        compound_identity(self.terms.iter().map(|(id, exponent)| (*id, exponent)))
    }
}

/// A normalized compound unit: canonical root-unit keys to nonzero exponents.
/// The empty map is the sole dimensionless unit.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct CompoundUnit {
    terms: BTreeMap<NodeKey, Integer>,
    dimension: Dimension,
}

impl CompoundUnit {
    /// The dimensionless unit.
    pub fn dimensionless() -> Self {
        Self::default()
    }

    /// The single-term compound unit `root^1` of a declared unit's root.
    pub(crate) fn of_root(unit: &Unit) -> Self {
        Self {
            terms: [(unit.root, Integer::one())].into(),
            dimension: unit.dimension.clone(),
        }
    }

    /// Ascending `(root unit, exponent)` terms.
    pub fn terms(&self) -> impl Iterator<Item = (NodeKey, &Integer)> {
        self.terms.iter().map(|(key, exponent)| (*key, exponent))
    }

    /// The normalized dimension map.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// The `quire.value.compound-unit/v1` identity.
    pub fn identity(&self) -> CompoundUnitIdentity {
        compound_identity(self.terms())
    }

    pub(crate) fn multiply(&self, other: &Self) -> Self {
        Self {
            terms: combine(&self.terms, &other.terms, Integer::add),
            dimension: self.dimension.multiply(&other.dimension),
        }
    }

    pub(crate) fn divide(&self, other: &Self) -> Self {
        Self {
            terms: combine(&self.terms, &other.terms, Integer::sub),
            dimension: self.dimension.divide(&other.dimension),
        }
    }

    pub(crate) fn power(&self, exponent: &Integer) -> Self {
        Self {
            terms: scale_exponents(&self.terms, exponent),
            dimension: self.dimension.power(exponent),
        }
    }
}

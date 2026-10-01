// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 dimensions, declared units, their admitted unit graph and the
//! evaluator-owned compound-unit value, at runtime (ADR-011 §6.1 layer SV).
//!
//! Base dimensions and units are I04 nominal nodes. A dimension is a sorted
//! map from base-dimension key to a nonzero mathematical exponent. The units
//! of one dimension node form an acyclic graph with exactly one targetless
//! canonical root; each edge maps `target_value = scale × source_value +
//! offset` exactly.
//!
//! This module owns the graph's topology and conversion logic: given nodes
//! that already passed their per-node checks, [`UnitGraph::from_checked_nodes`]
//! resolves their references, refuses a malformed graph and composes each
//! unit's exact path to its root. Reading a node's preimage, recomputing its
//! key and checking its owner is the compile side's work (`qsl-semantics`'
//! `value::unit`), which needs `serde` and RFC 8785.
//!
//! It never constructs a `NodeKey`: only QSL `check` mints one (ADR-011 §6.1,
//! ADR-013 O-04). A node id read from a preimage is its 32 digest bytes; it
//! resolves to a `NodeKey` only by lookup among the admitted nodes' keys.
//!
//! A declared unit's kernel [`UnitId`] is its admitted node key under the
//! `quire.checked-semantic-node/v1` label ([`Unit::id`], and C-30's
//! [`UnitGraph::declared_unit_id`], which refuses any key that is not an
//! admitted unit's). A compound unit's is its `quire.value.compound-unit/v1`
//! digest ([`CompoundUnit::id`], [`compound_unit_id`]), encoded and hashed by
//! `quire-canonical` (ADR-013 §2, ADR-013:113: the one RFC 8785
//! implementation).

use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::hash::{Hash, Hasher};

use serde::Serialize;

use quire_exact::{Integer, NodeKey, Rational, UnitId, COMPOUND_UNIT_DOMAIN, NODE_KEY_DOMAIN};

use crate::semantic_node::{
    check_terms, InvalidSemanticGraph, SemanticGraphCause, IDENTITY_LIMITS,
};

/// The graph refusal of `cause`.
fn refuse(cause: SemanticGraphCause) -> InvalidSemanticGraph {
    InvalidSemanticGraph { cause }
}

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

// ---- units -------------------------------------------------------------------

/// One exact affine edge `target = scale × source + offset`.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct UnitEdge {
    scale: Rational,
    offset: Rational,
}

impl UnitEdge {
    /// A unit node's edge after its semantic checks, in order: a zero scale
    /// refuses, then a canonical root (`is_root`) whose edge is not the
    /// identity (scale one, offset zero).
    pub fn checked(
        scale: Rational,
        offset: Rational,
        is_root: bool,
    ) -> Result<Self, SemanticGraphCause> {
        if scale.is_zero() {
            return Err(SemanticGraphCause::ZeroScale);
        }
        let identity = scale == Rational::from_integer(Integer::one()) && offset.is_zero();
        if is_root && !identity {
            return Err(SemanticGraphCause::NonIdentityRoot);
        }
        Ok(Self { scale, offset })
    }

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

    /// The declared-arm [`UnitId`]: this admitted unit's node key under the
    /// `quire.checked-semantic-node/v1` label (ADR-013 T-6, OQ-B).
    pub fn id(&self) -> UnitId {
        UnitId::declared(self.key)
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

/// A dimension node after its per-node checks, as read: its
/// `(base dimension id, exponent)` terms, empty for a base dimension. Each id
/// is the 32 digest bytes of a node key, resolved by lookup among the
/// admitted dimension keys.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DimensionNode {
    /// The terms, as read.
    pub terms: Vec<([u8; 32], Integer)>,
}

/// A unit node after its per-node checks, as read. Its dimension and target
/// ids are the 32 digest bytes of node keys, resolved by lookup among the
/// admitted keys.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnitNode {
    /// The unit's dimension node id.
    pub dimension: [u8; 32],
    /// The target unit node id, or `None` for a canonical root.
    pub target: Option<[u8; 32]>,
    /// The checked edge to the target ([`UnitEdge::checked`]).
    pub edge: UnitEdge,
}

/// An admitted closed set of dimension and unit nodes.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UnitGraph {
    dimensions: BTreeMap<NodeKey, Dimension>,
    units: BTreeMap<NodeKey, Unit>,
    /// Resolves a unit node id to an admitted unit's key.
    unit_ids: BTreeMap<[u8; 32], NodeKey>,
}

/// A unit node whose dimension and target resolved to admitted keys.
struct ResolvedUnit {
    dimension: NodeKey,
    target: Option<NodeKey>,
    edge: UnitEdge,
}

/// The index that resolves a node id to one of `keys` by lookup.
fn id_index(keys: impl IntoIterator<Item = NodeKey>) -> BTreeMap<[u8; 32], NodeKey> {
    keys.into_iter().map(|key| (*key.as_bytes(), key)).collect()
}

/// The admitted key `id` names, or `None` when no admitted node has it.
fn resolve(index: &BTreeMap<[u8; 32], NodeKey>, id: [u8; 32]) -> Option<NodeKey> {
    index.get(&id).copied()
}

impl UnitGraph {
    /// Admit the topology of dimension and unit nodes that already passed
    /// their per-node checks, keyed by their distinct retained keys.
    ///
    /// The graph, referenced by node ids, is checked for unknown or derived
    /// dimension terms, unknown dimensions, unknown and cross-dimension
    /// targets, per-dimension root count and target cycles, in that order.
    /// Every refusal is `invalid_semantic_graph`; the typed cause names the
    /// first failed check.
    pub fn from_checked_nodes(
        dimensions: &BTreeMap<NodeKey, DimensionNode>,
        units: &BTreeMap<NodeKey, UnitNode>,
    ) -> Result<Self, InvalidSemanticGraph> {
        let dimension_ids = id_index(dimensions.keys().copied());
        let dimension_maps = dimension_maps(dimensions, &dimension_ids)?;
        let unit_ids = id_index(units.keys().copied());
        let units = unit_paths(
            &resolve_units(units, &dimension_ids, &unit_ids)?,
            &dimension_maps,
        )?;
        Ok(Self {
            dimensions: dimension_maps,
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

    /// Every admitted unit, ascending by node key.
    pub fn units(&self) -> impl Iterator<Item = &Unit> {
        self.units.values()
    }

    /// ADR-013 C-30: a unit node key as a declared-arm [`UnitId`]. Only the
    /// key of an admitted unit converts, and admission checked that key
    /// against its `quire.unit-node/v1` preimage; any other node key, such
    /// as a dimension's, is refused.
    pub fn declared_unit_id(&self, key: NodeKey) -> Result<UnitId, NotAUnitKey> {
        self.unit(key).map(Unit::id).ok_or(NotAUnitKey { key })
    }

    /// Construct a compound unit from schema-valid `(unit node id,
    /// exponent)` terms: every term must name an admitted canonical root
    /// unit with a nonzero exponent, strictly ascending by id.
    pub fn compound_unit(
        &self,
        terms: &[([u8; 32], Integer)],
    ) -> Result<CompoundUnit, InvalidCompoundUnit> {
        check_terms(terms).map_err(|cause| InvalidCompoundUnit {
            cause: match cause {
                SemanticGraphCause::ZeroExponent => CompoundUnitCause::ZeroExponent,
                SemanticGraphCause::DuplicateTerm => CompoundUnitCause::DuplicateTerm,
                // `check_terms` raises only the three term causes.
                _ => CompoundUnitCause::UnsortedTerms,
            },
        })?;
        let mut dimension = Dimension::dimensionless();
        let mut compound_terms = BTreeMap::new();
        for (id, exponent) in terms {
            let (key, root) = resolve(&self.unit_ids, *id)
                .and_then(|key| Some((key, self.units.get(&key)?)))
                .filter(|(key, unit)| unit.root == *key)
                .ok_or(InvalidCompoundUnit {
                    cause: CompoundUnitCause::NotRootUnit,
                })?;
            dimension = dimension.multiply(&root.dimension.power(exponent));
            compound_terms.insert(key, exponent.clone());
        }
        Ok(CompoundUnit::new(compound_terms, dimension))
    }
}

/// C-30's refusal: the node key names no admitted unit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("node key {key} names no admitted unit")]
pub struct NotAUnitKey {
    /// The refused node key.
    pub key: NodeKey,
}

/// A base dimension maps to itself; a derived dimension to its base terms,
/// each resolved by lookup among the admitted dimension keys.
fn dimension_maps(
    dimensions: &BTreeMap<NodeKey, DimensionNode>,
    ids: &BTreeMap<[u8; 32], NodeKey>,
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
    units: &BTreeMap<NodeKey, UnitNode>,
    dimension_ids: &BTreeMap<[u8; 32], NodeKey>,
    unit_ids: &BTreeMap<[u8; 32], NodeKey>,
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

// RFC 8785 JCS: `quire-canonical` orders members itself, so field
// declaration order carries no meaning.
#[derive(Serialize)]
struct CanonicalNodeId {
    digest: String,
    domain: &'static str,
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

/// A node id's 32 digest bytes as 64 lowercase hexadecimal digits, the
/// spelling `NodeKey`'s `Display` uses.
struct DigestHex([u8; 32]);

impl fmt::Display for DigestHex {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0
            .iter()
            .try_for_each(|byte| write!(formatter, "{byte:02x}"))
    }
}

/// The compound-arm [`UnitId`] of exactly these `(unit node id, exponent)`
/// terms, each id as its 32 digest bytes: the SHA-256 of their JCS
/// `quire.value.compound-unit/v1` preimage, encoded and hashed by
/// `quire-canonical` (ADR-013 §2, ADR-013:113: the one RFC 8785
/// implementation).
pub fn compound_unit_id<'a>(terms: impl IntoIterator<Item = ([u8; 32], &'a Integer)>) -> UnitId {
    let preimage = CanonicalCompound {
        terms: terms
            .into_iter()
            .map(|(unit, exponent)| CanonicalCompoundTerm {
                exponent: exponent.to_string(),
                unit_node_id: CanonicalNodeId {
                    digest: DigestHex(unit).to_string(),
                    domain: NODE_KEY_DOMAIN,
                },
            })
            .collect(),
        version: COMPOUND_UNIT_DOMAIN,
    };
    // A struct of strings, arrays and a constant always has an RFC 8785
    // encoding, and `IDENTITY_LIMITS` sets no byte ceiling; the one refusal
    // left is a failed heap reservation, which the `serde_json` encoder this
    // replaced aborted the process on.
    let digest = quire_canonical::sha256(&preimage, IDENTITY_LIMITS)
        .unwrap_or_else(|error| panic!("a compound-unit preimage encodes: {error}"));
    UnitId::compound(*digest.as_bytes())
}

/// A normalized compound unit: canonical root-unit keys to nonzero exponents.
/// The empty map is the sole dimensionless unit. Two compound units are equal
/// exactly when their terms are, which is also when their ids are.
#[derive(Clone, Debug)]
pub struct CompoundUnit {
    terms: BTreeMap<NodeKey, Integer>,
    dimension: Dimension,
}

impl PartialEq for CompoundUnit {
    fn eq(&self, other: &Self) -> bool {
        self.terms == other.terms
    }
}

impl Eq for CompoundUnit {}

impl Hash for CompoundUnit {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.terms.hash(state);
    }
}

impl CompoundUnit {
    fn new(terms: BTreeMap<NodeKey, Integer>, dimension: Dimension) -> Self {
        Self { terms, dimension }
    }

    /// The single-term compound unit `root^1` of a declared unit's root.
    pub(crate) fn of_root(unit: &Unit) -> Self {
        Self::new([(unit.root, Integer::one())].into(), unit.dimension.clone())
    }

    /// Ascending `(root unit, exponent)` terms.
    pub fn terms(&self) -> impl Iterator<Item = (NodeKey, &Integer)> {
        self.terms.iter().map(|(key, exponent)| (*key, exponent))
    }

    /// The normalized dimension map.
    pub fn dimension(&self) -> &Dimension {
        &self.dimension
    }

    /// The compound-arm [`UnitId`]: the `quire.value.compound-unit/v1`
    /// digest of the terms (ADR-013 T-6, OQ-B), computed on each call.
    pub fn id(&self) -> UnitId {
        compound_unit_id(
            self.terms
                .iter()
                .map(|(key, exponent)| (*key.as_bytes(), exponent)),
        )
    }

    pub(crate) fn multiply(&self, other: &Self) -> Self {
        Self::new(
            combine(&self.terms, &other.terms, Integer::add),
            self.dimension.multiply(&other.dimension),
        )
    }

    pub(crate) fn divide(&self, other: &Self) -> Self {
        Self::new(
            combine(&self.terms, &other.terms, Integer::sub),
            self.dimension.divide(&other.dimension),
        )
    }

    pub(crate) fn power(&self, exponent: &Integer) -> Self {
        Self::new(
            scale_exponents(&self.terms, exponent),
            self.dimension.power(exponent),
        )
    }
}

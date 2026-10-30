// SPDX-License-Identifier: AGPL-3.0-or-later
//! I04 nominal semantic-node declarations shared by enum, dimension and unit
//! nodes (ADR-011 §6.2: layer 3 `semantic_value`).
//!
//! A node key is the structural [`NodeKey`] the check stage carries for the
//! declaration; nothing here hashes or recomputes one. This module owns the
//! owner projection and the reader for a node id spelled in a document: a
//! checked-package node id `{package, path, ordinal}` whose path segments are
//! names or nested node ids ([`NodeIdDocument::key`]).

use std::collections::BTreeSet;

use serde::Deserialize;

use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::Setting;
use quire_exact::{Integer, InvalidNodeKey, NodeKey, PathSegment};
use quire_semantic_value::semantic_node::{InvalidSemanticGraph, SemanticGraphCause};

/// The stable subject projection of the exact admitted owner of a nominal
/// declaration.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum NodeOwner {
    /// An exact admitted source.
    Source(OwnerSubject),
    /// An exact DefinitionRef.
    Definition(OwnerSubject),
    /// An exact model-selected declaration.
    Model(ModelSubject),
}

/// Authority and identity of a source or definition owner.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct OwnerSubject {
    /// Nonempty authority.
    pub authority: String,
    /// Nonempty identity.
    pub identity: String,
}

/// Identity and node of a model owner.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[serde(deny_unknown_fields)]
pub struct ModelSubject {
    /// The domain package identity.
    pub identity: String,
    /// The IR node identity.
    pub node: String,
}

impl NodeOwner {
    /// The structural key of the declaration `path` under this owner: the
    /// owner supplies the package and the leading path segments, the
    /// declaration path follows, and the ordinal is 0. A source or
    /// definition owner is package `authority` with path `[kind, identity,
    /// ..path]`; a model owner is package `identity` with path `["model",
    /// node, ..path]`. The owner's strings are never parsed or hashed.
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey`] when the key cannot be built.
    pub fn declaration_key(&self, path: &[String]) -> Result<NodeKey, InvalidNodeKey> {
        let (package, kind, identity) = match self {
            Self::Source(subject) => (&subject.authority, "source", &subject.identity),
            Self::Definition(subject) => (&subject.authority, "definition", &subject.identity),
            Self::Model(subject) => (&subject.identity, "model", &subject.node),
        };
        let mut segments = Vec::new();
        segments
            .try_reserve_exact(path.len().saturating_add(2))
            .map_err(|_| InvalidNodeKey::Allocation {
                requested: path.len().saturating_add(2),
            })?;
        segments.push(PathSegment::Text(kind.to_owned()));
        segments.push(PathSegment::Text(identity.clone()));
        segments.extend(path.iter().cloned().map(PathSegment::Text));
        NodeKey::new(package.clone(), segments, 0)
    }

    /// The structural handle of the record or tuple `name` declared under
    /// this owner, which `ValueType::Composite` carries before lowering.
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey`] when the key cannot be built.
    pub fn declared_type_handle(&self, name: &str) -> Result<NodeKey, InvalidNodeKey> {
        self.declaration_key(&[name.to_owned()])
    }

    pub(crate) fn is_well_formed(&self) -> bool {
        match self {
            Self::Source(subject) | Self::Definition(subject) => {
                !subject.authority.is_empty() && !subject.identity.is_empty()
            }
            Self::Model(subject) => !subject.identity.is_empty() && !subject.node.is_empty(),
        }
    }
}

/// The owners the package's lock selection admits; a declaration owner must
/// join this set.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnerSelection(BTreeSet<NodeOwner>);

impl OwnerSelection {
    /// Select exactly `owners`.
    pub fn new(owners: impl IntoIterator<Item = NodeOwner>) -> Self {
        Self(owners.into_iter().collect())
    }

    /// Whether `owner` joins the selection.
    pub fn contains(&self, owner: &NodeOwner) -> bool {
        self.0.contains(owner)
    }
}

pub(crate) fn refuse(cause: SemanticGraphCause) -> InvalidSemanticGraph {
    InvalidSemanticGraph { cause }
}

/// A checked-package node id as a document spells it:
/// `{"package", "path": [name | nested node id, ...], "ordinal"}`. Kept as
/// parsed JSON and read by [`Self::key`], which walks nested ids with an
/// explicit stack, never recursion.
#[derive(Deserialize)]
#[serde(transparent)]
pub(crate) struct NodeIdDocument(serde_json::Value);

/// One node id being assembled: its package, the path read so far, the path
/// elements still to read and its ordinal.
struct NodeFrame<'a> {
    package: &'a str,
    path: Vec<PathSegment>,
    remaining: std::slice::Iter<'a, serde_json::Value>,
    ordinal: u64,
}

impl<'a> NodeFrame<'a> {
    fn open(value: &'a serde_json::Value) -> Option<Self> {
        let object = value.as_object()?;
        if object.len() != 3 {
            return None;
        }
        let package = object.get("package")?.as_str()?;
        let path = object.get("path")?.as_array()?;
        let ordinal = object.get("ordinal")?.as_u64()?;
        let mut segments = Vec::new();
        segments.try_reserve_exact(path.len()).ok()?;
        Some(Self {
            package,
            path: segments,
            remaining: path.iter(),
            ordinal,
        })
    }
}

impl NodeIdDocument {
    /// The structural key this document spells, or `None` for a shape or
    /// nesting that is not a valid node identity.
    pub(crate) fn key(&self) -> Option<NodeKey> {
        let mut stack = vec![NodeFrame::open(&self.0)?];
        loop {
            let frame = stack.last_mut()?;
            if let Some(element) = frame.remaining.next() {
                match element {
                    serde_json::Value::String(name) => {
                        frame.path.push(PathSegment::Text(name.clone()));
                    }
                    nested => stack.push(NodeFrame::open(nested)?),
                }
                continue;
            }
            let done = stack.pop()?;
            let key = NodeKey::new(done.package, done.path, done.ordinal).ok()?;
            match stack.last_mut() {
                Some(parent) => parent.path.push(PathSegment::Node(key)),
                None => return Some(key),
            }
        }
    }
}

/// A schema `Rational`: canonical integer numerator and positive denominator
/// spellings.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RationalDocument {
    numerator: String,
    denominator: String,
}

impl RationalDocument {
    /// The exact spelled parts, or `None` for a non-canonical spelling.
    pub(crate) fn parts(&self) -> Option<(Integer, Integer)> {
        let numerator: Integer = self.numerator.parse().ok()?;
        let denominator: Integer = self.denominator.parse().ok()?;
        (!denominator.is_zero() && !denominator.is_negative()).then_some((numerator, denominator))
    }
}

/// A schema `QualifiedName`: one or more identifiers.
pub(crate) fn is_qualified_name(segments: &[String]) -> bool {
    !segments.is_empty()
        && segments
            .iter()
            .all(|segment| quire_exact::is_identifier(segment))
}

/// Why a nominal declaration was refused: an `invalid_semantic_graph` refusal,
/// or a reservation or byte limit it reached.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum NominalRefusal {
    /// The preimage or its graph is invalid.
    #[error(transparent)]
    Graph(#[from] InvalidSemanticGraph),
    /// The preimage's canonical bytes would exceed `identity.input_bytes`
    /// (FR-259 Behavior 4): a stage limit, never a malformed value.
    #[error("stage_limit_exceeded: {0:?}")]
    Limit(LimitExceeded),
    /// A heap reservation for the preimage's canonical bytes failed
    /// (FR-259 Behavior 6): `resource_exhausted`/`allocation-failed`, no
    /// limit and no setting.
    #[error("resource_exhausted/allocation-failed: {requested} bytes")]
    Allocation {
        /// The size in bytes of the reservation that failed.
        requested: usize,
    },
}

/// The `identity.input_bytes` stage limit an identity encoding reached:
/// input bytes, its bound and the bytes it needed (FR-259 Behavior 4).
pub fn identity_limit(bound: u64, required: u64) -> LimitExceeded {
    LimitExceeded::new(Setting::IdentityInputBytes, bound, u128::from(required))
}

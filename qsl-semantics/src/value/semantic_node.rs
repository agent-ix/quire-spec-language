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
    /// The nonempty name of the package whose nodes this owner mints; it is
    /// the package component of every node key under the owner.
    pub package_name: String,
}

impl OwnerSubject {
    /// An owner subject of `authority` and `identity` minting nodes in
    /// package `package_name`.
    pub fn new(
        authority: impl Into<String>,
        identity: impl Into<String>,
        package_name: impl Into<String>,
    ) -> Self {
        Self {
            authority: authority.into(),
            identity: identity.into(),
            package_name: package_name.into(),
        }
    }
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
    /// definition owner is package `package_name` with path `path`; a model
    /// owner is package `identity` with path `[node, ..path]`. The owner's strings are never parsed or hashed.
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey`] when the key cannot be built.
    pub fn declaration_key(&self, path: &[String]) -> Result<NodeKey, InvalidNodeKey> {
        self.declaration_key_at(path, 0)
    }

    /// [`Self::declaration_key`] with an explicit producer `ordinal`, which
    /// separates declarations of different kinds that share one package and
    /// declaration path (FR-322: the ordinal is unique among nodes sharing
    /// both).
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey`] when the key cannot be built.
    pub fn declaration_key_at(
        &self,
        path: &[String],
        ordinal: u64,
    ) -> Result<NodeKey, InvalidNodeKey> {
        let (package, prefix): (&String, Vec<&String>) = match self {
            Self::Source(subject) | Self::Definition(subject) => {
                (&subject.package_name, Vec::new())
            }
            Self::Model(subject) => (&subject.identity, vec![&subject.node]),
        };
        let mut segments = Vec::new();
        let requested = path.len().saturating_add(prefix.len());
        segments
            .try_reserve_exact(requested)
            .map_err(|_| InvalidNodeKey::Allocation { requested })?;
        segments.extend(prefix.into_iter().cloned().map(PathSegment::Text));
        segments.extend(path.iter().cloned().map(PathSegment::Text));
        NodeKey::new(package.clone(), segments, ordinal)
    }

    /// The structural key of a model-owned declaration of `form`
    /// (`record_value_type` or `variant_type`): package `identity`, path
    /// `["model", form, node, ..path]`, ordinal 0, the IR `declaration_id`
    /// shape.
    ///
    /// # Errors
    ///
    /// [`InvalidNodeKey`] when the key cannot be built, or when this is not a
    /// model owner (the error is then `EmptyPackage`: no model package).
    pub fn model_declaration_key(
        &self,
        form: &str,
        path: &[String],
    ) -> Result<NodeKey, InvalidNodeKey> {
        let Self::Model(subject) = self else {
            return Err(InvalidNodeKey::EmptyPackage);
        };
        let requested = path.len().saturating_add(3);
        let mut segments = Vec::new();
        segments
            .try_reserve_exact(requested)
            .map_err(|_| InvalidNodeKey::Allocation { requested })?;
        segments.push(PathSegment::Text("model".to_owned()));
        segments.push(PathSegment::Text(form.to_owned()));
        segments.push(PathSegment::Text(subject.node.clone()));
        segments.extend(path.iter().cloned().map(PathSegment::Text));
        NodeKey::new(subject.identity.clone(), segments, 0)
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
                !subject.authority.is_empty()
                    && !subject.identity.is_empty()
                    && !subject.package_name.is_empty()
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

impl NodeIdDocument {
    /// The structural key this document spells, or `None` for a shape or
    /// nesting that is not a valid node identity, through the shared
    /// `qsl_foundation::node_wire` codec.
    pub(crate) fn key(&self) -> Option<NodeKey> {
        qsl_foundation::node_wire::node_key_from_json(&self.0).ok()
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

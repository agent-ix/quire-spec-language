// SPDX-License-Identifier: AGPL-3.0-or-later
//! The proved checked-package document a replay request carries (QSpec
//! FR-336's `package` member), retained as read.

use super::{PackageMemberCause, ReplayRequestRefusal};
use crate::identity::SourceRefWire;
use quire_canonical::{Document, Node, NodeRef, ReadError};

/// A complete raw checked-package document, without an admission claim.
///
/// Reading checks JSON syntax only. The request decoder reads the package's
/// name, lock sources and lock `refinement_internals` from it
/// (`package.name`, `package.lock.sources`,
/// `package.lock.refinement_internals`); the original bytes stay unchanged.
#[derive(Clone)]
pub struct ReplayPackage {
    document: Document,
    bytes: Vec<u8>,
}

impl std::fmt::Debug for ReplayPackage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ReplayPackage")
            .field("input_bytes", &self.bytes.len())
            .finish()
    }
}

impl PartialEq for ReplayPackage {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for ReplayPackage {}

impl ReplayPackage {
    /// Read one complete raw JSON document under the caller's byte ceiling.
    pub fn read(bytes: &[u8], max_input_bytes: u64) -> Result<Self, ReadError> {
        let document = quire_canonical::read(bytes, max_input_bytes)?;
        let mut retained = Vec::new();
        retained
            .try_reserve(bytes.len())
            .map_err(|_| ReadError::Allocation {
                requested: bytes.len(),
            })?;
        retained.extend_from_slice(bytes);
        Ok(Self {
            document,
            bytes: retained,
        })
    }

    /// The entire original document.
    #[must_use]
    pub fn document(&self) -> &Document {
        &self.document
    }

    /// The package's top-level `name`, when it is a string.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        match member(self.document.root(), "name")?.node() {
            Node::String(text) => Some(text),
            _ => None,
        }
    }

    /// The original bytes.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// A document holding only what the request decoder reads: the package
    /// `name`, its `lock.sources` and optionally its `lock.refinement_internals`. For tests that exercise the decoder
    /// without compiling a package; it carries no other checked-package member.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    #[must_use]
    pub fn descriptor_only(
        name: &str,
        sources: &[SourceRefWire],
        refinement_internals: Option<serde_json::Value>,
    ) -> Self {
        let sources: Vec<_> = sources
            .iter()
            .map(|(authority, identity)| {
                serde_json::json!({"authority": authority, "identity": identity})
            })
            .collect();
        let mut lock = serde_json::json!({"sources": sources});
        if let Some(internals) = refinement_internals {
            lock["refinement_internals"] = internals;
        }
        let bytes = serde_json::json!({"name": name, "lock": lock}).to_string();
        match Self::read(bytes.as_bytes(), u64::MAX) {
            Ok(package) => package,
            Err(error) => panic!("descriptor JSON reads: {error:?}"),
        }
    }

    /// This document with `edit` applied to its JSON value, for tests that
    /// vary one member of an otherwise complete package.
    #[cfg(any(test, feature = "test-support"))]
    #[doc(hidden)]
    #[must_use]
    pub fn edited(&self, edit: impl FnOnce(&mut serde_json::Value)) -> Self {
        let mut value: serde_json::Value = match serde_json::from_slice(&self.bytes) {
            Ok(value) => value,
            Err(error) => panic!("the package bytes are JSON: {error}"),
        };
        edit(&mut value);
        match Self::read(value.to_string().as_bytes(), u64::MAX) {
            Ok(package) => package,
            Err(error) => panic!("the edited package reads: {error:?}"),
        }
    }

    pub(super) fn descriptor(&self) -> Result<Descriptor, ReplayRequestRefusal> {
        let root = self.document.root();
        let name = string(required(root, "name", "/package")?, "/package/name")?.to_owned();
        let lock = required(root, "lock", "/package")?;
        let sources_node = required(lock, "sources", "/package/lock")?;
        let sources = array(sources_node, "/package/lock/sources")?
            .enumerate()
            .map(|(index, source)| {
                let pointer = format!("/package/lock/sources/{index}");
                Ok((
                    string(
                        required(source, "authority", &pointer)?,
                        &format!("{pointer}/authority"),
                    )?
                    .to_owned(),
                    string(
                        required(source, "identity", &pointer)?,
                        &format!("{pointer}/identity"),
                    )?
                    .to_owned(),
                ))
            })
            .collect::<Result<Vec<_>, ReplayRequestRefusal>>()?;
        // Kept unchecked, as the recompile reports a defect of its value.
        let refinement_internals = member(lock, "refinement_internals")
            .map(|rows| {
                let bytes = quire_canonical::to_vec(
                    &rows,
                    quire_canonical::Limits::new(self.bytes.len() as u64),
                )
                .map_err(|_| {
                    field_refusal(
                        "/package/lock/refinement_internals",
                        PackageMemberCause::WrongKind,
                    )
                })?;
                serde_json::from_slice(&bytes).map_err(|_| {
                    field_refusal(
                        "/package/lock/refinement_internals",
                        PackageMemberCause::WrongKind,
                    )
                })
            })
            .transpose()?;
        Ok(Descriptor {
            name,
            sources,
            refinement_internals,
        })
    }
}

pub(super) struct Descriptor {
    pub(super) name: String,
    pub(super) sources: Vec<SourceRefWire>,
    pub(super) refinement_internals: Option<serde_json::Value>,
}

fn required<'document>(
    value: NodeRef<'document>,
    key: &str,
    pointer: &str,
) -> Result<NodeRef<'document>, ReplayRequestRefusal> {
    if !matches!(value.node(), Node::Object(_)) {
        return Err(field_refusal(pointer, PackageMemberCause::WrongKind));
    }
    member(value, key)
        .ok_or_else(|| field_refusal(&format!("{pointer}/{key}"), PackageMemberCause::Missing))
}

fn string<'document>(
    value: NodeRef<'document>,
    pointer: &str,
) -> Result<&'document str, ReplayRequestRefusal> {
    match value.node() {
        Node::String(text) => Ok(text),
        _ => Err(field_refusal(pointer, PackageMemberCause::WrongKind)),
    }
}

fn array<'document>(
    value: NodeRef<'document>,
    pointer: &str,
) -> Result<quire_canonical::Items<'document>, ReplayRequestRefusal> {
    match value.node() {
        Node::Array(values) => Ok(values),
        _ => Err(field_refusal(pointer, PackageMemberCause::WrongKind)),
    }
}

fn field_refusal(pointer: &str, cause: PackageMemberCause) -> ReplayRequestRefusal {
    ReplayRequestRefusal::PackageMember {
        pointer: pointer.to_owned(),
        cause,
    }
}

fn member<'document>(value: NodeRef<'document>, name: &str) -> Option<NodeRef<'document>> {
    let Node::Object(members) = value.node() else {
        return None;
    };
    members
        .into_iter()
        .find(|(key, _)| *key == name)
        .map(|(_, value)| value)
}

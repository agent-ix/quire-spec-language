// SPDX-License-Identifier: AGPL-3.0-or-later
//! The producer interface `1.3.0` bundle: FR-150's normalization input.
//!
//! QSL does not (yet) receive this bundle from a live Semantic IR 2.0.0
//! intake (`filament-core-data#173`, unmerged); the shape below is the
//! `model-effective-declaration.schema.json`/`model-complete.md` producer
//! bundle exactly as the correspondence defines it, so normalization built
//! against it needs no rewrite once a real intake supplies one. This module
//! owns no registry: a [`Bundle`] is a value the caller passes in and
//! [`crate::model::normalize`] consumes; nothing here is reachable except
//! through that value.

use crate::model::key::{ProducerDigest, ProducerKey, Revision};

/// The one contract version this rung admits (`model-complete.md`).
pub const INTERFACE_VERSION_1_3_0: &str = "1.3.0";

/// A field or association-end multiplicity (FCD FR-113).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Multiplicity {
    /// Lower bound, inclusive.
    pub lower: u64,
    /// Upper bound, inclusive; `None` is unbounded.
    pub upper: Option<u64>,
    /// Whether membership order is significant.
    pub ordered: bool,
    /// Whether membership is unique.
    pub unique: bool,
}

/// An object type export: `{key}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectTypeRecord {
    /// This type's original producer key.
    pub key: ProducerKey,
}

/// A field member of an object type: `{key, owner, value_type, multiplicity}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldMemberRecord {
    /// This member's own original producer key.
    pub key: ProducerKey,
    /// The owning object type's original producer key.
    pub owner: ProducerKey,
    /// The declared value type's original producer key.
    pub value_type: ProducerKey,
    /// The declared multiplicity.
    pub multiplicity: Multiplicity,
}

/// A generalization record: `{key, specific, general}` (`specific`
/// generalizes to `general`; `specific` is the more derived type).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneralizationRecord {
    /// This record's own original producer key.
    pub key: ProducerKey,
    /// The specific (more derived) type's original producer key.
    pub specific: ProducerKey,
    /// The general (less derived) type's original producer key.
    pub general: ProducerKey,
}

/// One producer record, in the bundle's declared order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BundleRecord {
    /// An object type export.
    ObjectType(ObjectTypeRecord),
    /// A field member of an object type.
    FieldMember(FieldMemberRecord),
    /// A generalization between two object types.
    Generalization(GeneralizationRecord),
}

impl BundleRecord {
    /// This record's own original producer key.
    pub fn key(&self) -> &ProducerKey {
        match self {
            Self::ObjectType(record) => &record.key,
            Self::FieldMember(record) => &record.key,
            Self::Generalization(record) => &record.key,
        }
    }
}

/// The model selection's export identity: `{identity, revision, digest}`
/// (`model-complete.md`'s `ModelSelection.export`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSelectionExport {
    /// The bundle's own producer identity (e.g. `bundle.n01`).
    pub identity: String,
    /// The bundle's producer revision.
    pub revision: Revision,
    /// The bundle's `filament-canonical-json-1` digest.
    pub digest: ProducerDigest,
}

/// The producer interface contract version: `{interface_version, wire_schema}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContractVersion {
    /// The producer interface version this bundle claims.
    pub interface_version: String,
    /// The wire schema identity for that interface version.
    pub wire_schema: String,
}

/// A model selection: `{authority, export, contract_version}` (FR-321).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSelection {
    /// The correspondence authority (e.g. `filament-core-data`).
    pub authority: String,
    /// The bundle's own export identity.
    pub export: ModelSelectionExport,
    /// The claimed contract version.
    pub contract_version: ContractVersion,
}

impl ModelSelection {
    pub(super) fn to_json(&self) -> serde_json::Value {
        use serde_json::{Map, Value};
        let mut export = Map::new();
        export.insert("identity".to_owned(), Value::String(self.export.identity.clone()));
        export.insert(
            "revision".to_owned(),
            Value::Object({
                let mut r = Map::new();
                r.insert("namespace".to_owned(), Value::String(self.export.revision.namespace.clone()));
                r.insert("value".to_owned(), Value::String(self.export.revision.value.clone()));
                r
            }),
        );
        export.insert(
            "digest".to_owned(),
            Value::Object({
                let mut d = Map::new();
                d.insert("domain".to_owned(), Value::String(self.export.digest.domain.clone()));
                d.insert("sha256".to_owned(), Value::String(super::key::hex(&self.export.digest.sha256)));
                d
            }),
        );
        let mut contract_version = Map::new();
        contract_version.insert(
            "interface_version".to_owned(),
            Value::String(self.contract_version.interface_version.clone()),
        );
        contract_version.insert("wire_schema".to_owned(), Value::String(self.contract_version.wire_schema.clone()));
        let mut object = Map::new();
        object.insert("authority".to_owned(), Value::String(self.authority.clone()));
        object.insert("export".to_owned(), Value::Object(export));
        object.insert("contract_version".to_owned(), Value::Object(contract_version));
        Value::Object(object)
    }

    /// A `filament-core-data` model selection for a bundle export named
    /// `identity` (e.g. `bundle.n01`), following TC-195/196/197/198's
    /// fixture convention.
    pub fn fixture(identity: impl Into<String>) -> Self {
        let identity = identity.into();
        Self {
            authority: "filament-core-data".to_owned(),
            export: ModelSelectionExport {
                digest: ProducerDigest::of_identity(&identity),
                revision: Revision::producer_object("1"),
                identity,
            },
            contract_version: ContractVersion {
                interface_version: INTERFACE_VERSION_1_3_0.to_owned(),
                wire_schema: "filament-core-data/producer-interface/1.3.0".to_owned(),
            },
        }
    }
}

/// A producer interface `1.3.0` bundle: a [`ModelSelection`] and its ordered
/// records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bundle {
    /// This bundle's model selection.
    pub model_selection: ModelSelection,
    /// The bundle's records, in the producer's declared order.
    pub records: Vec<BundleRecord>,
}

impl Bundle {
    /// A bundle over `records`, selected under `model_selection`.
    pub fn new(model_selection: ModelSelection, records: Vec<BundleRecord>) -> Self {
        Self { model_selection, records }
    }
}

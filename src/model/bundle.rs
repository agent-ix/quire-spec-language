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
use crate::value::OrderingOperator;

/// The one contract version this rung normalizes (`model-complete.md`).
pub const INTERFACE_VERSION_1_3_0: &str = "1.3.0";

/// The prior contract version this rung recognizes but cannot normalize: it
/// yields the fixed `unsupplied-producer-record` refusal sequence FR-150
/// defines (TC-195 N08), not an `unknown_wire` refusal (PR #140 F4).
pub const INTERFACE_VERSION_1_2_0: &str = "1.2.0";

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

/// An object type export: `{key, interfaceFeatures}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectTypeRecord {
    /// This type's original producer key.
    pub key: ProducerKey,
    /// `Some(features)` when the producer supplies `interfaceFeatures`
    /// (FR-152's Interface kind), `None` when it does not. `Some(vec![])`
    /// is a real, valid interface with zero declared features; the
    /// distinction from `None` is the capability itself, not emptiness.
    pub interface_features: Option<Vec<ProducerKey>>,
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

/// A scalar type export bound to a closed `Int[lower,upper]` domain
/// (FR-151's `model.Count`/`model.Small` fixtures). QSL V1 does not carry the
/// FR-149 equality-conversion table; this is the narrow slice this rung
/// needs to decide value-type conformance and FR-146 interval containment
/// for a scalar redefinition, not a general scalar type system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarTypeRecord {
    /// This type's own original producer key.
    pub key: ProducerKey,
    /// Inclusive lower bound.
    pub lower: i64,
    /// Inclusive upper bound.
    pub upper: i64,
}

/// One operation parameter: `{key, value_type, multiplicity}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationParameterRecord {
    /// This parameter's own original producer key.
    pub key: ProducerKey,
    /// The declared parameter value type's original producer key.
    pub value_type: ProducerKey,
    /// The declared parameter multiplicity.
    pub multiplicity: Multiplicity,
}

/// An operation's declared result: `{value_type, multiplicity}`; absent
/// entirely when the operation has no result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationResult {
    /// The declared result value type's original producer key.
    pub value_type: ProducerKey,
    /// The declared result multiplicity.
    pub multiplicity: Multiplicity,
}

/// An operation's producer-supplied effect frame: `{modifies, creates,
/// deletes}` (FR-151's `quire.model.conformance.effect/v1`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationEffect {
    /// Field members this operation writes.
    pub modifies: Vec<ProducerKey>,
    /// Object types this operation creates.
    pub creates: Vec<ProducerKey>,
    /// Object types this operation deletes.
    pub deletes: Vec<ProducerKey>,
}

/// One postcondition clause an operation's own postcondition declares about
/// `self.<field>` — a single accepted FR-146 guard-fact form, exactly as
/// FR-151's refinement obligation names it: `present(self.<field>)`, or one
/// ordering between `self.<field>` and an integer literal.
///
/// FR-146's expression parser is out of scope for `crate::model`, which
/// normalizes and checks a caller-constructed [`Bundle`] only and never
/// parses producer-supplied expression text. A postcondition clause is
/// therefore not parsed here: the caller states one accepted single-relation
/// guard form directly, as this typed value. What that clause actually
/// establishes is not caller-trusted, though: FR-151's refinement rule
/// (`crate::model::conformance::check_field_refinement_obligation`) rebuilds
/// the small synthetic guard tree the clause describes and runs it through
/// `crate::value`'s own FR-146 fact-derivation primitive
/// (`established_field_fact`) — the identical guard-fact propagation a real
/// checked postcondition's `Definedness::walk` already uses — then decides
/// discharge from what that derivation actually proves, never from the
/// clause's own literal restated as already-true. A [`Comparison`] clause
/// naming only a lower bound, for instance, does not by itself establish an
/// upper bound the derivation did not also produce.
///
/// [`Comparison`]: Self::Comparison
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PostconditionClause {
    /// `present(self.<field>)`.
    Presence {
        /// The field the postcondition establishes presence for.
        field: ProducerKey,
    },
    /// `self.<field> <operator> <literal>`, e.g. `self.cs <= 5`.
    Comparison {
        /// The field the postcondition relates to `literal`.
        field: ProducerKey,
        /// The stated ordering between `self.<field>` and `literal`.
        operator: OrderingOperator,
        /// The integer literal `self.<field>` is compared against.
        literal: i64,
    },
}

impl PostconditionClause {
    /// The field this clause is about.
    pub fn field(&self) -> &ProducerKey {
        match self {
            Self::Presence { field } | Self::Comparison { field, .. } => field,
        }
    }
}

/// An operation member of an object type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationMemberRecord {
    /// This member's own original producer key.
    pub key: ProducerKey,
    /// The owning object type's original producer key (the receiver type).
    pub owner: ProducerKey,
    /// Declared parameters, in signature order.
    pub parameters: Vec<OperationParameterRecord>,
    /// Declared result, or `None` when the operation has no result.
    pub result: Option<OperationResult>,
    /// Declared effect frame.
    pub effect: OperationEffect,
    /// Whether this operation carries an own precondition clause.
    pub has_own_precondition: bool,
    /// This operation's own postcondition clause(s), exactly as the caller
    /// states them (see [`PostconditionClause`]).
    pub own_postcondition_clauses: Vec<PostconditionClause>,
    /// Whether an `operation-body` declaration supplies this member's body.
    /// FR-151's dispatch family (`crate::model::dispatch`) is the original
    /// declaration together with every redefining operation reaching a
    /// subtype; only family members with a body are dispatch candidates. A
    /// member without a body (an abstract redefinition) is still a real
    /// family member for redefinition-conformance checking, just never a
    /// candidate.
    pub has_body: bool,
}

/// A redefinition record: `{key, owner, redefining, redefined}` — `owner`
/// declares `redefining`, which redefines the inherited `redefined` member.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RedefinitionRecord {
    /// This record's own original producer key.
    pub key: ProducerKey,
    /// The redefining member's owning object type.
    pub owner: ProducerKey,
    /// The redefining (more derived) member's original producer key.
    pub redefining: ProducerKey,
    /// The redefined (inherited) member's original producer key.
    pub redefined: ProducerKey,
}

/// FCD FR-114 component record: FR-152's Part candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRecord {
    /// This component's own original producer key.
    pub key: ProducerKey,
    /// The owning composite type's original producer key
    /// (`owningTypeIdentity`).
    pub owning_type: ProducerKey,
    /// The declared part type's original producer key (`typeIdentity`).
    pub value_type: ProducerKey,
    /// The declared multiplicity.
    pub multiplicity: Multiplicity,
    /// Whether the producer supplies the `part-signature` capability
    /// (FR-152's Part kind requires it).
    pub has_part_signature: bool,
}

/// FCD FR-114 endpoint direction (`port-direction`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PortDirection {
    /// `in`.
    In,
    /// `out`.
    Out,
    /// `inout`.
    InOut,
}

/// FCD FR-114 endpoint record: FR-152's Port candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointRecord {
    /// This endpoint's own original producer key.
    pub key: ProducerKey,
    /// The owning component's original producer key
    /// (`owningComponentIdentity`).
    pub owning_component: ProducerKey,
    /// The declared interface type's original producer key (`typeIdentity`).
    pub value_type: ProducerKey,
    /// `Some(direction)` when the producer supplies the `port-direction`
    /// capability (FR-152's Port kind requires it); `None` when it does not.
    pub direction: Option<PortDirection>,
    /// The declared multiplicity.
    pub multiplicity: Multiplicity,
}

/// One end of a [`RelationshipRecord`]: `{type_identity, multiplicity}`.
/// `type_identity` names whatever the end's `typeIdentity` names in the
/// correspondence — an endpoint, a component, an operation member or an
/// object type — which [`crate::model::systems`] resolves by kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipEnd {
    /// The named producer key.
    pub type_identity: ProducerKey,
    /// The declared multiplicity.
    pub multiplicity: Multiplicity,
}

/// FCD FR-115 relationship traversal direction (`semantics.direction`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RelationshipDirection {
    /// `source-to-target`.
    SourceToTarget,
    /// `target-to-source`.
    TargetToSource,
    /// `bidirectional`.
    Bidirectional,
    /// `undirected`.
    Undirected,
}

/// FCD FR-115 relationship record: FR-152's Connection/Allocation candidate,
/// or (when both ends name object types) a navigation-only relationship
/// with no kind.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipRecord {
    /// This relationship's own original producer key.
    pub key: ProducerKey,
    /// The source end.
    pub source: RelationshipEnd,
    /// The target end.
    pub target: RelationshipEnd,
    /// The producer's `semantics.category` bytes, e.g. `"allocation"`,
    /// `"connection"`, `"composition"`. Compared verbatim, never mapped
    /// through a closed Rust enum: FR-152 checks this field for exact byte
    /// equality to `"allocation"` and otherwise leaves it to the producer.
    pub category: String,
    /// The producer's `semantics.direction`.
    pub direction: RelationshipDirection,
}

/// A subsetting record: `{key, owner, subsetting, subsetted}` — `owner`
/// declares `subsetting`, whose runtime values are a subset of `subsetted`'s.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubsettingRecord {
    /// This record's own original producer key.
    pub key: ProducerKey,
    /// The subsetting member's owning object type.
    pub owner: ProducerKey,
    /// The subsetting feature's original producer key.
    pub subsetting: ProducerKey,
    /// The subsetted feature's original producer key.
    pub subsetted: ProducerKey,
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
    /// A scalar type export bound to a closed integer interval.
    ScalarType(ScalarTypeRecord),
    /// An operation member of an object type.
    OperationMember(OperationMemberRecord),
    /// An explicit redefinition of an inherited field or operation member.
    Redefinition(RedefinitionRecord),
    /// An explicit subsetting of another feature.
    Subsetting(SubsettingRecord),
    /// FCD FR-114 component (FR-152 Part candidate).
    Component(ComponentRecord),
    /// FCD FR-114 endpoint (FR-152 Port candidate).
    Endpoint(EndpointRecord),
    /// FCD FR-115 relationship (FR-152 Connection/Allocation candidate).
    Relationship(RelationshipRecord),
}

impl BundleRecord {
    /// This record's own original producer key.
    pub fn key(&self) -> &ProducerKey {
        match self {
            Self::ObjectType(record) => &record.key,
            Self::FieldMember(record) => &record.key,
            Self::Generalization(record) => &record.key,
            Self::ScalarType(record) => &record.key,
            Self::OperationMember(record) => &record.key,
            Self::Redefinition(record) => &record.key,
            Self::Subsetting(record) => &record.key,
            Self::Component(record) => &record.key,
            Self::Endpoint(record) => &record.key,
            Self::Relationship(record) => &record.key,
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
        export.insert(
            "identity".to_owned(),
            Value::String(self.export.identity.clone()),
        );
        export.insert(
            "revision".to_owned(),
            Value::Object({
                let mut r = Map::new();
                r.insert(
                    "namespace".to_owned(),
                    Value::String(self.export.revision.namespace.clone()),
                );
                r.insert(
                    "value".to_owned(),
                    Value::String(self.export.revision.value.clone()),
                );
                r
            }),
        );
        export.insert(
            "digest".to_owned(),
            Value::Object({
                let mut d = Map::new();
                d.insert(
                    "domain".to_owned(),
                    Value::String(self.export.digest.domain.clone()),
                );
                d.insert(
                    "sha256".to_owned(),
                    Value::String(super::key::hex(&self.export.digest.sha256)),
                );
                d
            }),
        );
        let mut contract_version = Map::new();
        contract_version.insert(
            "interface_version".to_owned(),
            Value::String(self.contract_version.interface_version.clone()),
        );
        contract_version.insert(
            "wire_schema".to_owned(),
            Value::String(self.contract_version.wire_schema.clone()),
        );
        let mut object = Map::new();
        object.insert(
            "authority".to_owned(),
            Value::String(self.authority.clone()),
        );
        object.insert("export".to_owned(), Value::Object(export));
        object.insert(
            "contract_version".to_owned(),
            Value::Object(contract_version),
        );
        Value::Object(object)
    }

    /// A `filament-core-data` model selection for a bundle export named
    /// `identity` (e.g. `bundle.n01`), following TC-195/196/197/198's
    /// fixture convention.
    ///
    /// Test-only (PR #140 F13): see [`ProducerDigest::of_identity`].
    #[cfg(any(test, feature = "test-support"))]
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
        Self {
            model_selection,
            records,
        }
    }
}

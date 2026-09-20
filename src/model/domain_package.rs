// SPDX-License-Identifier: AGPL-3.0-or-later
//! The domain package: FR-150's normalization input.
//!
//! The shape below is the `model-effective-declaration.schema.json`/
//! `model-complete.md` domain package exactly as FR-154 defines it. This
//! module owns no registry: a [`DomainPackage`] is a value the caller passes
//! in and [`crate::model::normalize`] consumes; nothing here is reachable
//! except through that value. [`crate::model::intake`] admits Semantic IR
//! 2.0.0 document bytes against a selection and reads an admitted
//! document's IR nodes into these records.

use crate::model::key::DeclarationKey;
use crate::value::OrderingOperator;

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

/// An object type export: `{key, interfaceFeatures, abstract, supertypes}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectTypeRecord {
    /// This type's original declaration key.
    pub key: DeclarationKey,
    /// `Some(features)` when the producer supplies `interfaceFeatures`
    /// (FR-152's Interface kind), `None` when it does not. `Some(vec![])`
    /// is a real, valid interface with zero declared features; the
    /// distinction from `None` is the capability itself, not emptiness.
    pub interface_features: Option<Vec<DeclarationKey>>,
    /// D05 (`model-complete.md:156`): an abstract type has no direct
    /// instances. A population member whose most-specific type is abstract
    /// is refused (`invalid_runtime_input`/`abstract-instance`), and
    /// dispatch covers only concrete subtypes. Named `abstract_type`, not
    /// `abstract`, since the latter is a reserved Rust keyword.
    pub abstract_type: bool,
    /// This type's declared `supertypes[]` (`model-complete.md`:155/159/160
    /// and :270/271): every entry names an object type of the package this
    /// type directly generalizes to. An inline property of the object type
    /// itself -- there is no producer key of its own for one generalization
    /// edge.
    pub supertypes: Vec<DeclarationKey>,
}

/// QSL's own closed native value-type vocabulary (shared-grammar.md's
/// `type-ref` production): the unparameterized value-type keywords a
/// `typeRef` may name directly, under `ix://quire/native/<Name>`, without
/// naming a node of any package. `Boolean`, `Integer`, `Float32` and
/// `Float64` take no parameters. `Integer`'s own optional inline bound
/// (`Int[lo,hi]`) is a constraint fact about the field, not a distinct
/// value-type reference. `Rational`, `Decimal` and `Text` are parameterized
/// in the grammar (`Rational[lo,hi;dmin,dmax]`, `Decimal[lo,hi;dmin,dmax]`,
/// `Text[min,max;profile]`) and REQUIRE their full parameter set to resolve
/// (see [`Self::unsupported_parameters`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeValueType {
    /// `Boolean`.
    Boolean,
    /// `Integer`.
    Integer,
    /// `Rational`.
    Rational,
    /// `Decimal`.
    Decimal,
    /// `Float32`.
    Float32,
    /// `Float64`.
    Float64,
    /// `Text`.
    Text,
}

impl NativeValueType {
    /// The `<Name>` past `ix://quire/native/`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Rational => "Rational",
            Self::Decimal => "Decimal",
            Self::Float32 => "Float32",
            Self::Float64 => "Float64",
            Self::Text => "Text",
        }
    }

    /// The required parameter names (shared-grammar.md's own labels for
    /// each production: `Rational`/`Decimal`'s `dmin`/`dmax`, `Text`'s
    /// `profile`) that `agent-ix-semantic-ir`'s own constraint vocabulary
    /// (`constraint_schema`'s closed `keyword` set: `min`, `max`,
    /// `exclusiveMin`, `exclusiveMax`, `minLength`, `maxLength`, `pattern`,
    /// `enumValues`, `nonEmpty`, `unique`, `format`) has no keyword for at
    /// all -- so a `typeRef` naming this native type can never resolve, no
    /// matter what a producer's own document supplies. `Rational`/`Decimal`
    /// also require `lo`/`hi`, which the vocabulary's `min`/`max` do carry;
    /// only the decimal-scale bound is unexpressable. `Boolean`, `Integer`,
    /// `Float32` and `Float64` take no required parameter, so this is empty
    /// for them.
    pub fn unsupported_parameters(&self) -> &'static [&'static str] {
        match self {
            Self::Boolean | Self::Integer | Self::Float32 | Self::Float64 => &[],
            Self::Rational | Self::Decimal => &["dmin", "dmax"],
            Self::Text => &["profile"],
        }
    }
}

impl std::str::FromStr for NativeValueType {
    type Err = ();

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "Boolean" => Ok(Self::Boolean),
            "Integer" => Ok(Self::Integer),
            "Rational" => Ok(Self::Rational),
            "Decimal" => Ok(Self::Decimal),
            "Float32" => Ok(Self::Float32),
            "Float64" => Ok(Self::Float64),
            "Text" => Ok(Self::Text),
            _ => Err(()),
        }
    }
}

/// A field, parameter or result's resolved value type: QSL's own closed
/// native vocabulary, declaring no node of any package, or a node of some
/// package. The sole output shape [`crate::model::intake`]'s unified
/// `typeRef` resolver produces -- never a synthetic `DeclarationKey` under a
/// made-up "quire/native" package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValueTypeRef {
    /// A native value type: `ix://quire/native/<Name>`.
    Native(NativeValueType),
    /// A node of a package: `ix://<package>/<artifact id>`.
    Package(DeclarationKey),
}

impl ValueTypeRef {
    /// This value type's own declaration key, when it names a node of a
    /// package; `None` for a native value type, which names no node.
    pub fn as_package(&self) -> Option<&DeclarationKey> {
        match self {
            Self::Native(_) => None,
            Self::Package(key) => Some(key),
        }
    }
}

impl std::fmt::Display for ValueTypeRef {
    /// The value type's own identity string, exactly as a `typeRef`
    /// resolving to it would spell it on the wire.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Native(native) => write!(f, "ix://quire/native/{}", native.as_str()),
            Self::Package(key) => f.write_str(&key.node),
        }
    }
}

/// A field member of an object type: `{key, owner, value_type, multiplicity,
/// subsets, redefines}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldMemberRecord {
    /// This member's own original declaration key.
    pub key: DeclarationKey,
    /// The owning object type's original declaration key.
    pub owner: DeclarationKey,
    /// The declared value type.
    pub value_type: ValueTypeRef,
    /// The declared multiplicity.
    pub multiplicity: Multiplicity,
    /// This field's declared `subsets[]` (`model-complete.md`:161): every
    /// entry names a field of the owning type or of a supertype whose
    /// runtime values this field's are a subset of. An inline property of
    /// the field itself.
    pub subsets: Vec<DeclarationKey>,
    /// This field's declared `redefines` (`model-complete.md`:162): the
    /// inherited field or operation this member redefines, or `None` when
    /// it declares no redefinition. An inline property of the field itself.
    pub redefines: Option<DeclarationKey>,
}

/// A scalar type export bound to a closed `Int[lower,upper]` domain
/// (FR-151's `model.Count`/`model.Small` fixtures). QSL V1 does not carry the
/// FR-149 equality-conversion table; this is the narrow slice this rung
/// needs to decide value-type conformance and FR-146 interval containment
/// for a scalar redefinition, not a general scalar type system.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarTypeRecord {
    /// This type's own original declaration key.
    pub key: DeclarationKey,
    /// Inclusive lower bound.
    pub lower: i64,
    /// Inclusive upper bound.
    pub upper: i64,
}

/// One operation parameter: `{key, value_type, multiplicity}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationParameterRecord {
    /// This parameter's own original declaration key.
    pub key: DeclarationKey,
    /// The declared parameter value type.
    pub value_type: ValueTypeRef,
    /// The declared parameter multiplicity.
    pub multiplicity: Multiplicity,
}

/// An operation's declared result: `{value_type, multiplicity}`; absent
/// entirely when the operation has no result.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationResult {
    /// The declared result value type.
    pub value_type: ValueTypeRef,
    /// The declared result multiplicity.
    pub multiplicity: Multiplicity,
}

/// An operation's producer-supplied effect frame: `{modifies, creates,
/// deletes}` (FR-151's `quire.model.conformance.effect/v1`).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OperationEffect {
    /// Field members this operation writes.
    pub modifies: Vec<DeclarationKey>,
    /// Object types this operation creates.
    pub creates: Vec<DeclarationKey>,
    /// Object types this operation deletes.
    pub deletes: Vec<DeclarationKey>,
}

/// One postcondition clause an operation's own postcondition declares about
/// `self.<field>` — a single accepted FR-146 guard-fact form, exactly as
/// FR-151's refinement obligation names it: `present(self.<field>)`, or one
/// ordering between `self.<field>` and an integer literal.
///
/// FR-146's expression parser is out of scope for `crate::model`, which
/// normalizes and checks a caller-constructed [`DomainPackage`] only and never
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
        field: DeclarationKey,
    },
    /// `self.<field> <operator> <literal>`, e.g. `self.cs <= 5`.
    Comparison {
        /// The field the postcondition relates to `literal`.
        field: DeclarationKey,
        /// The stated ordering between `self.<field>` and `literal`.
        operator: OrderingOperator,
        /// The integer literal `self.<field>` is compared against.
        literal: i64,
    },
}

impl PostconditionClause {
    /// The field this clause is about.
    pub fn field(&self) -> &DeclarationKey {
        match self {
            Self::Presence { field } | Self::Comparison { field, .. } => field,
        }
    }
}

/// An operation member of an object type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationMemberRecord {
    /// This member's own original declaration key.
    pub key: DeclarationKey,
    /// The owning object type's original declaration key (the receiver type).
    pub owner: DeclarationKey,
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
    /// This operation's declared `redefines` (`model-complete.md`:162): the
    /// inherited operation this member redefines, or `None` when it
    /// declares no redefinition. An inline property of the operation itself.
    pub redefines: Option<DeclarationKey>,
}

/// FCD FR-114 component record: FR-152's Part candidate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentRecord {
    /// This component's own original declaration key.
    pub key: DeclarationKey,
    /// The owning composite type's original declaration key
    /// (`owningTypeIdentity`).
    pub owning_type: DeclarationKey,
    /// The declared part type's original declaration key (`typeIdentity`).
    pub value_type: DeclarationKey,
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
    /// This endpoint's own original declaration key.
    pub key: DeclarationKey,
    /// The owning component's original declaration key
    /// (`owningComponentIdentity`).
    pub owning_component: DeclarationKey,
    /// The declared interface type's original declaration key (`typeIdentity`).
    pub value_type: DeclarationKey,
    /// `Some(direction)` when the producer supplies the `port-direction`
    /// capability (FR-152's Port kind requires it); `None` when it does not.
    pub direction: Option<PortDirection>,
    /// The declared multiplicity.
    pub multiplicity: Multiplicity,
}

/// One end of a [`RelationshipRecord`]: `{type_identity, role, multiplicity}`.
/// `type_identity` names whatever the end's `typeIdentity` names in the
/// correspondence — an endpoint, a component, an operation member or an
/// object type — which [`crate::model::systems`] resolves by kind.
///
/// `role` (model-complete.md's Relationships row: "each end has a role and
/// a multiplicity") is `Some` when the producer's own end shape carries one
/// -- a type's inline `relationships[]` entry always does on its source end
/// and may on its target end -- and `None` when the end's own producer
/// shape carries no role concept at all, as a Connection node's
/// `sourceEnd`/`targetEnd` never do (`ConnectionEnd` has no `role` member).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipEnd {
    /// The named producer key.
    pub type_identity: DeclarationKey,
    /// `Some(role)` when the producer's own end shape carries a role;
    /// `None` when it does not.
    pub role: Option<String>,
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

/// FCD FR-115 relationship record: FR-152's Connection candidate, or (when
/// both ends name object types) a navigation-only relationship with no
/// kind. Never an Allocation: an allocation's wire shape names only a
/// source and a target element and carries neither ends' multiplicity nor a
/// direction (`model-complete.md`:335 — see [`AllocationRecord`]), so it is
/// read as one, never folded into this shape with fabricated fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelationshipRecord {
    /// This relationship's own original declaration key.
    pub key: DeclarationKey,
    /// The source end.
    pub source: RelationshipEnd,
    /// The target end.
    pub target: RelationshipEnd,
    /// The producer's `semantics.direction`.
    pub direction: RelationshipDirection,
}

/// FR-152's Allocation candidate (`model-complete.md`:335,
/// `quire.model.systems.allocation/v1`): a source element (a Part, Port or
/// operation) and a target element (a Part), and nothing else. Allocation
/// compatibility has no other rule -- no multiplicity, no direction -- so
/// this record carries none; a caller that needs one is asking a question
/// this declaration kind has no answer to, not receiving a fabricated
/// default.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AllocationRecord {
    /// This allocation's own original declaration key.
    pub key: DeclarationKey,
    /// The source element's original declaration key.
    pub source_element: DeclarationKey,
    /// The target element's original declaration key.
    pub target_element: DeclarationKey,
}

/// FR-153's population declaration extent: `closed` or `open`. Object
/// closure holds exactly when a population's extent is `closed`
/// (FR-153:68); `open` is admission's own `incomplete_population`
/// unknown-closure case for object closure, distinct from
/// [`GeneralizationClosure`](crate::model::dispatch::GeneralizationClosure)'s
/// subtype closure.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Extent {
    /// The population's object closure holds.
    Closed,
    /// The population's object closure does not hold.
    Open,
}

/// An FR-153/FR-208:50 population declaration record: `{key, member_types,
/// extent}`. Lists the population's declared member types and carries one
/// population-level `extent`; a member type carries no multiplicity of its
/// own here — the declared maximum `N` stays on the binding
/// ([`crate::model::population::admit_binding`]'s own `declared_maximum`
/// parameter).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationRecord {
    /// This record's own original declaration key.
    pub key: DeclarationKey,
    /// The population's declared member types' original declaration keys.
    pub member_types: Vec<DeclarationKey>,
    /// The population's declared extent.
    pub extent: Extent,
}

/// One producer record, in the domain package's declared order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainPackageRecord {
    /// An object type export.
    ObjectType(ObjectTypeRecord),
    /// A field member of an object type.
    FieldMember(FieldMemberRecord),
    /// A scalar type export bound to a closed integer interval.
    ScalarType(ScalarTypeRecord),
    /// An operation member of an object type.
    OperationMember(OperationMemberRecord),
    /// FCD FR-114 component (FR-152 Part candidate).
    Component(ComponentRecord),
    /// FCD FR-114 endpoint (FR-152 Port candidate).
    Endpoint(EndpointRecord),
    /// FCD FR-115 relationship (FR-152 Connection candidate, or a
    /// navigation-only relationship).
    Relationship(RelationshipRecord),
    /// FR-152 Allocation declaration.
    Allocation(AllocationRecord),
    /// FR-153 population declaration.
    Population(PopulationRecord),
}

impl DomainPackageRecord {
    /// This record's own original declaration key.
    pub fn key(&self) -> &DeclarationKey {
        match self {
            Self::ObjectType(record) => &record.key,
            Self::FieldMember(record) => &record.key,
            Self::ScalarType(record) => &record.key,
            Self::OperationMember(record) => &record.key,
            Self::Component(record) => &record.key,
            Self::Endpoint(record) => &record.key,
            Self::Relationship(record) => &record.key,
            Self::Allocation(record) => &record.key,
            Self::Population(record) => &record.key,
        }
    }
}

/// A domain package selection: `{identity, version, digest_domain: "sha256-jcs",
/// digest}` (FR-321, `model-complete.md`:50).
#[derive(Clone, Eq, PartialEq)]
pub struct DomainPackageRef {
    /// The domain package's own identity (e.g. `test/orders`).
    pub identity: String,
    /// The domain package's own version.
    pub version: String,
    /// The SHA-256 digest of the domain package's JCS bytes.
    pub digest: [u8; 32],
}

impl std::fmt::Debug for DomainPackageRef {
    /// Prints `digest` as hex, not 32 decimal bytes — readable in refusal
    /// payloads (e.g. `ForeignModelSelection { expected }`) the way the
    /// deleted `ProducerDigest`'s own hex `Debug` was.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainPackageRef")
            .field("identity", &self.identity)
            .field("version", &self.version)
            .field("digest", &super::key::hex(&self.digest))
            .finish()
    }
}

impl DomainPackageRef {
    pub(super) fn to_json(&self) -> serde_json::Value {
        use serde_json::{Map, Value};
        let mut object = Map::new();
        object.insert("identity".to_owned(), Value::String(self.identity.clone()));
        object.insert("version".to_owned(), Value::String(self.version.clone()));
        object.insert(
            "digest_domain".to_owned(),
            Value::String(super::key::SHA256_JCS_DIGEST_DOMAIN.to_owned()),
        );
        object.insert(
            "digest".to_owned(),
            Value::String(super::key::hex(&self.digest)),
        );
        Value::Object(object)
    }

    /// A `test/orders` version-`1` selection whose digest is the SHA-256 of
    /// `placeholder`'s exact UTF-8 bytes, following TC-195's own placeholder
    /// selection convention: F1 selects `n01`, F2 selects `n02`, and F1 as
    /// version `2` selects `n01v2`.
    ///
    /// Test-only (PR #140 F13): this derives a digest from a display
    /// identity, which is exactly the name-derived-identity defect this
    /// engine exists to exclude. Gated behind `test-support` so a production
    /// caller cannot reach it; `cargo test --all-features` enables it for
    /// `tests/model_normalization.rs`.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(placeholder: impl Into<String>) -> Self {
        Self::fixture_with_version(placeholder, "1")
    }

    /// [`Self::fixture`], with an explicit `version` rather than the
    /// hard-wired `"1"` — needed to reach F1 as version `2` under
    /// placeholder `n01v2` (TC-195 N09's third clause).
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_with_version(
        placeholder: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        use sha2::{Digest, Sha256};
        Self {
            identity: "test/orders".to_owned(),
            version: version.into(),
            digest: Sha256::digest(placeholder.into().as_bytes()).into(),
        }
    }
}

/// A domain package: a [`DomainPackageRef`] and its ordered records.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DomainPackage {
    /// This domain package's model selection.
    pub model_selection: DomainPackageRef,
    /// The domain package's records, in the producer's declared order.
    pub records: Vec<DomainPackageRecord>,
}

impl DomainPackage {
    /// A domain package over `records`, selected under `model_selection`.
    pub fn new(model_selection: DomainPackageRef, records: Vec<DomainPackageRecord>) -> Self {
        Self {
            model_selection,
            records,
        }
    }
}

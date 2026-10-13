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

use std::collections::BTreeMap;

use crate::model::key::DeclarationKey;
use crate::model::normalize::ModelRefusal;
use crate::model::origin::JsonValue;
use quire_canonical::FixedShape;
use quire_exact::{OrderingOperator, Presence};

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

/// A record value type (FR-208 `quire.meaning.model.record-value-type/v1`):
/// a named record of one or more fields, none an identity field. Its values
/// have no object identity: two are equal exactly when they have one
/// most-specific type and every field is equal. Its fields are
/// [`FieldMemberRecord`]s owned by this type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecordValueTypeRecord {
    /// This type's own original declaration key.
    pub key: DeclarationKey,
    /// Authored identity fields, retained until the charged FR-208 binder
    /// rejects this record meaning. A record value type has no identity field.
    pub identity_fields: Vec<DeclarationKey>,
    /// Declared ancestors of this same business meaning.
    pub supertypes: Vec<DeclarationKey>,
}

/// An event type (FR-208): an identity-free type whose occurrence field
/// resolves to native Timestamp. Event subtypes retain the occurrence field
/// and other effective members through their authored `supertypes`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventTypeRecord {
    /// This type's own original declaration key.
    pub key: DeclarationKey,
    /// Authored identity fields, retained so normalization can refuse them
    /// at the business-meaning boundary rather than erasing the input.
    pub identity_fields: Vec<DeclarationKey>,
    /// The authored occurrence field's original declaration key.
    pub occurrence_field: Option<DeclarationKey>,
    /// Declared event-type ancestors.
    pub supertypes: Vec<DeclarationKey>,
}

/// A state declared by an FR-208 state machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateMachineStateRecord {
    /// The state's original declaration key.
    pub key: DeclarationKey,
    /// The authored local state name.
    pub name: String,
}

/// A transition declared by an FR-208 state machine.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateMachineTransitionRecord {
    /// The transition's original declaration key.
    pub key: DeclarationKey,
    /// The transition's source state.
    pub from: DeclarationKey,
    /// The transition's target state.
    pub to: DeclarationKey,
    /// The operation serving as its trigger.
    pub trigger: DeclarationKey,
    /// The optional authored guard clause id.
    pub guard: Option<String>,
    /// Emitted event-type identities in authored order.
    pub emits: Vec<DeclarationKey>,
    /// Original JSON document pointer to this transition.
    pub path: String,
    /// The transition's exact FCD origin value.
    pub origin: serde_json::Value,
}

/// A state-machine meaning (FR-208). Its fields, operations, and clauses
/// remain ordinary owner-linked records alongside this type.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateMachineRecord {
    /// This type's original declaration key.
    pub key: DeclarationKey,
    /// Declared state-machine ancestors.
    pub supertypes: Vec<DeclarationKey>,
    /// States in source order.
    pub states: Vec<StateMachineStateRecord>,
    /// Transitions in source order.
    pub transitions: Vec<StateMachineTransitionRecord>,
}

/// One process step, retained as structural process data rather than a
/// declaration/member in the domain package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessStepRecord {
    /// The step's authored identity.
    pub key: DeclarationKey,
    /// The authored step name.
    pub name: String,
    /// The closed process step-kind vocabulary.
    pub step_kind: ProcessStepKind,
    /// Event types consumed by this step.
    pub consumes: Vec<DeclarationKey>,
    /// Event types emitted by this step.
    pub emits: Vec<DeclarationKey>,
}

/// The closed step-kind vocabulary defined by FR-208.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProcessStepKind {
    /// A command step.
    Command,
    /// An event step.
    Event,
    /// A decision step.
    Decision,
    /// A compensation step.
    Compensation,
    /// A wait step.
    Wait,
}

/// A process declaration: an identified run type and its ordered steps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProcessRecord {
    /// The original process declaration key.
    pub key: DeclarationKey,
    /// The process correlation key fields.
    pub identity_fields: Vec<DeclarationKey>,
    /// Whether the source supplied `identityFields`. A subtype may supply an
    /// empty list to inherit its process identity; omission remains a
    /// normalization diagnostic.
    pub identity_fields_declared: bool,
    /// Declared process ancestors.
    pub supertypes: Vec<DeclarationKey>,
    /// Authored steps, preserving declaration order.
    pub steps: Vec<ProcessStepRecord>,
}

/// An authored clause member. Its language and text are retained verbatim;
/// retaining a clause does not parse or evaluate it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseRecord {
    /// The clause's original member key.
    pub key: DeclarationKey,
    /// The original declaration owning this clause.
    pub owner: DeclarationKey,
    /// The authored language identifier.
    pub language: String,
    /// The exact authored expression text.
    pub text: String,
}

/// One authored namespace vocabulary entry, in declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamespaceTerm {
    /// The authored term.
    pub term: String,
    /// Its authored documentation.
    pub doc: String,
    /// The exact FCD origin, including generated or source provenance.
    pub origin: serde_json::Value,
}

/// A namespace is one original declaration, never an effective type or an
/// object universe. Membership and vocabulary are data, not declarations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NamespaceRecord {
    /// The original namespace node key.
    pub key: DeclarationKey,
    /// Declared members, preserving authored order.
    pub members: Vec<DeclarationKey>,
    /// Declared vocabulary, preserving authored order.
    pub vocabulary: Vec<NamespaceTerm>,
}

/// A persistence interface declaration: the original storage contract and
/// the object types whose values it persists. Its operation members remain
/// ordinary [`OperationMemberRecord`]s owned by this key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PersistenceInterfaceRecord {
    /// The original persistence-interface declaration key.
    pub key: DeclarationKey,
    /// The object types named by the declaration's `persists` list.
    pub persists: Vec<DeclarationKey>,
}

/// QSL's own closed native value-type vocabulary: the unparameterized
/// value-type keywords a `typeRef` may name directly, under
/// `ix://quire/native/<Name>`, without naming a node of any package. It
/// includes shared-grammar.md's keywords plus `Timestamp`, required by
/// FR-056-AC-17/19. `Boolean`, `Integer`, `Float32`, `Float64` and
/// `Timestamp` take no parameters. `Integer`'s own optional inline bound
/// (`Int[lo,hi]`) is a constraint fact about the field, not a distinct
/// value-type reference. `Rational`, `Decimal` and `Text` are parameterized
/// in the grammar (`Rational[lo,hi;dmin,dmax]`, `Decimal[lo,hi;dmin,dmax]`,
/// `Text[min,max;profile]`) and REQUIRE their full parameter set to resolve
/// (see [`Self::unsupported_parameters`]).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NativeValueType {
    /// `UUID`.
    Uuid,
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
    /// `String`.
    String,
    /// `Timestamp`.
    Timestamp,
}

impl NativeValueType {
    /// The `<Name>` past `ix://quire/native/`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Uuid => "UUID",
            Self::Boolean => "Boolean",
            Self::Integer => "Integer",
            Self::Rational => "Rational",
            Self::Decimal => "Decimal",
            Self::Float32 => "Float32",
            Self::Float64 => "Float64",
            Self::Text => "Text",
            Self::String => "String",
            Self::Timestamp => "Timestamp",
        }
    }

    /// The required parameter names (shared-grammar.md's own labels for
    /// each production: `Rational`/`Decimal`'s `dmin`/`dmax`, `Text`'s
    /// `profile`) that `agent-ix-semantic-ir`'s own constraint vocabulary
    /// (`constraint_schema`'s closed `keyword` set: `min`, `max`,
    /// `exclusiveMin`, `exclusiveMax`, `minLength`, `maxLength`, `pattern`,
    /// `enumValues`, `nonEmpty`, `unique`, `format`) has no keyword for these
    /// parameter sets, so `Rational`, `Decimal` and `Text` cannot resolve no
    /// matter what a producer's own document supplies. `Rational`/`Decimal`
    /// also require `lo`/`hi`, which the vocabulary's `min`/`max` do carry;
    /// only the decimal-scale bound is unexpressable. `Boolean`, `Integer`,
    /// `Float32`, `Float64` and `Timestamp` take no required parameter, so this
    /// is empty for them.
    pub fn unsupported_parameters(&self) -> &'static [&'static str] {
        match self {
            Self::Uuid
            | Self::Boolean
            | Self::Integer
            | Self::Float32
            | Self::Float64
            | Self::String
            | Self::Timestamp => &[],
            Self::Rational | Self::Decimal => &["dmin", "dmax"],
            Self::Text => &["profile"],
        }
    }
}

impl std::str::FromStr for NativeValueType {
    type Err = ();

    #[qsl_attrs::string_edge]
    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "UUID" => Ok(Self::Uuid),
            "Boolean" => Ok(Self::Boolean),
            "Integer" => Ok(Self::Integer),
            "Rational" => Ok(Self::Rational),
            "Decimal" => Ok(Self::Decimal),
            "Float32" => Ok(Self::Float32),
            "Float64" => Ok(Self::Float64),
            "Text" => Ok(Self::Text),
            "String" => Ok(Self::String),
            "Timestamp" => Ok(Self::Timestamp),
            _ => Err(()),
        }
    }
}

/// A field, parameter or result's resolved value type: QSL's own closed
/// native vocabulary, declaring no node of any package, or a node of some
/// package. The sole output shape [`crate::model::intake`]'s unified
/// `typeRef` resolver produces -- never a synthetic `DeclarationKey` under a
/// made-up "quire/native" package.
///
/// quire:canonical
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
/// presence, subsets, redefines}`.
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
    /// This field's declared `presence` (`model-complete.md`'s Presence
    /// row): `Required` when every object carries the field, `Optional`
    /// when it may be absent. Independent of `multiplicity`'s lower bound --
    /// a lower bound of `0` makes an empty collection legal and never makes
    /// a field optional; only this carries that.
    pub presence: Presence,
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

/// The value domain carried by a scalar type declaration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScalarValueDomain {
    /// A closed integer interval (FR-151's `model.Count`/`model.Small`
    /// fixtures).
    Integer {
        /// Inclusive lower bound.
        lower: i128,
        /// Inclusive upper bound.
        upper: i128,
    },
    /// An alias for a native value type.
    Native(NativeValueType),
}

/// A scalar type export. Integer bounds support FR-146 interval containment;
/// native aliases preserve the target's distinct value-type identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScalarTypeRecord {
    /// This type's own original declaration key.
    pub key: DeclarationKey,
    /// This type's value domain.
    pub domain: ScalarValueDomain,
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
/// (`crate::check::check_field_refinement_obligation`, moved from
/// `crate::model::conformance` by FR-074/ADR-011 §7.3 M-2) rebuilds
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
        literal: i128,
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

/// An operation member of an object type, state machine, or persistence interface.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationMemberRecord {
    /// This member's own original declaration key.
    pub key: DeclarationKey,
    /// The owning declaration's original key (the receiver type when it has one).
    pub owner: DeclarationKey,
    /// Declared parameters, in signature order.
    pub parameters: Vec<OperationParameterRecord>,
    /// Declared result, or `None` when the operation has no result.
    pub result: Option<OperationResult>,
    /// Declared effect frame.
    pub effect: OperationEffect,
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

/// A variant type's `enum` export (FR-056 "Variant types"): the type's
/// declaration, its artifact id as `name`, and its case names as the
/// unordered set `members`, kept in ascending UTF-8 byte order of the name so
/// the document's `variants` order changes no record. The export locus is the
/// type's original FCD origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumRecord {
    /// The variant type's own original declaration key; also the `enum`
    /// export key.
    pub key: DeclarationKey,
    /// The type's artifact id.
    pub name: String,
    /// The declared case names, ascending by UTF-8 bytes.
    pub members: Vec<String>,
}

/// One case of a variant type: the `variant` export (FR-056 "Variant types").
///
/// The record's [`key`](Self::key) is the case node's own declaration
/// identity (`ix://<package>/variant/<Name>-<value>`, FR-154's variant slot),
/// which is the declaration's key. The `variant` export is keyed instead by
/// [`export_key`](Self::export_key), the owner's identity and the declared
/// case name. The export locus is the case's original FCD origin.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VariantRecord {
    /// The case node's own original declaration key.
    pub key: DeclarationKey,
    /// The owning variant type; also the owner `enum` export key.
    pub owner: DeclarationKey,
    /// The declared case name, compared as exact UTF-8 bytes.
    pub case: String,
}

impl VariantRecord {
    /// The `variant` export key: the owner's declaration identity and the
    /// declared case name. Never the case node's minted identity.
    pub fn export_key(&self) -> (&DeclarationKey, &str) {
        (&self.owner, &self.case)
    }
}

/// One producer record, in the domain package's declared order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DomainPackageRecord {
    /// A variant type's `enum` export (FR-056 "Variant types").
    Enum(EnumRecord),
    /// One case of a variant type: a `variant` export.
    Variant(VariantRecord),
    /// An object type export.
    ObjectType(ObjectTypeRecord),
    /// A field member of an object type or record value type.
    FieldMember(FieldMemberRecord),
    /// A record value type (FR-208).
    RecordValueType(RecordValueTypeRecord),
    /// An event type (FR-208).
    EventType(EventTypeRecord),
    /// A state machine (FR-208).
    StateMachine(StateMachineRecord),
    /// A process type (FR-208).
    Process(ProcessRecord),
    /// An authored clause member (FR-208).
    Clause(ClauseRecord),
    /// An original namespace with no effective type (FR-208).
    Namespace(NamespaceRecord),
    /// An original persistence interface with no effective type (FR-208).
    PersistenceInterface(PersistenceInterfaceRecord),
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
            Self::Enum(record) => &record.key,
            Self::Variant(record) => &record.key,
            Self::ObjectType(record) => &record.key,
            Self::FieldMember(record) => &record.key,
            Self::RecordValueType(record) => &record.key,
            Self::EventType(record) => &record.key,
            Self::StateMachine(record) => &record.key,
            Self::Process(record) => &record.key,
            Self::Clause(record) => &record.key,
            Self::Namespace(record) => &record.key,
            Self::PersistenceInterface(record) => &record.key,
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
///
/// quire:canonical
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

/// A [`DomainPackageRef`]'s preimage form inside the `quire.model.*` identities.
#[derive(serde::Serialize, FixedShape)]
pub(super) struct DomainPackageRefWire<'a> {
    identity: &'a str,
    digest_domain: &'static str,
    digest: String,
}

impl DomainPackageRef {
    /// This selection's FR-321 preimage form: `{identity, digest_domain,
    /// digest}`, the digest lowercase hex.
    pub(super) fn wire(&self) -> DomainPackageRefWire<'_> {
        DomainPackageRefWire {
            identity: &self.identity,
            digest_domain: super::key::SHA256_JCS_DIGEST_DOMAIN,
            digest: super::key::hex(&self.digest),
        }
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
        Self {
            identity: "test/orders".to_owned(),
            version: version.into(),
            digest: qsl_foundation::ByteDigest::of(placeholder.into().as_bytes()).as_bytes(),
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
    /// Original intake provenance, separate from normalized identities.
    pub(crate) originals: BTreeMap<DeclarationKey, OriginalDeclaration>,
    /// FR-154 intake refusals retained until normalization has charged every
    /// declaration in the source package. Synthetic packages carry none.
    pub(crate) intake_refusals: Vec<ModelRefusal>,
    /// Source declaration count before a refusing node can be omitted from
    /// the partial typed records. `None` means use `records.len()`.
    pub(crate) declared_record_count: Option<usize>,
}

/// An original declaration's authored provenance. The FCD origin is
/// retained whole: generated origins have no source span, and source
/// origins retain their path and optional end coordinates unchanged.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OriginalDeclaration {
    /// The exact construct meaning for a type or population. Members carry
    /// no meaning id under QSpec FR-208.
    pub meaning: Option<String>,
    /// The original FCD origin object, without fabricated byte positions.
    pub origin: JsonValue,
    /// A member's declared name. Type and population nodes have none.
    pub member_name: Option<String>,
}

impl DomainPackage {
    /// A synthetic domain package over `records`, selected under
    /// `model_selection`. This constructor receives no source document and
    /// supplies no authored provenance. Intake callers use
    /// [`crate::model::intake::read_domain_package`] to retain it.
    pub fn new(model_selection: DomainPackageRef, records: Vec<DomainPackageRecord>) -> Self {
        Self {
            model_selection,
            records,
            originals: BTreeMap::new(),
            intake_refusals: Vec::new(),
            declared_record_count: None,
        }
    }

    /// The original intake provenance of `key`, or none for a synthetic
    /// record that was constructed without a source document.
    pub fn original(&self, key: &DeclarationKey) -> Option<&OriginalDeclaration> {
        self.originals.get(key)
    }
}

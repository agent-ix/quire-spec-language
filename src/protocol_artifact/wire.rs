// SPDX-License-Identifier: AGPL-3.0-only
//! FR-042: closed transport records in the contract's canonical member order.
//!
//! These records carry untrusted claims. Deserializing or constructing one does
//! not establish source correspondence, family admission or an accepted artifact.

use super::{NumberError, NumberWire, ProtocolNumber};
use crate::serde_object::from_object;
use crate::ByteDigest;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};

// Unlike a derived struct's default decoder, every record below requires an
// object even after Serde buffers it inside an internally tagged alternative.
macro_rules! record {
    ($(#[$meta:meta])* $name:ident { $($(#[$field_meta:meta])* $field:ident: $ty:ty),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
        pub struct $name { $($(#[$field_meta])* pub $field: $ty),* }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                #[derive(Deserialize)]
                #[serde(deny_unknown_fields)]
                struct Fields { $($(#[$field_meta])* $field: $ty),* }
                let Fields { $($field),* } = from_object(decoder)?;
                Ok(Self { $($field),* })
            }
        }
    };
}

// Empty alternatives use struct variants, so deny_unknown_fields applies there
// too; internally tagged unit variants otherwise accept extra members.
macro_rules! tagged {
    ($(#[$meta:meta])* $name:ident {
        $($variant:ident => $tag:literal { $($(#[$field_meta:meta])* $field:ident: $ty:ty),* $(,)? }),* $(,)?
    }) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
        #[serde(tag = "kind")]
        pub enum $name {
            $(#[serde(rename = $tag)] $variant { $($(#[$field_meta])* $field: $ty),* }),*
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                #[derive(Deserialize)]
                #[serde(tag = "kind", deny_unknown_fields)]
                enum Fields {
                    $(#[serde(rename = $tag)] $variant { $($(#[$field_meta])* $field: $ty),* }),*
                }
                Ok(match from_object(decoder)? {
                    $(Fields::$variant { $($field),* } => Self::$variant { $($field),* }),*
                })
            }
        }
    };
}

macro_rules! labels {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $wire:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize)]
        pub enum $name { $(#[serde(rename = $wire)] $variant),* }
        impl $name {
            /// Exact closed spelling, also used for lexical wire ordering.
            pub const fn as_str(self) -> &'static str {
                match self { $(Self::$variant => $wire),* }
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
                let value = String::deserialize(decoder)?;
                match value.as_str() {
                    $($wire => Ok(Self::$variant),)*
                    other => Err(de::Error::unknown_variant(other, &[$($wire),*])),
                }
            }
        }
    };
}

/// A required member whose value may be explicitly null; omission still refuses.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(transparent)]
pub struct Nullable<T>(pub Option<T>);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Nullable<T> {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        struct Required<T>(std::marker::PhantomData<T>);
        impl<'de, T: Deserialize<'de>> de::Visitor<'de> for Required<T> {
            type Value = Nullable<T>;
            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("an explicitly present value or null")
            }
            fn visit_newtype_struct<D: Deserializer<'de>>(
                self,
                decoder: D,
            ) -> Result<Self::Value, D::Error> {
                Option::<T>::deserialize(decoder).map(Nullable)
            }
        }
        // Serde's missing-field decoder cannot silently synthesize this newtype.
        decoder.deserialize_newtype_struct("RequiredNullable", Required(std::marker::PhantomData))
    }
}

/// Offered exact-number fields; only checked() invokes the existing FR-038 codec.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Number(pub NumberWire);

impl Number {
    /// Validate canonical spelling and exact components, without repairing them.
    pub fn checked(&self) -> Result<ProtocolNumber, NumberError> {
        ProtocolNumber::try_from(&self.0)
    }
}
impl Serialize for Number {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.checked()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

/// An integer-only position retains wrong offered kinds for typed admission errors.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(transparent)]
pub struct Integer(pub NumberWire);

impl Integer {
    /// Validate numeric components; the artifact validator also requires Integer.
    pub fn checked(&self) -> Result<ProtocolNumber, NumberError> {
        ProtocolNumber::try_from(&self.0)
    }
}
impl Serialize for Integer {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self.checked().map_err(serde::ser::Error::custom)? {
            value @ ProtocolNumber::Integer(_) => value.serialize(serializer),
            ProtocolNumber::Rational(_) => {
                Err(serde::ser::Error::custom("expected integer numeric kind"))
            }
        }
    }
}

// Exact shared raw-byte digest spelling; no new digest identity or algorithm.
mod digest {
    use super::*;
    pub(super) fn serialize<S: Serializer>(
        value: &ByteDigest,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(value)
    }
    pub(super) fn deserialize<'de, D: Deserializer<'de>>(
        decoder: D,
    ) -> Result<ByteDigest, D::Error> {
        String::deserialize(decoder)?
            .parse()
            .map_err(de::Error::custom)
    }
}

labels! {
    /// Shared ArtifactRef/3 kinds used by compiled and assessment boundaries.
    ArtifactKind {
        Binding => "binding", DependencyClosure => "dependency-closure", Environment => "environment",
        ExecutableProjection => "executable-projection", FaultModel => "fault-model",
        GeneratedArtifact => "generated-artifact", Invocation => "invocation", LinkedPackage => "linked-package",
        ModelLock => "model-lock", ModelManifest => "model-manifest", ModelPackage => "model-package",
        Observation => "observation", Oracle => "oracle", Population => "population", Property => "property",
        ReviewDisposition => "review-disposition", ReviewProcedure => "review-procedure",
        RunArtifact => "run-artifact", Snapshot => "snapshot", Source => "source", Trace => "trace",
        Window => "window"
    }
}
labels! {
    /// Expected producer export kind; a tag alone never supplies that authority.
    ExportKind {
        Component => "component", Endpoint => "endpoint", Enum => "enum", Field => "field",
        Object => "object", Operation => "operation", Population => "population", Record => "record",
        Reference => "reference", Relationship => "relationship", Scalar => "scalar", Variant => "variant"
    }
}

record! { /// An opaque revision within its explicitly selected namespace.
    Revision { namespace: String, value: String }
}
record! { /// Wire identity/version, separate from semantic definition revision.
    Wire { identity: String, version: String }
}
record! { /// Narrow representation of the exact shared ArtifactRef/3 object.
    ArtifactRef {
        #[serde(rename = "refVersion")] ref_version: String,
        kind: ArtifactKind, authority: String, identity: String, revision: Revision,
        #[serde(with = "digest")] digest: ByteDigest,
        wire: Wire
    }
}
record! { /// Explicit formal document owner; native labels cannot substitute for it.
    Formal { document: String, revision: Revision }
}
record! { /// Original decoded UTF-8 byte coordinates, before boundary validation.
    Span { start: u32, end: u32 }
}
record! { /// Index into the package's original source table and its byte region.
    Locus { source: u32, span: Span }
}
record! { /// Independently supplied producer source and authored formal coordinates.
    ForeignLocus { source: ArtifactRef, formal: Formal, span: Span }
}
record! { /// A declaration-owned table index; its expected table follows the field.
    Handle { declaration: u32, index: u32 }
}
record! { /// Exact selected producer digest domain, never implicitly rehashed here.
    SelectedDigest { domain: String, version: String, algorithm: String, value: String }
}
record! { /// Selected implementation and binary artifact, not an authenticity claim.
    Producer { implementation: String, revision: Revision, binary: ArtifactRef }
}
record! { /// One immutable dependency and its own dependency-table prerequisites.
    Dependency { artifact: ArtifactRef, requires: Vec<u32> }
}
record! { /// Selected definition source, required rule sources and definition closure.
    Definition { identity: String, revision: Revision, artifact: u32, rules: Vec<u32>, requires: Vec<u32> }
}
record! { /// Original producer-domain identity, retaining its selected interface.
    ProducerObject { interface: u32, kind: String, authority: String, identity: String, revision: Revision, digest: SelectedDigest }
}
record! { /// Claims checked against the exact separately supplied relation artifact.
    Correspondence { producer: ProducerObject, native: u32, relation: u32, exports: Vec<u32> }
}
record! { /// Located producer export, whose kind/path must exist in its admitted view.
    Export { kind: ExportKind, path: Vec<String>, locus: ForeignLocus }
}
record! { /// Selected model artifact and exports, with explicit nullable correspondence.
    Model { artifact: u32, profile: String, exports: Vec<Export>, correspondence: Nullable<Correspondence> }
}
record! { /// Editable native authority labels; these are not formal revisions.
    NativeSource { identity: String, revision: String }
}
record! { /// Original source text, retained for hashing/loci and never reparsed here.
    Source { artifact: ArtifactRef, native: NativeSource, path: String, formal: Formal, text: String }
}
record! { /// Closed capabilities are validated independently from semantic definitions.
    Features { declarations: Vec<String>, required: Vec<String>, optional: Vec<String> }
}
record! { /// Explicit native language/edition selection.
    Language { identity: String, edition: String }
}
record! { /// Untrusted complete payload in its required canonical member order.
    Package {
        wire: String, media: String, schema: String, #[serde(rename = "type")] package_type: String, encoding: String, numeric: String,
        contract: ArtifactRef, producer: Producer, baseline: ArtifactRef, language: Language,
        package_definition: u32, features: Features, sources: Vec<Source>, dependencies: Vec<Dependency>,
        definitions: Vec<Definition>, models: Vec<Model>, types: Vec<Type>, declarations: Vec<Declaration>
    }
}

record! { /// Exact model/export indices; equal spelling never merges nominal owners.
    ExportRef { model: u32, export: u32 }
}

tagged! {
    /// Resolved native type claims, preserving exact exports, units and wrappers.
    Type {
        Boolean => "boolean" {},
        Scalar => "scalar" { export: ExportRef, unit: Nullable<String>, representation: Representation },
        Enum => "enum" { export: ExportRef },
        Record => "record" { export: ExportRef },
        Object => "object" { export: ExportRef },
        Reference => "reference" { export: ExportRef, object: ExportRef, universe: ExportRef },
        Option => "option" { value: u32 },
        Sequence => "sequence" { element: u32, maximum: Integer }
    }
}
tagged! {
    /// Native authored bounds encoded only through the exact integer alternative.
    Representation {
        Integer => "integer" { minimum: Integer, maximum: Integer },
        Rational => "rational" { numerator_minimum: Integer, numerator_maximum: Integer, maximum_denominator: Integer },
        Text => "text" { maximum_scalars: Integer }
    }
}
labels! {
    /// Static observation/activation roles; concrete observations are absent.
    AnchorKind {
        Predicate => "predicate", Current => "current", InvocationInput => "invocation_input",
        InvocationPre => "invocation_pre", InvocationPost => "invocation_post", Activation => "activation",
        TemporalInstant => "temporal_instant", ProtocolInstant => "protocol_instant", Fifo => "fifo",
        Registration => "registration", CompensationActivation => "compensation_activation", Retry => "retry",
        Recovery => "recovery", Control => "control", Finish => "finish"
    }
}
labels! {
    /// Binder role remains separate from its type, scope and immutable anchor.
    BinderKind {
        Parameter => "parameter", Input => "input", Trigger => "trigger", Capture => "capture",
        Let => "let", Query => "query", SelfValue => "self", Result => "result",
        InvocationParameter => "invocation_parameter", Event => "event", Finish => "finish", Fifo => "fifo",
        ForwardEffect => "forward_effect", CompensationTrigger => "compensation_trigger",
        EarlierAttempt => "earlier_attempt", LaterAttempt => "later_attempt", Recovery => "recovery"
    }
}
record! { /// Declaration-local lexical nesting and its original source region.
    Scope { parent: Nullable<Handle>, locus: Locus }
}
record! { /// Owner and runtime-requirement indices have role-specific interpretations.
    Anchor { kind: AnchorKind, owner: Nullable<Handle>, binding: Nullable<u32>, locus: Locus }
}
record! { /// Immutable lexical value with explicit initialization and source owner.
    Binder { name: String, kind: BinderKind, #[serde(rename = "type")] value_type: u32, scope: Handle, anchor: Handle, initializer: Nullable<Handle>, locus: Locus }
}
tagged! {
    /// The retained value origin, separate from the surrounding evaluation anchor.
    Origin {
        Independent => "independent" {},
        Anchor => "anchor" { anchor: Handle },
        Selected => "selected" { value: Handle }
    }
}
labels! {
    /// Only these native unary operations have a value-graph representation.
    Unary { Not => "not", Negate => "negate", Present => "present", Value => "value", Deref => "deref" }
}
labels! {
    /// Closed binary operations; integer division and remainder are absent.
    Binary {
        Implies => "implies", Or => "or", And => "and", Equal => "equal", NotEqual => "not_equal",
        Less => "less", LessEqual => "less_equal", Greater => "greater", GreaterEqual => "greater_equal",
        Add => "add", Subtract => "subtract", Multiply => "multiply", RationalDivide => "rational_divide"
    }
}
labels! {
    /// Ordered-sequence queries preserve binders and duplicates.
    Query { ForAll => "forall", Exists => "exists", Filter => "filter", Map => "map", Count => "count", Sum => "sum" }
}
record! { /// Original value occurrence and selected type/profile/lexical provenance.
    Value {
        original_expression: u32, locus: Locus, operator_locus: Nullable<Locus>, #[serde(rename = "type")] value_type: u32,
        profile: u32, scope: Handle, anchor: Handle, origin: Origin, operation: ValueOperation
    }
}
tagged! {
    /// Native evaluation graph claims, never the prover's Boolean abstraction.
    ValueOperation {
        Boolean => "boolean" { value: bool },
        Number => "number" { value: Number },
        Text => "text" { value: String },
        Enum => "enum" { variant: ExportRef },
        Read => "read" { binder: Handle },
        Group => "group" { value: Handle },
        Field => "field" { base: Handle, field: ExportRef },
        Unary => "unary" { operator: Unary, value: Handle },
        Binary => "binary" { operator: Binary, left: Handle, right: Handle },
        If => "if" { condition: Handle, then_value: Handle, else_value: Handle },
        Let => "let" { binder: Handle, initializer: Handle, body: Handle },
        Pre => "pre" { value: Handle, anchor: Handle },
        Call => "call" { predicate: u32, arguments: Vec<Handle> },
        Size => "size" { collection: Handle, result: u32 },
        Contains => "contains" { collection: Handle, member: Handle },
        Query => "query" { operator: Query, binder: Handle, collection: Handle, body: Handle, result: u32 },
        Parent => "parent" { reference: Handle, edge: ExportRef, universe: ExportRef },
        Reaches => "reaches" { start: Handle, target: Handle, edge: ExportRef, universe: ExportRef }
    }
}

record! { /// Authored requirement identity under its separate revision namespace.
    Requirement { package: String, identity: String, revision: Revision }
}
record! { /// Complete declaration-owned tables, with explicit authored clause mapping.
    Declaration {
        name: String, locus: Locus, requirement: Requirement, clause: String, execution: Execution,
        profile: u32, requires: Vec<u32>, scopes: Vec<Scope>, anchors: Vec<Anchor>, binders: Vec<Binder>,
        values: Vec<Value>, temporal: Vec<Temporal>, bindings: Vec<BindingRequirement>, body: Body
    }
}
tagged! {
    /// Authored execution selection; operation alternatives select exact exports.
    Execution {
        Initialization => "initialization" { name: String },
        Handler => "handler" { name: String },
        Pre => "pre" { operation: ExportRef },
        Post => "post" { operation: ExportRef }
    }
}
labels! {
    /// State clause kind in this wire, distinct from a proof or runtime result.
    ClauseKind { Invariant => "invariant", Pre => "pre", Post => "post" }
}
tagged! {
    /// Closed native declaration families; every local handle keeps its owner.
    Body {
        Predicate => "predicate" { parameters: Vec<Handle>, result: u32, root: Handle },
        State => "state" { clause_kind: ClauseKind, context: ExportRef, operation: Nullable<ExportRef>, root: Handle },
        Temporal => "temporal" { input: Handle, clock: u32, activation: Activation, captures: Vec<Handle>, root: Handle },
        Protocol => "protocol" {
            input: Handle, activation: Activation, captures: Vec<Handle>, roles: Vec<Role>,
            relationships: Vec<Relationship>, channels: Vec<Channel>, compensations: Vec<Compensation>,
            temporal_requirements: Vec<u32>, controls: Vec<Control>, causal_edges: Vec<CausalEdge>, run: Handle, finish: Box<Finish>
        }
    }
}
tagged! {
    /// Declaration activation and explicit immutable capture origin.
    Activation {
        Origin => "origin" { anchor: Handle },
        Each => "each" { trigger: Handle, guard: Nullable<Handle>, anchor: Handle }
    }
}
record! { /// Authored closed interval in the selected clock/range domain.
    Interval { lower: Integer, upper: Integer }
}
record! { /// Original temporal occurrence; values and temporal truth remain distinct.
    Temporal { original_node: u32, locus: Locus, operation: TemporalOperation }
}
labels! {
    /// Unary temporal operators; interval presence follows the selected operator.
    TemporalUnary { Not => "not", Eventually => "eventually", Always => "always", Once => "once", Historically => "historically" }
}
labels! {
    /// Binary temporal operators over an explicitly selected temporal definition.
    TemporalBinary { Implies => "implies", Or => "or", And => "and", Until => "until", Release => "release", Since => "since", Triggered => "triggered" }
}
tagged! {
    /// Closed temporal syntax-derived graph, with exact clock-bound intervals.
    TemporalOperation {
        Constant => "constant" { value: bool },
        Holds => "holds" { value: Handle },
        Group => "group" { value: Handle },
        Unary => "unary" { operator: TemporalUnary, interval: Nullable<Interval>, value: Handle },
        Binary => "binary" { operator: TemporalBinary, interval: Nullable<Interval>, left: Handle, right: Handle }
    }
}

record! { /// Participant responsibility with a distinct later role-instance binding.
    Role { name: String, model: ExportRef, instance: u32, locus: Locus }
}
record! { /// An authoritative relationship export and its later binding requirement.
    Relationship { name: String, model: ExportRef, binding: u32, locus: Locus }
}
record! { /// Message/delivery requirements remain distinct from operation effects.
    Channel {
        name: String, from: Handle, to: Handle, message_type: u32, ordering: Ordering, delivery: Interval,
        message: u32, send: u32, receive: u32, delivery_instance: u32, locus: Locus
    }
}
tagged! {
    /// Per-channel ordering; FIFO does not create cross-channel causality.
    Ordering {
        Unordered => "unordered" {},
        Fifo => "fifo" { binder: Handle, key: Handle, anchor: Handle }
    }
}
record! { /// Original protocol control occurrence, before graph/family verification.
    Control { name: String, original_node: u32, locus: Locus, operation: ControlOperation }
}
record! { /// Authored choice alternative, preserving guard and body order.
    ChoiceCase { label: String, guard: Handle, body: Handle, locus: Locus }
}
record! { /// One parallel branch; its table ordinal supplies a join index.
    Branch { label: String, body: Handle, locus: Locus }
}
tagged! {
    /// Await may start from an event or a compensation activation, with distinct tables.
    AwaitAnchor {
        Event => "event" { node: Handle },
        Compensation => "compensation" { compensation: Handle }
    }
}
tagged! {
    /// Closed finite control structures; structural validation derives their edges.
    ControlOperation {
        Sequence => "sequence" { children: Vec<Handle> },
        Choice => "choice" { owner: Handle, visible: Vec<Handle>, cases: Vec<ChoiceCase> },
        Parallel => "parallel" { branches: Vec<Branch>, join: Vec<u32> },
        Repeat => "repeat" { owner: Handle, visible: Vec<Handle>, maximum: Integer, guard: Handle, body: Handle, exhausted: Handle },
        Await => "await" { after: AwaitAnchor, profile: u32, clock: u32, within: Interval, event: Handle, then_body: Handle, timeout: Handle },
        Event => "event" { event: Event, binder: Handle, related: Vec<Related>, constraint: Handle },
        Check => "check" { profile: u32, value: Handle },
        Commit => "commit" { owner: Handle, binder: Handle, constraint: Handle, instance: u32 }
    }
}
tagged! {
    /// Send/receive, attempts/effects and compensation events keep separate roles.
    Event {
        Send => "send" { channel: Handle },
        Receive => "receive" { channel: Handle, send: Handle },
        Attempt => "attempt" { owner: Handle, operation: ExportRef, contracts: Vec<u32>, instance: u32 },
        Effect => "effect" { attempt: Handle, instance: u32 },
        Event => "event" { owner: Handle, compensation: Nullable<Handle>, instance: u32 }
    }
}
record! { /// Authored relationship occurrence between exact value owners.
    Related { relationship: u32, from: Handle, to: Handle, locus: Locus }
}
labels! {
    /// A control endpoint; entering and completing a control are distinct.
    Port { Enter => "enter", Exit => "exit" }
}
labels! {
    /// Derived causality, with bounded repeat progress as its explicit cyclic edge.
    EdgeKind {
        Sequence => "sequence", Branch => "branch", Join => "join", RepeatProgress => "repeat_progress",
        AwaitSuccess => "await_success", AwaitTimeout => "await_timeout"
    }
}
record! { /// Control node and the selected entry/completion port.
    Endpoint { node: Handle, port: Port }
}
record! { /// A claimed derived edge; only repeat progress has an authored maximum.
    CausalEdge { kind: EdgeKind, owner: Handle, from: Endpoint, to: Endpoint, maximum: Nullable<Integer> }
}
record! { /// Authored final predicate and separate complete-workflow closure binding.
    Finish { name: String, binder: Handle, constraint: Handle, closure: u32, locus: Locus }
}
record! { /// Complete recovery obligation with distinct registration/activation/retry roles.
    Compensation {
        name: String, forward_effect: Handle, forward: Handle, owner: Handle, operation: ExportRef, profile: u32,
        clock: u32, registration_anchor: Handle, registration_instance: u32, registration_captures: Vec<Handle>,
        trigger: Handle, guard: Handle, activation_anchor: Handle, activation_captures: Vec<Handle>, within: Interval,
        maximum_attempts: Integer, attempt_type: u32, earlier: Handle, later: Handle, retry: Handle, attempt_instance: u32,
        effect_instance: u32, commit: Nullable<Handle>, recovery: Handle, recover: Handle, recovery_bindings: Vec<u32>, locus: Locus
    }
}
labels! {
    /// Requirements for later inputs; no runtime identity is supplied by these tags.
    BindingKind {
        WorkflowInstance => "workflow_instance", RoleInstance => "role_instance", Participant => "participant",
        Component => "component", Endpoint => "endpoint", Message => "message", Send => "send", Receive => "receive",
        Delivery => "delivery", Attempt => "attempt", Effect => "effect", CompensationRegistration => "compensation_registration",
        CompensationAttempt => "compensation_attempt", CompensationEffect => "compensation_effect", Commit => "commit",
        Snapshot => "snapshot", Invocation => "invocation", Population => "population", Relationship => "relationship",
        Clock => "clock", Window => "window", Observation => "observation", Progress => "progress",
        Closure => "closure", Capture => "capture"
    }
}
record! { /// Exact static authority, role and prerequisite contracts for a later binding.
    BindingRequirement {
        name: String, kind: BindingKind, #[serde(rename = "type")] value_type: Nullable<u32>, authority: ArtifactRef, contract: u32,
        model: Nullable<ExportRef>, subject: Subject, anchor: Handle, scope: Handle, relation: Nullable<u32>, requires: Vec<u32>, locus: Locus
    }
}
tagged! {
    /// The exact declaration-local subject of a runtime requirement.
    Subject {
        Declaration => "declaration" { declaration: u32 },
        Role => "role" { role: Handle },
        Channel => "channel" { channel: Handle },
        Control => "control" { control: Handle },
        Compensation => "compensation" { compensation: Handle }
    }
}

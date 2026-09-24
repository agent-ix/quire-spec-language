// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042: closed transport records in the contract's canonical member order.
//!
//! These records carry untrusted claims. Deserializing or constructing one does
//! not establish source correspondence, family admission or an accepted artifact.

use super::{NumberError, NumberWire, ProtocolNumber};
use qsl_foundation::serde_object::from_object;
use qsl_foundation::ByteDigest;
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
        $($(#[$variant_meta:meta])* $variant:ident => $tag:literal { $($(#[$field_meta:meta])* $field:ident: $ty:ty),* $(,)? }),* $(,)?
    }) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
        #[serde(tag = "kind")]
        pub enum $name {
            // $variant_meta is docs-only: it is NOT re-emitted onto the private `Fields` decoder enum below (unlike `record!`'s $field_meta, which mirrors onto both sides), so a future `#[serde(...)]` variant attribute here would apply on serialize but be silently dropped on deserialize.
            $($(#[$variant_meta])* #[serde(rename = $tag)] $variant { $($(#[$field_meta])* $field: $ty),* }),*
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
    ($(#[$meta:meta])* $name:ident { $($(#[$variant_meta:meta])* $variant:ident => $wire:literal),* $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize)]
        pub enum $name { $($(#[$variant_meta])* #[serde(rename = $wire)] $variant),* }
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

/// A `sha256-jcs` digest (FR-056-CON-4): 64 lowercase hex digits, with no
/// `sha256:` prefix, so a raw-byte [`ByteDigest`] spelling cannot occupy it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct JcsDigest(pub [u8; 32]);

impl Serialize for JcsDigest {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use std::fmt::Write as _;
        let mut text = String::with_capacity(64);
        for byte in self.0 {
            write!(text, "{byte:02x}").map_err(serde::ser::Error::custom)?;
        }
        serializer.serialize_str(&text)
    }
}

impl<'de> Deserialize<'de> for JcsDigest {
    fn deserialize<D: Deserializer<'de>>(decoder: D) -> Result<Self, D::Error> {
        // `from_hex` admits exactly 64 lowercase hex digits and no prefix.
        ByteDigest::from_hex(&String::deserialize(decoder)?)
            .map(|digest| Self(digest.as_bytes()))
            .map_err(de::Error::custom)
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
        /// A protocol binding artifact.
        Binding => "binding",
        /// A dependency-closure artifact.
        DependencyClosure => "dependency-closure",
        /// An environment artifact.
        Environment => "environment",
        /// An executable-projection artifact.
        ExecutableProjection => "executable-projection",
        /// A fault-model artifact.
        FaultModel => "fault-model",
        /// A generated artifact.
        GeneratedArtifact => "generated-artifact",
        /// An invocation artifact.
        Invocation => "invocation",
        /// A linked-package artifact.
        LinkedPackage => "linked-package",
        /// A model-lock artifact.
        ModelLock => "model-lock",
        /// A model-manifest artifact.
        ModelManifest => "model-manifest",
        /// A model-package artifact.
        ModelPackage => "model-package",
        /// An observation artifact.
        Observation => "observation",
        /// An oracle artifact.
        Oracle => "oracle",
        /// A population artifact.
        Population => "population",
        /// A property artifact.
        Property => "property",
        /// A review-disposition artifact.
        ReviewDisposition => "review-disposition",
        /// A review-procedure artifact.
        ReviewProcedure => "review-procedure",
        /// A run artifact.
        RunArtifact => "run-artifact",
        /// A snapshot artifact.
        Snapshot => "snapshot",
        /// A source artifact.
        Source => "source",
        /// A trace artifact.
        Trace => "trace",
        /// A temporal-window artifact.
        Window => "window"
    }
}
labels! {
    /// Expected producer export kind; a tag alone never supplies that authority.
    ExportKind {
        /// A component export.
        Component => "component",
        /// An endpoint export.
        Endpoint => "endpoint",
        /// An enum export.
        Enum => "enum",
        /// A field export.
        Field => "field",
        /// An object export.
        Object => "object",
        /// An operation export.
        Operation => "operation",
        /// A population export.
        Population => "population",
        /// A record export.
        Record => "record",
        /// A reference export.
        Reference => "reference",
        /// A relationship export.
        Relationship => "relationship",
        /// A scalar export.
        Scalar => "scalar",
        /// An enum variant export.
        Variant => "variant"
    }
}

record! { /// An opaque revision within its explicitly selected namespace.
    Revision {
        /// The revision scheme this value is interpreted under.
        namespace: String,
        /// The revision's own opaque spelling within `namespace`.
        value: String
    }
}
record! { /// Wire identity/version, separate from semantic definition revision.
    Wire {
        /// The wire schema's identity string.
        identity: String,
        /// The wire schema's own version, independent of the payload's semantic revision.
        version: String
    }
}
record! { /// Narrow representation of the exact shared ArtifactRef/3 object.
    ArtifactRef {
        /// The ArtifactRef schema's own version, distinct from `revision` below.
        #[serde(rename = "refVersion")] ref_version: String,
        /// The referenced artifact's closed kind (`ArtifactKind`).
        kind: ArtifactKind,
        /// The authority that minted or governs this artifact's identity.
        authority: String,
        /// The artifact's own identity string, unique within `authority`.
        identity: String,
        /// The artifact's selected revision.
        revision: Revision,
        /// The artifact's content digest, in its exact selected domain.
        #[serde(with = "digest")] digest: ByteDigest,
        /// The wire schema this artifact reference itself conforms to.
        wire: Wire
    }
}
record! { /// Explicit formal document owner; native labels cannot substitute for it.
    Formal {
        /// The owning formal document's identity.
        document: String,
        /// The selected revision of `document`.
        revision: Revision
    }
}
record! { /// Original decoded UTF-8 byte coordinates, before boundary validation.
    Span {
        /// The inclusive UTF-8 byte offset where the span starts.
        start: u32,
        /// The exclusive UTF-8 byte offset where the span ends.
        end: u32
    }
}
record! { /// Index into the package's original source table and its byte region.
    Locus {
        /// Index into the package's `sources` table naming the origin text.
        source: u32,
        /// The byte region within that source text.
        span: Span
    }
}
record! { /// Independently supplied producer source and authored formal coordinates.
    ForeignLocus {
        /// The external producer artifact this locus originates from.
        source: ArtifactRef,
        /// The formal document and revision this locus's coordinates are authored against.
        formal: Formal,
        /// The byte region within the foreign source.
        span: Span
    }
}
record! { /// A declaration-owned table index; its expected table follows the field.
    Handle {
        /// Index into the package's `declarations` table owning the referenced node.
        declaration: u32,
        /// Index into that declaration's own table the handle's context expects.
        index: u32
    }
}
record! { /// Selected implementation and binary artifact, not an authenticity claim.
    Producer {
        /// The producer implementation's identity.
        implementation: String,
        /// The producer implementation's selected revision.
        revision: Revision,
        /// The producer's binary artifact.
        binary: ArtifactRef
    }
}
record! { /// One immutable dependency and its own dependency-table prerequisites.
    Dependency {
        /// The depended-upon artifact.
        artifact: ArtifactRef,
        /// Indices into the package's `dependencies` table this dependency itself requires.
        requires: Vec<u32>
    }
}
record! { /// Selected definition source, required rule sources and definition closure.
    Definition {
        /// The definition's own identity.
        identity: String,
        /// The definition's selected revision.
        revision: Revision,
        /// Index into the package's `sources` table for this definition's artifact.
        artifact: u32,
        /// Indices into `sources` for the rule sources this definition applies.
        rules: Vec<u32>,
        /// Indices into the package's `dependencies` table this definition requires.
        requires: Vec<u32>
    }
}
record! { /// Located producer export, whose kind/path must exist in its admitted view.
    Export {
        /// The exported member's closed kind.
        kind: ExportKind,
        /// The dotted path naming this export within the producer's admitted view.
        path: Vec<String>,
        /// Where this export is declared in the foreign producer source.
        locus: ForeignLocus
    }
}
record! { /// FR-042/FR-056: the exact domain package a model was linked against.
    DomainPackage {
        /// The domain package's own identity.
        identity: String,
        /// The domain package's own version.
        version: String,
        /// The `sha256-jcs` digest of its Semantic IR 2.0.0 document.
        digest: JcsDigest
    }
}
record! { /// Selected model artifact and exports, naming its domain package directly.
    Model {
        /// Index into the package's `dependencies` table for the model artifact.
        artifact: u32,
        /// The selected model profile.
        profile: String,
        /// The model's exports.
        exports: Vec<Export>,
        /// The linked domain package; explicit null for a directly admitted native model.
        domain_package: Nullable<DomainPackage>
    }
}
record! { /// Editable native authority labels; these are not formal revisions.
    NativeSource {
        /// The native source's own identity label.
        identity: String,
        /// The native source's own editable revision label.
        revision: String
    }
}
record! { /// Original source text, retained for hashing/loci and never reparsed here.
    Source {
        /// The artifact this source text was retrieved from.
        artifact: ArtifactRef,
        /// The native identity/revision labels for this source.
        native: NativeSource,
        /// The source's own path, as recorded by its producer.
        path: String,
        /// The formal document and revision this source corresponds to.
        formal: Formal,
        /// The original, unreparsed source text.
        text: String
    }
}
record! { /// Closed capabilities are validated independently from semantic definitions.
    Features {
        /// The closed set of feature identities this package declares.
        declarations: Vec<String>,
        /// The feature identities a consumer must support to admit this package.
        required: Vec<String>,
        /// The feature identities a consumer may ignore without losing admissibility.
        optional: Vec<String>
    }
}
record! { /// Explicit native language/edition selection.
    Language {
        /// The native language's identity.
        identity: String,
        /// The selected language edition.
        edition: String
    }
}
record! { /// Untrusted complete payload in its required canonical member order.
    Package {
        /// The wire schema identity this payload claims to conform to.
        wire: String,
        /// The payload's media type.
        media: String,
        /// The schema artifact this payload claims to validate against.
        schema: String,
        /// The package's own kind tag.
        #[serde(rename = "type")] package_type: String,
        /// The payload's declared byte encoding.
        encoding: String,
        /// The payload's declared numeric representation.
        numeric: String,
        /// The contract artifact this package is checked against.
        contract: ArtifactRef,
        /// The producer that generated this package.
        producer: Producer,
        /// The baseline artifact this package's contents are compared against.
        baseline: ArtifactRef,
        /// The native language/edition this package's declarations are authored in.
        language: Language,
        /// Index into `definitions` naming this package's own definition.
        package_definition: u32,
        /// The closed feature set this package declares and requires.
        features: Features,
        /// The package's source-text table.
        sources: Vec<Source>,
        /// The package's dependency table.
        dependencies: Vec<Dependency>,
        /// The package's definition table.
        definitions: Vec<Definition>,
        /// The package's model table.
        models: Vec<Model>,
        /// The package's resolved native type table.
        types: Vec<Type>,
        /// The package's declaration table.
        declarations: Vec<Declaration>
    }
}

record! { /// Exact model/export indices; equal spelling never merges nominal owners.
    ExportRef {
        /// Index into the package's `models` table.
        model: u32,
        /// Index into that model's `exports` table.
        export: u32
    }
}

tagged! {
    /// Resolved native type claims, preserving exact exports, units and wrappers.
    Type {
        /// The native boolean type.
        Boolean => "boolean" {},
        /// A scalar type, with its exported definition, optional unit and authored bounds.
        Scalar => "scalar" {
            /// The scalar's exported definition.
            export: ExportRef,
            /// The scalar's unit, absent when unitless.
            unit: Nullable<String>,
            /// The scalar's authored numeric bounds.
            representation: Representation
        },
        /// An exported enum type.
        Enum => "enum" {
            /// The enum's exported definition.
            export: ExportRef
        },
        /// An exported record type.
        Record => "record" {
            /// The record's exported definition.
            export: ExportRef
        },
        /// An exported object type.
        Object => "object" {
            /// The object's exported definition.
            export: ExportRef
        },
        /// A reference type, naming its object, its target and the universe it resolves within.
        Reference => "reference" {
            /// The reference type's own exported definition.
            export: ExportRef,
            /// The referenced object type.
            object: ExportRef,
            /// The universe this reference resolves against.
            universe: ExportRef
        },
        /// An optional wrapper around another type.
        Option => "option" {
            /// Index into `types` for the wrapped type.
            value: u32
        },
        /// A sequence wrapper around another type, with an authored maximum length.
        Sequence => "sequence" {
            /// Index into `types` for the element type.
            element: u32,
            /// The sequence's authored maximum length.
            maximum: Integer
        }
    }
}
tagged! {
    /// Native authored bounds encoded only through the exact integer alternative.
    Representation {
        /// Integer bounds.
        Integer => "integer" {
            /// The inclusive lower bound.
            minimum: Integer,
            /// The inclusive upper bound.
            maximum: Integer
        },
        /// Rational bounds, with a separate denominator ceiling.
        Rational => "rational" {
            /// The numerator's inclusive lower bound.
            numerator_minimum: Integer,
            /// The numerator's inclusive upper bound.
            numerator_maximum: Integer,
            /// The denominator's inclusive upper bound.
            maximum_denominator: Integer
        },
        /// Text bounds.
        Text => "text" {
            /// The text's maximum length, in Unicode scalar values.
            maximum_scalars: Integer
        }
    }
}
labels! {
    /// Static observation/activation roles; concrete observations are absent.
    AnchorKind {
        /// A predicate declaration's own evaluation point.
        Predicate => "predicate",
        /// The current, present-moment evaluation point.
        Current => "current",
        /// An invocation's input evaluation point.
        InvocationInput => "invocation_input",
        /// An invocation's precondition evaluation point.
        InvocationPre => "invocation_pre",
        /// An invocation's postcondition evaluation point.
        InvocationPost => "invocation_post",
        /// A declaration's activation evaluation point.
        Activation => "activation",
        /// A temporal formula's instant of evaluation.
        TemporalInstant => "temporal_instant",
        /// A protocol formula's instant of evaluation.
        ProtocolInstant => "protocol_instant",
        /// A FIFO-ordered channel's evaluation point.
        Fifo => "fifo",
        /// A compensation's registration evaluation point.
        Registration => "registration",
        /// A compensation's activation evaluation point.
        CompensationActivation => "compensation_activation",
        /// A compensation's retry evaluation point.
        Retry => "retry",
        /// A compensation's recovery evaluation point.
        Recovery => "recovery",
        /// A control node's evaluation point.
        Control => "control",
        /// A workflow's finish evaluation point.
        Finish => "finish"
    }
}
labels! {
    /// Binder role remains separate from its type, scope and immutable anchor.
    BinderKind {
        /// An operation's own declared parameter.
        Parameter => "parameter",
        /// A declaration's input binder.
        Input => "input",
        /// A protocol's trigger binder.
        Trigger => "trigger",
        /// A control's captured value binder.
        Capture => "capture",
        /// A `let` expression's bound name.
        Let => "let",
        /// A query expression's own iteration binder.
        Query => "query",
        /// The implicit receiver (`self`) binder.
        SelfValue => "self",
        /// An operation's own result binder.
        Result => "result",
        /// An invocation's parameter binder.
        InvocationParameter => "invocation_parameter",
        /// An event's own binder.
        Event => "event",
        /// A workflow finish's own binder.
        Finish => "finish",
        /// A FIFO channel's own binder.
        Fifo => "fifo",
        /// A compensation's forward-effect binder.
        ForwardEffect => "forward_effect",
        /// A compensation's trigger binder.
        CompensationTrigger => "compensation_trigger",
        /// A compensation's earlier-attempt binder.
        EarlierAttempt => "earlier_attempt",
        /// A compensation's later-attempt binder.
        LaterAttempt => "later_attempt",
        /// A compensation's recovery binder.
        Recovery => "recovery"
    }
}
record! { /// Declaration-local lexical nesting and its original source region.
    Scope {
        /// The enclosing scope, absent for the declaration's outermost scope.
        parent: Nullable<Handle>,
        /// Where this scope is opened in the original source.
        locus: Locus
    }
}
record! { /// Owner and runtime-requirement indices have role-specific interpretations.
    Anchor {
        /// The anchor's static-observation/activation role.
        kind: AnchorKind,
        /// The node that owns this anchor, absent when the anchor is declaration-scoped.
        owner: Nullable<Handle>,
        /// Index into the declaration's runtime-requirement table this anchor is bound to.
        binding: Nullable<u32>,
        /// Where this anchor occurs in the original source.
        locus: Locus
    }
}
record! { /// Immutable lexical value with explicit initialization and source owner.
    Binder {
        /// The binder's own name.
        name: String,
        /// The binder's role.
        kind: BinderKind,
        /// Index into `types` for the binder's static type.
        #[serde(rename = "type")] value_type: u32,
        /// The scope this binder is visible within.
        scope: Handle,
        /// The evaluation point this binder's initializer is anchored to.
        anchor: Handle,
        /// The binder's own initializing value, absent when supplied externally.
        initializer: Nullable<Handle>,
        /// Where this binder is declared in the original source.
        locus: Locus
    }
}
tagged! {
    /// The retained value origin, separate from the surrounding evaluation anchor.
    Origin {
        /// A value with no dependency on a captured or selected input.
        Independent => "independent" {},
        /// A value taken from the surrounding evaluation anchor.
        Anchor => "anchor" {
            /// The anchor this value is taken from.
            anchor: Handle
        },
        /// A value taken from another, already-evaluated node.
        Selected => "selected" {
            /// The node this value is selected from.
            value: Handle
        }
    }
}
labels! {
    /// Only these native unary operations have a value-graph representation.
    Unary {
        /// Boolean negation.
        Not => "not",
        /// Arithmetic negation.
        Negate => "negate",
        /// Presence test on an optional value.
        Present => "present",
        /// Unwrap an optional value's payload.
        Value => "value",
        /// Dereference a reference value.
        Deref => "deref"
    }
}
labels! {
    /// Closed binary operations; integer division and remainder are absent.
    Binary {
        /// Boolean implication.
        Implies => "implies",
        /// Boolean disjunction.
        Or => "or",
        /// Boolean conjunction.
        And => "and",
        /// Value equality.
        Equal => "equal",
        /// Value inequality.
        NotEqual => "not_equal",
        /// Strictly-less-than ordering.
        Less => "less",
        /// Less-than-or-equal ordering.
        LessEqual => "less_equal",
        /// Strictly-greater-than ordering.
        Greater => "greater",
        /// Greater-than-or-equal ordering.
        GreaterEqual => "greater_equal",
        /// Addition.
        Add => "add",
        /// Subtraction.
        Subtract => "subtract",
        /// Multiplication.
        Multiply => "multiply",
        /// Rational (non-integer) division.
        RationalDivide => "rational_divide"
    }
}
labels! {
    /// Ordered-sequence queries preserve binders and duplicates.
    Query {
        /// Universal quantification over a collection.
        ForAll => "forall",
        /// Existential quantification over a collection.
        Exists => "exists",
        /// Filter a collection by a predicate body.
        Filter => "filter",
        /// Map a collection through a body expression.
        Map => "map",
        /// Count a collection's elements.
        Count => "count",
        /// Sum a collection's elements.
        Sum => "sum"
    }
}
record! { /// Original value occurrence and selected type/profile/lexical provenance.
    Value {
        /// Index into the original (pre-checking) expression table this value was lowered from.
        original_expression: u32,
        /// Where this value occurs in the original source.
        locus: Locus,
        /// Where this value's operator occurs, absent when the value has none.
        operator_locus: Nullable<Locus>,
        /// Index into `types` for this value's static type.
        #[serde(rename = "type")] value_type: u32,
        /// Index into the declaration's runtime-requirement table this value is profiled under.
        profile: u32,
        /// The scope this value evaluates within.
        scope: Handle,
        /// The evaluation point this value is anchored to.
        anchor: Handle,
        /// Where this value's own contribution originates from.
        origin: Origin,
        /// The operation this value performs.
        operation: ValueOperation
    }
}
tagged! {
    /// Native evaluation graph claims, never the prover's Boolean abstraction.
    ValueOperation {
        /// A boolean literal.
        Boolean => "boolean" {
            /// The literal's own value.
            value: bool
        },
        /// A numeric literal.
        Number => "number" {
            /// The literal's own value.
            value: Number
        },
        /// A text literal.
        Text => "text" {
            /// The literal's own value.
            value: String
        },
        /// An enum member reference.
        Enum => "enum" {
            /// The referenced enum member's exported definition.
            variant: ExportRef
        },
        /// A read of a previously bound value.
        Read => "read" {
            /// The binder being read.
            binder: Handle
        },
        /// A parenthesized sub-expression, retained for source fidelity.
        Group => "group" {
            /// The grouped value.
            value: Handle
        },
        /// A field projection on a composite value.
        Field => "field" {
            /// The value the field is projected from.
            base: Handle,
            /// The projected field's exported definition.
            field: ExportRef
        },
        /// A unary operation.
        Unary => "unary" {
            /// The unary operator applied.
            operator: Unary,
            /// The operand.
            value: Handle
        },
        /// A binary operation.
        Binary => "binary" {
            /// The binary operator applied.
            operator: Binary,
            /// The left operand.
            left: Handle,
            /// The right operand.
            right: Handle
        },
        /// A conditional expression.
        If => "if" {
            /// The condition value.
            condition: Handle,
            /// The value when `condition` holds.
            then_value: Handle,
            /// The value when `condition` does not hold.
            else_value: Handle
        },
        /// A local binding expression.
        Let => "let" {
            /// The bound name.
            binder: Handle,
            /// The binder's initializing value.
            initializer: Handle,
            /// The expression evaluated with the binding in scope.
            body: Handle
        },
        /// A reference to an operation's own precondition-anchored value.
        Pre => "pre" {
            /// The value as it stood at the precondition anchor.
            value: Handle,
            /// The precondition evaluation point referenced.
            anchor: Handle
        },
        /// A predicate call.
        Call => "call" {
            /// Index into the package's `declarations` table for the called predicate.
            predicate: u32,
            /// The call's argument values.
            arguments: Vec<Handle>
        },
        /// A collection's size.
        Size => "size" {
            /// The collection being measured.
            collection: Handle,
            /// Index into `types` for the resulting count's type.
            result: u32
        },
        /// A collection membership test.
        Contains => "contains" {
            /// The collection being tested.
            collection: Handle,
            /// The candidate member.
            member: Handle
        },
        /// An ordered-sequence query over a collection.
        Query => "query" {
            /// The query operator applied.
            operator: Query,
            /// The query's own iteration binder.
            binder: Handle,
            /// The collection being queried.
            collection: Handle,
            /// The query's body expression, evaluated once per element.
            body: Handle,
            /// Index into `types` for the query's result type.
            result: u32
        },
        /// A generalization/relationship parent projection.
        Parent => "parent" {
            /// The value the parent is projected from.
            reference: Handle,
            /// The relationship edge's exported definition.
            edge: ExportRef,
            /// The universe this projection resolves within.
            universe: ExportRef
        },
        /// A reachability test over a relationship edge.
        Reaches => "reaches" {
            /// The starting value.
            start: Handle,
            /// The target value tested for reachability.
            target: Handle,
            /// The relationship edge's exported definition.
            edge: ExportRef,
            /// The universe this reachability test resolves within.
            universe: ExportRef
        }
    }
}

record! { /// Authored requirement identity under its separate revision namespace.
    Requirement {
        /// The requirement's owning package.
        package: String,
        /// The requirement's own identity, unique within `package`.
        identity: String,
        /// The requirement's selected revision.
        revision: Revision
    }
}
record! { /// Complete declaration-owned tables, with explicit authored clause mapping.
    Declaration {
        /// The declaration's own name.
        name: String,
        /// Where this declaration occurs in the original source.
        locus: Locus,
        /// The requirement this declaration is authored to satisfy.
        requirement: Requirement,
        /// The authored clause kind this declaration maps to.
        clause: String,
        /// The declaration's selected execution binding.
        execution: Execution,
        /// Index into the declaration's runtime-requirement table for its default profile.
        profile: u32,
        /// Indices into the package's `dependencies` table this declaration requires.
        requires: Vec<u32>,
        /// The declaration's own lexical scope table.
        scopes: Vec<Scope>,
        /// The declaration's own evaluation-anchor table.
        anchors: Vec<Anchor>,
        /// The declaration's own binder table.
        binders: Vec<Binder>,
        /// The declaration's own value table.
        values: Vec<Value>,
        /// The declaration's own temporal-formula table.
        temporal: Vec<Temporal>,
        /// The declaration's own runtime-requirement (binding) table.
        bindings: Vec<BindingRequirement>,
        /// The declaration's root body, selecting its native declaration family.
        body: Body
    }
}
tagged! {
    /// Authored execution selection; operation alternatives select exact exports.
    Execution {
        /// A model's initialization operation.
        Initialization => "initialization" {
            /// The initialization operation's own name.
            name: String
        },
        /// An event handler operation.
        Handler => "handler" {
            /// The handler operation's own name.
            name: String
        },
        /// An operation's precondition clause.
        Pre => "pre" {
            /// The operation this precondition belongs to.
            operation: ExportRef
        },
        /// An operation's postcondition clause.
        Post => "post" {
            /// The operation this postcondition belongs to.
            operation: ExportRef
        }
    }
}
labels! {
    /// State clause kind in this wire, distinct from a proof or runtime result.
    ClauseKind {
        /// An invariant clause.
        Invariant => "invariant",
        /// A precondition clause.
        Pre => "pre",
        /// A postcondition clause.
        Post => "post"
    }
}
tagged! {
    /// Closed native declaration families; every local handle keeps its owner.
    Body {
        /// A predicate declaration body.
        Predicate => "predicate" {
            /// The predicate's own declared parameters.
            parameters: Vec<Handle>,
            /// Index into `types` for the predicate's result type.
            result: u32,
            /// The predicate's root expression.
            root: Handle
        },
        /// A state clause (invariant/precondition/postcondition) declaration body.
        State => "state" {
            /// The clause kind this declaration expresses.
            clause_kind: ClauseKind,
            /// The clause's owning context (model or operation).
            context: ExportRef,
            /// The operation this clause belongs to, absent for a model invariant.
            operation: Nullable<ExportRef>,
            /// The clause's root expression.
            root: Handle
        },
        /// A temporal declaration body.
        Temporal => "temporal" {
            /// The temporal formula's own input.
            input: Handle,
            /// Index into the declaration's runtime-requirement table for the selected clock.
            clock: u32,
            /// The declaration's activation.
            activation: Activation,
            /// The values this declaration captures at activation.
            captures: Vec<Handle>,
            /// The temporal formula's root node.
            root: Handle
        },
        /// A protocol declaration body.
        Protocol => "protocol" {
            /// The protocol's own input.
            input: Handle,
            /// The declaration's activation.
            activation: Activation,
            /// The values this declaration captures at activation.
            captures: Vec<Handle>,
            /// The protocol's participant roles.
            roles: Vec<Role>,
            /// The protocol's authoritative relationships.
            relationships: Vec<Relationship>,
            /// The protocol's message channels.
            channels: Vec<Channel>,
            /// The protocol's compensation obligations.
            compensations: Vec<Compensation>,
            /// Indices into the declaration's runtime-requirement table this protocol requires.
            temporal_requirements: Vec<u32>,
            /// The protocol's control-flow node table.
            controls: Vec<Control>,
            /// The protocol's derived causal-edge table.
            causal_edges: Vec<CausalEdge>,
            /// The protocol's own run control node.
            run: Handle,
            /// The protocol's own finish obligation.
            finish: Box<Finish>
        }
    }
}
tagged! {
    /// Declaration activation and explicit immutable capture origin.
    Activation {
        /// Activated once, at the declaration's own origin.
        Origin => "origin" {
            /// The origin evaluation point.
            anchor: Handle
        },
        /// Activated once per occurrence of a trigger, subject to an optional guard.
        Each => "each" {
            /// The triggering value.
            trigger: Handle,
            /// The guard condition gating activation, absent when unconditional.
            guard: Nullable<Handle>,
            /// The activation's own evaluation point.
            anchor: Handle
        }
    }
}
record! { /// Authored closed interval in the selected clock/range domain.
    Interval {
        /// The interval's inclusive lower bound.
        lower: Integer,
        /// The interval's inclusive upper bound.
        upper: Integer
    }
}
record! { /// Original temporal occurrence; values and temporal truth remain distinct.
    Temporal {
        /// Index into the original (pre-checking) expression table this formula was lowered from.
        original_node: u32,
        /// Where this temporal formula occurs in the original source.
        locus: Locus,
        /// The temporal operation this formula performs.
        operation: TemporalOperation
    }
}
labels! {
    /// Unary temporal operators; interval presence follows the selected operator.
    TemporalUnary {
        /// Temporal negation.
        Not => "not",
        /// Eventually holds.
        Eventually => "eventually",
        /// Always holds.
        Always => "always",
        /// Held at some point in the past.
        Once => "once",
        /// Has always held in the past.
        Historically => "historically"
    }
}
labels! {
    /// Binary temporal operators over an explicitly selected temporal definition.
    TemporalBinary {
        /// Temporal implication.
        Implies => "implies",
        /// Temporal disjunction.
        Or => "or",
        /// Temporal conjunction.
        And => "and",
        /// Holds until the right operand holds.
        Until => "until",
        /// Releases the right operand once the left holds.
        Release => "release",
        /// Has held since the right operand last held.
        Since => "since",
        /// Was triggered by the right operand holding.
        Triggered => "triggered"
    }
}
tagged! {
    /// Closed temporal syntax-derived graph, with exact clock-bound intervals.
    TemporalOperation {
        /// A constant boolean truth value.
        Constant => "constant" {
            /// The constant's own value.
            value: bool
        },
        /// A reference to a state-clause value's truth.
        Holds => "holds" {
            /// The referenced value.
            value: Handle
        },
        /// A parenthesized sub-formula, retained for source fidelity.
        Group => "group" {
            /// The grouped formula.
            value: Handle
        },
        /// A unary temporal operation.
        Unary => "unary" {
            /// The unary temporal operator applied.
            operator: TemporalUnary,
            /// The operator's clock-bound interval, absent when unbounded.
            interval: Nullable<Interval>,
            /// The operand formula.
            value: Handle
        },
        /// A binary temporal operation.
        Binary => "binary" {
            /// The binary temporal operator applied.
            operator: TemporalBinary,
            /// The operator's clock-bound interval, absent when unbounded.
            interval: Nullable<Interval>,
            /// The left operand formula.
            left: Handle,
            /// The right operand formula.
            right: Handle
        }
    }
}

record! { /// Participant responsibility with a distinct later role-instance binding.
    Role {
        /// The role's own name.
        name: String,
        /// The model this role is played against.
        model: ExportRef,
        /// Index into the declaration's runtime-requirement table for the role's later instance binding.
        instance: u32,
        /// Where this role is declared in the original source.
        locus: Locus
    }
}
record! { /// An authoritative relationship export and its later binding requirement.
    Relationship {
        /// The relationship's own name.
        name: String,
        /// The relationship's exported model definition.
        model: ExportRef,
        /// Index into the declaration's runtime-requirement table for the relationship's later binding.
        binding: u32,
        /// Where this relationship is declared in the original source.
        locus: Locus
    }
}
record! { /// Message/delivery requirements remain distinct from operation effects.
    Channel {
        /// The channel's own name.
        name: String,
        /// The sending role.
        from: Handle,
        /// The receiving role.
        to: Handle,
        /// Index into `types` for the channel's message type.
        message_type: u32,
        /// The channel's ordering guarantee.
        ordering: Ordering,
        /// The channel's authored delivery-time interval.
        delivery: Interval,
        /// Index into the declaration's runtime-requirement table for the message binding.
        message: u32,
        /// Index into the declaration's runtime-requirement table for the send-event binding.
        send: u32,
        /// Index into the declaration's runtime-requirement table for the receive-event binding.
        receive: u32,
        /// Index into the declaration's runtime-requirement table for the delivery-instance binding.
        delivery_instance: u32,
        /// Where this channel is declared in the original source.
        locus: Locus
    }
}
tagged! {
    /// Per-channel ordering; FIFO does not create cross-channel causality.
    Ordering {
        /// No ordering guarantee between messages on this channel.
        Unordered => "unordered" {},
        /// First-in-first-out ordering, scoped to this channel alone.
        Fifo => "fifo" {
            /// The FIFO sequence's own binder.
            binder: Handle,
            /// The FIFO ordering key.
            key: Handle,
            /// The FIFO evaluation point.
            anchor: Handle
        }
    }
}
record! { /// Original protocol control occurrence, before graph/family verification.
    Control {
        /// The control node's own name.
        name: String,
        /// Index into the original (pre-checking) node table this control was lowered from.
        original_node: u32,
        /// Where this control node occurs in the original source.
        locus: Locus,
        /// The control operation this node performs.
        operation: ControlOperation
    }
}
record! { /// Authored choice alternative, preserving guard and body order.
    ChoiceCase {
        /// The case's own label.
        label: String,
        /// The condition selecting this case.
        guard: Handle,
        /// The control executed when this case is selected.
        body: Handle,
        /// Where this case is declared in the original source.
        locus: Locus
    }
}
record! { /// One parallel branch; its table ordinal supplies a join index.
    Branch {
        /// The branch's own label.
        label: String,
        /// The control executed by this branch.
        body: Handle,
        /// Where this branch is declared in the original source.
        locus: Locus
    }
}
tagged! {
    /// Await may start from an event or a compensation activation, with distinct tables.
    AwaitAnchor {
        /// Awaiting a plain event.
        Event => "event" {
            /// The awaited event node.
            node: Handle
        },
        /// Awaiting a compensation's own activation.
        Compensation => "compensation" {
            /// The awaited compensation.
            compensation: Handle
        }
    }
}
tagged! {
    /// Closed finite control structures; structural validation derives their edges.
    ControlOperation {
        /// Ordered sequential execution.
        Sequence => "sequence" {
            /// The child controls, in execution order.
            children: Vec<Handle>
        },
        /// A guarded alternative selection.
        Choice => "choice" {
            /// The node owning this choice's local bindings.
            owner: Handle,
            /// The values visible to this choice's guards.
            visible: Vec<Handle>,
            /// The choice's own alternatives, in authored order.
            cases: Vec<ChoiceCase>
        },
        /// Concurrent execution of independent branches.
        Parallel => "parallel" {
            /// The concurrent branches.
            branches: Vec<Branch>,
            /// Indices into the branches this parallel node joins on completion.
            join: Vec<u32>
        },
        /// Bounded repeated execution.
        Repeat => "repeat" {
            /// The node owning this repeat's local bindings.
            owner: Handle,
            /// The values visible to this repeat's guard and body.
            visible: Vec<Handle>,
            /// The authored maximum number of iterations.
            maximum: Integer,
            /// The condition that continues iteration.
            guard: Handle,
            /// The control executed on each iteration.
            body: Handle,
            /// The control executed once the iteration bound is reached.
            exhausted: Handle
        },
        /// A bounded wait for an event, with a timeout alternative.
        Await => "await" {
            /// The anchor this await starts from.
            after: AwaitAnchor,
            /// Index into the declaration's runtime-requirement table for the await's profile.
            profile: u32,
            /// Index into the declaration's runtime-requirement table for the selected clock.
            clock: u32,
            /// The authored timeout interval.
            within: Interval,
            /// The awaited event.
            event: Handle,
            /// The control executed when the event arrives in time.
            then_body: Handle,
            /// The control executed when the timeout elapses first.
            timeout: Handle
        },
        /// Emits an event, binding it and checking a constraint over it.
        Event => "event" {
            /// The emitted event.
            event: Event,
            /// The binder capturing the emitted event.
            binder: Handle,
            /// The relationship occurrences related to this event.
            related: Vec<Related>,
            /// The constraint checked over the emitted event.
            constraint: Handle
        },
        /// A structural check with no visible protocol effect.
        Check => "check" {
            /// Index into the declaration's runtime-requirement table for the check's profile.
            profile: u32,
            /// The value checked.
            value: Handle
        },
        /// Commits a binding, making it visible to later control nodes.
        Commit => "commit" {
            /// The node owning this commit's local bindings.
            owner: Handle,
            /// The binder being committed.
            binder: Handle,
            /// The constraint checked before committing.
            constraint: Handle,
            /// Index into the declaration's runtime-requirement table for the commit's instance binding.
            instance: u32
        }
    }
}
tagged! {
    /// Send/receive, attempts/effects and compensation events keep separate roles.
    Event {
        /// A message send.
        Send => "send" {
            /// The channel the message is sent on.
            channel: Handle
        },
        /// A message receive.
        Receive => "receive" {
            /// The channel the message is received on.
            channel: Handle,
            /// The corresponding send event.
            send: Handle
        },
        /// An operation attempt.
        Attempt => "attempt" {
            /// The node owning this attempt.
            owner: Handle,
            /// The attempted operation.
            operation: ExportRef,
            /// The contract clauses this attempt is checked against.
            contracts: Vec<u32>,
            /// Index into the declaration's runtime-requirement table for the attempt's instance binding.
            instance: u32
        },
        /// An operation's committed effect.
        Effect => "effect" {
            /// The attempt this effect resulted from.
            attempt: Handle,
            /// Index into the declaration's runtime-requirement table for the effect's instance binding.
            instance: u32
        },
        /// A compensation event.
        Event => "event" {
            /// The node owning this compensation event.
            owner: Handle,
            /// The compensation being registered or activated, absent when none applies.
            compensation: Nullable<Handle>,
            /// Index into the declaration's runtime-requirement table for the event's instance binding.
            instance: u32
        }
    }
}
record! { /// Authored relationship occurrence between exact value owners.
    Related {
        /// The relationship's exported definition.
        relationship: u32,
        /// The relationship's source-side value.
        from: Handle,
        /// The relationship's target-side value.
        to: Handle,
        /// Where this relationship occurrence is declared in the original source.
        locus: Locus
    }
}
labels! {
    /// A control endpoint; entering and completing a control are distinct.
    Port {
        /// The control's entry point.
        Enter => "enter",
        /// The control's completion point.
        Exit => "exit"
    }
}
labels! {
    /// Derived causality, with bounded repeat progress as its explicit cyclic edge.
    EdgeKind {
        /// Sequential ordering between two controls.
        Sequence => "sequence",
        /// Branch selection into a choice's chosen case.
        Branch => "branch",
        /// Join of concurrent branches back into their parallel node.
        Join => "join",
        /// One repeat iteration's progress into the next.
        RepeatProgress => "repeat_progress",
        /// An await's success path.
        AwaitSuccess => "await_success",
        /// An await's timeout path.
        AwaitTimeout => "await_timeout"
    }
}
record! { /// Control node and the selected entry/completion port.
    Endpoint {
        /// The control node.
        node: Handle,
        /// The selected entry or completion port.
        port: Port
    }
}
record! { /// A claimed derived edge; only repeat progress has an authored maximum.
    CausalEdge {
        /// The edge's derived kind.
        kind: EdgeKind,
        /// The control node this edge is derived from.
        owner: Handle,
        /// The edge's source endpoint.
        from: Endpoint,
        /// The edge's target endpoint.
        to: Endpoint,
        /// The edge's authored maximum, present only for repeat-progress edges.
        maximum: Nullable<Integer>
    }
}
record! { /// Authored final predicate and separate complete-workflow closure binding.
    Finish {
        /// The finish obligation's own name.
        name: String,
        /// The finish obligation's own binder.
        binder: Handle,
        /// The predicate checked at workflow completion.
        constraint: Handle,
        /// Index into the declaration's runtime-requirement table for the closure binding.
        closure: u32,
        /// Where this finish obligation is declared in the original source.
        locus: Locus
    }
}
record! { /// Complete recovery obligation with distinct registration/activation/retry roles.
    Compensation {
        /// The compensation's own name.
        name: String,
        /// The forward operation's effect that this compensation reverses.
        forward_effect: Handle,
        /// The forward operation's own attempt.
        forward: Handle,
        /// The node owning this compensation's local bindings.
        owner: Handle,
        /// The compensating operation.
        operation: ExportRef,
        /// Index into the declaration's runtime-requirement table for the compensation's profile.
        profile: u32,
        /// Index into the declaration's runtime-requirement table for the selected clock.
        clock: u32,
        /// The compensation's registration evaluation point.
        registration_anchor: Handle,
        /// Index into the declaration's runtime-requirement table for the registration instance binding.
        registration_instance: u32,
        /// The values captured at registration.
        registration_captures: Vec<Handle>,
        /// The compensation's trigger.
        trigger: Handle,
        /// The compensation's activation guard.
        guard: Handle,
        /// The compensation's activation evaluation point.
        activation_anchor: Handle,
        /// The values captured at activation.
        activation_captures: Vec<Handle>,
        /// The authored retry window.
        within: Interval,
        /// The authored maximum number of attempts.
        maximum_attempts: Integer,
        /// Index into `types` for the attempt-instance identity type.
        attempt_type: u32,
        /// The compensation's earlier-attempt binder.
        earlier: Handle,
        /// The compensation's later-attempt binder.
        later: Handle,
        /// The compensation's retry binder.
        retry: Handle,
        /// Index into the declaration's runtime-requirement table for the attempt instance binding.
        attempt_instance: u32,
        /// Index into the declaration's runtime-requirement table for the effect instance binding.
        effect_instance: u32,
        /// The compensation's own commit, absent when the compensation is never committed.
        commit: Nullable<Handle>,
        /// The compensation's recovery predicate.
        recovery: Handle,
        /// The compensation's recovery binder.
        recover: Handle,
        /// Indices into the declaration's runtime-requirement table this recovery depends on.
        recovery_bindings: Vec<u32>,
        /// Where this compensation is declared in the original source.
        locus: Locus
    }
}
labels! {
    /// Requirements for later inputs; no runtime identity is supplied by these tags.
    BindingKind {
        /// A later-bound workflow instance identity.
        WorkflowInstance => "workflow_instance",
        /// A later-bound role instance identity.
        RoleInstance => "role_instance",
        /// A later-bound participant identity.
        Participant => "participant",
        /// A later-bound component identity.
        Component => "component",
        /// A later-bound channel endpoint identity.
        Endpoint => "endpoint",
        /// A later-bound message identity.
        Message => "message",
        /// A later-bound send-event identity.
        Send => "send",
        /// A later-bound receive-event identity.
        Receive => "receive",
        /// A later-bound delivery identity.
        Delivery => "delivery",
        /// A later-bound attempt identity.
        Attempt => "attempt",
        /// A later-bound effect identity.
        Effect => "effect",
        /// A later-bound compensation registration identity.
        CompensationRegistration => "compensation_registration",
        /// A later-bound compensation attempt identity.
        CompensationAttempt => "compensation_attempt",
        /// A later-bound compensation effect identity.
        CompensationEffect => "compensation_effect",
        /// A later-bound commit identity.
        Commit => "commit",
        /// A later-bound snapshot identity.
        Snapshot => "snapshot",
        /// A later-bound invocation identity.
        Invocation => "invocation",
        /// A later-bound population identity.
        Population => "population",
        /// A later-bound relationship instance identity.
        Relationship => "relationship",
        /// A later-bound clock identity.
        Clock => "clock",
        /// A later-bound temporal window identity.
        Window => "window",
        /// A later-bound observation identity.
        Observation => "observation",
        /// A later-bound repeat-progress identity.
        Progress => "progress",
        /// A later-bound closure (finish) identity.
        Closure => "closure",
        /// A later-bound capture-set identity.
        Capture => "capture"
    }
}
record! { /// Exact static authority, role and prerequisite contracts for a later binding.
    BindingRequirement {
        /// The binding's own name.
        name: String,
        /// The binding's runtime-identity role.
        kind: BindingKind,
        /// Index into `types` for the bound value's static type, absent when the binding has none.
        #[serde(rename = "type")] value_type: Nullable<u32>,
        /// The authority that must supply this binding.
        authority: ArtifactRef,
        /// Index into `sources` for the contract this binding is checked against.
        contract: u32,
        /// The model this binding is checked against, absent when none applies.
        model: Nullable<ExportRef>,
        /// The declaration-local node this binding is required by.
        subject: Subject,
        /// The evaluation point this binding is anchored to.
        anchor: Handle,
        /// The scope this binding is visible within.
        scope: Handle,
        /// Index into the declaration's runtime-requirement table for a required prior relation, absent when none applies.
        relation: Nullable<u32>,
        /// Indices into the declaration's runtime-requirement table this binding itself requires.
        requires: Vec<u32>,
        /// Where this binding requirement is declared in the original source.
        locus: Locus
    }
}
tagged! {
    /// The exact declaration-local subject of a runtime requirement.
    Subject {
        /// The declaration itself is the subject.
        Declaration => "declaration" {
            /// Index into the package's `declarations` table.
            declaration: u32
        },
        /// A protocol role is the subject.
        Role => "role" {
            /// The subject role.
            role: Handle
        },
        /// A protocol channel is the subject.
        Channel => "channel" {
            /// The subject channel.
            channel: Handle
        },
        /// A protocol control node is the subject.
        Control => "control" {
            /// The subject control node.
            control: Handle
        },
        /// A compensation obligation is the subject.
        Compensation => "compensation" {
            /// The subject compensation.
            compensation: Handle
        }
    }
}

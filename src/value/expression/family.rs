// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-065): the checked-package producer for `Value`'s
//! function-declaration and function-application forms -- the one family
//! slice this ticket migrates onto the `crate::family` contract.
//!
//! This module mints identity and provenance for the two migrated forms and
//! carries them through S4 linking (`link_function_identity`) and v2
//! emission (`emit_v2`/`decode_v2`). It does not replace
//! [`super::check::Typer`]'s typing, definedness or termination checking --
//! those algorithms are unchanged and stay exactly where they are; this
//! module is what makes their two function-form entry points
//! identity-bearing and is what the checked-package producer
//! ([`super::CheckedPackage::emit_function_package_v2`]) is built from.

use qsl_attrs::string_edge;
use sha2::{Digest, Sha256};

use quire_exact::{CollectionKind, Location, NodeKey, Origin, Role};

use crate::absence::AbsenceMode;
use crate::forms::{
    Accumulation, BinaryOperator, BinderQuery, Expression, FieldInitializer, FunctionDeclaration,
};
use crate::value::{IeeeWidth, QuantityUnit, RoundingMode, TextProfile, ValueType};

/// ADR-013 O-11: a non-empty sequence of identifiers, `::`-separated on
/// display -- the layer-6 `replay` facade's (and, for this ticket,
/// [`super::CheckedPackage::call`]'s) only function-selection key. Never a
/// bare `&str`; the one allowed name lookup (R-06) resolves this against a
/// checked package's declarations, and nothing compares it as a display
/// string (FR-065-AC-6).
///
/// `Serialize`/`Deserialize` (PR #262 review, finding F6) round-trip through
/// the same `::`-joined spelling [`std::fmt::Display`] and [`FromStr`]
/// already use, via `#[serde(try_from = "String", into = "String")]` --
/// `emit_v2`/`decode_v2`'s wire `name` field is typed on `QualifiedName`
/// itself now, not a bare `String` a caller has to re-parse and re-validate
/// (the same hole `call`'s own `&QualifiedName` parameter closes one
/// function over).
#[derive(
    Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct QualifiedName(Box<[String]>);

/// A `QualifiedName` must be one or more identifiers; this segment sequence
/// is not.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("a qualified name is a non-empty sequence of identifiers")]
pub struct InvalidQualifiedName;

fn is_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == b'_')
        && bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

impl QualifiedName {
    /// A qualified name from its segments, refusing anything that is not a
    /// non-empty sequence of identifiers.
    pub fn new(segments: Vec<String>) -> Result<Self, InvalidQualifiedName> {
        if !segments.is_empty() && segments.iter().all(|segment| is_identifier(segment)) {
            Ok(Self(segments.into_boxed_slice()))
        } else {
            Err(InvalidQualifiedName)
        }
    }

    /// A single-segment qualified name: an ordinary unqualified function
    /// name, the only shape `Value`'s Complete-V1 function family has.
    pub fn unqualified(name: impl Into<String>) -> Result<Self, InvalidQualifiedName> {
        Self::new(vec![name.into()])
    }

    pub(crate) fn segments(&self) -> &[String] {
        &self.0
    }

    /// This name's segments as a plain unqualified name, when it has
    /// exactly one segment. Complete-V1 declares no qualified names, so a
    /// multi-segment `QualifiedName` resolves against no declaration here.
    pub(crate) fn as_unqualified(&self) -> Option<&str> {
        match &self.0[..] {
            [name] => Some(name.as_str()),
            _ => None,
        }
    }
}

impl std::fmt::Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.segments().join("::"))
    }
}

impl std::str::FromStr for QualifiedName {
    type Err = InvalidQualifiedName;

    /// Parse `Display`'s own `::`-joined spelling back into segments.
    fn from_str(spelling: &str) -> Result<Self, Self::Err> {
        Self::new(spelling.split("::").map(str::to_owned).collect())
    }
}

impl TryFrom<String> for QualifiedName {
    type Error = InvalidQualifiedName;

    fn try_from(spelling: String) -> Result<Self, Self::Error> {
        spelling.parse()
    }
}

impl From<QualifiedName> for String {
    fn from(name: QualifiedName) -> Self {
        name.to_string()
    }
}

/// The declaring package's `name@version` a checked node's identity
/// preimage includes (ADR-013 O-04). Complete-V1's `PackageDeclarations` has
/// no package name/version of its own (unlike the outer domain-package
/// layer); every pre-migration caller of `PackageDeclarations::check`
/// (including its ~40 existing test call sites) keeps using the unchanged
/// `check` entry point and gets this default.
pub(crate) const DEFAULT_PACKAGE_IDENTITY: &str = "value.function-package@0.0.0-unversioned";

/// The contract-level `quire_exact::Meter`'s limits, for every call site in
/// this module and [`super`] that builds one just to satisfy
/// [`crate::family::CheckContext::new`]'s signature without itself wanting
/// to bound anything (`Self::check`/`Self::evaluate` are not metered against
/// this limit today -- see [`crate::family::contract`]'s own doc on
/// `_meter`). One `u64::MAX`-in-every-field literal, not six (PR #262
/// review, nit): each copy was a fact -- "this call site does not want a
/// scalar limit" -- restated by hand in ten fields, with nothing checking
/// the six copies stayed identical.
pub(crate) const SCALAR_LIMITS_UNLIMITED: quire_exact::ScalarLimits = quire_exact::ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn sha256(bytes: &[u8]) -> [u8; 32] {
    Sha256::digest(bytes).into()
}

/// A length-prefixed byte writer used only to mint identity preimages
/// (`mint_declaration_identity`/`mint_call_identity`, PR #262 review,
/// finding F2). `{:?}` (`Debug`) was rejected: `Debug` is documented by
/// `std` as not a stable serialization contract, so a field rename, an
/// added `#[derive(Debug)]` field, or a dependency changing its own `Debug`
/// impl would silently change every minted identity -- no compile error, no
/// failing test. Every write below goes through [`Self::write_bytes`],
/// which prepends the byte count before the bytes themselves, so two
/// distinct sequences of writes can never collide into the same combined
/// bytes -- the injectivity gap the same finding raised about
/// `declaration.name` interpolated next to `\0` separators (a name
/// containing `\0` used to blend into its neighbour; a length prefix makes
/// that impossible regardless of what the string contains). Every tag
/// written is an explicit `&'static str` literal chosen at its `match` arm,
/// never a derived discriminant, and every `match` below (`encode_expression`,
/// `encode_value_type`, and their small closed-enum helpers) is exhaustive:
/// adding a variant to `Expression`, `ValueType` or any nested enum this
/// preimage reads is a compile error here, forcing this file to pick an
/// explicit new tag, not a silent reinterpretation of the old bytes.
struct Preimage(Vec<u8>);

impl Preimage {
    fn write_bytes(&mut self, bytes: &[u8]) {
        self.0
            .extend_from_slice(&(bytes.len() as u64).to_le_bytes());
        self.0.extend_from_slice(bytes);
    }

    fn write_str(&mut self, text: &str) {
        self.write_bytes(text.as_bytes());
    }

    fn write_u64(&mut self, value: u64) {
        self.0.extend_from_slice(&value.to_le_bytes());
    }

    fn write_bool(&mut self, value: bool) {
        self.0.push(u8::from(value));
    }
}

fn binary_operator_tag(operator: BinaryOperator) -> &'static str {
    match operator {
        BinaryOperator::Add => "add",
        BinaryOperator::Subtract => "subtract",
        BinaryOperator::Multiply => "multiply",
        BinaryOperator::Divide => "divide",
        BinaryOperator::Equal => "equal",
        BinaryOperator::NotEqual => "not-equal",
        BinaryOperator::Less => "less",
        BinaryOperator::LessOrEqual => "less-or-equal",
        BinaryOperator::Greater => "greater",
        BinaryOperator::GreaterOrEqual => "greater-or-equal",
        BinaryOperator::And => "and",
        BinaryOperator::Or => "or",
        BinaryOperator::Implies => "implies",
    }
}

fn binder_query_tag(query: BinderQuery) -> &'static str {
    match query {
        BinderQuery::Map => "map",
        BinderQuery::Filter => "filter",
        BinderQuery::FlatMap => "flat-map",
        BinderQuery::Forall => "forall",
        BinderQuery::Exists => "exists",
    }
}

fn accumulation_tag(form: Accumulation) -> &'static str {
    match form {
        Accumulation::Fold => "fold",
        Accumulation::Reduce => "reduce",
    }
}

fn absence_mode_tag(mode: AbsenceMode) -> &'static str {
    match mode {
        AbsenceMode::Undefined => "undefined",
        AbsenceMode::Empty => "empty",
        AbsenceMode::Refused => "refused",
    }
}

fn collection_kind_tag(kind: CollectionKind) -> &'static str {
    match kind {
        CollectionKind::Sequence => "sequence",
        CollectionKind::Set => "set",
        CollectionKind::Bag => "bag",
        CollectionKind::OrderedSet => "ordered-set",
    }
}

fn rounding_mode_tag(mode: RoundingMode) -> &'static str {
    match mode {
        RoundingMode::Exact => "exact",
        RoundingMode::TowardZero => "toward-zero",
        RoundingMode::TowardPositive => "toward-positive",
        RoundingMode::TowardNegative => "toward-negative",
        RoundingMode::NearestEven => "nearest-even",
        RoundingMode::NearestAway => "nearest-away",
    }
}

fn text_profile_tag(profile: TextProfile) -> &'static str {
    match profile {
        TextProfile::UnicodeScalars => "unicode-scalars",
        TextProfile::Nfc => "nfc",
        TextProfile::Nfd => "nfd",
        TextProfile::Nfkc => "nfkc",
        TextProfile::Nfkd => "nfkd",
        TextProfile::BinaryUtf8 => "binary-utf8",
    }
}

fn encode_quantity_unit(out: &mut Preimage, unit: &QuantityUnit) {
    match unit {
        // Both arms read the unit's own already-content-addressed identity
        // (`Unit::key`/`CompoundUnit::identity`, `src/value/unit.rs`) rather
        // than re-deriving one from the unit's internal dimension/edge
        // graph: those identities are this codebase's own established
        // stable-identity mechanism (RFC 8785 JCS preimages, `value::node`),
        // not `Debug`.
        QuantityUnit::Declared(unit) => {
            out.write_str("declared");
            out.write_str(&unit.key().to_string());
        }
        QuantityUnit::Compound(unit) => {
            out.write_str("compound");
            out.write_str(&unit.identity().to_string());
        }
    }
}

fn encode_value_type(out: &mut Preimage, value_type: &ValueType) {
    match value_type {
        ValueType::Boolean => out.write_str("boolean"),
        ValueType::Integer => out.write_str("integer"),
        ValueType::Int(interval) => {
            out.write_str("int");
            out.write_str(&interval.lower().to_string());
            out.write_str(&interval.upper().to_string());
        }
        ValueType::Rational(domain) => {
            out.write_str("rational");
            out.write_str(&domain.numerator().lower().to_string());
            out.write_str(&domain.numerator().upper().to_string());
            out.write_str(&domain.denominator().lower().to_string());
            out.write_str(&domain.denominator().upper().to_string());
        }
        ValueType::Decimal(decimal) => {
            out.write_str("decimal");
            out.write_str(&decimal.lower().to_string());
            out.write_str(&decimal.upper().to_string());
            out.write_u64(u64::from(decimal.min_scale()));
            out.write_u64(u64::from(decimal.max_scale()));
            out.write_str(rounding_mode_tag(decimal.rounding()));
        }
        ValueType::Float(width) => {
            out.write_str("float");
            out.write_str(match width {
                IeeeWidth::Binary32 => "binary32",
                IeeeWidth::Binary64 => "binary64",
            });
        }
        ValueType::Quantity(unit) => {
            out.write_str("quantity");
            encode_quantity_unit(out, unit);
        }
        ValueType::Text(text_type) => {
            out.write_str("text");
            out.write_u64(text_type.min());
            out.write_u64(text_type.max());
            out.write_str(text_profile_tag(text_type.profile()));
        }
        ValueType::Enum(key) => {
            out.write_str("enum");
            out.write_str(&key.to_string());
        }
        ValueType::Option(payload) => {
            out.write_str("option");
            encode_value_type(out, payload);
        }
        ValueType::Composite(key) => {
            out.write_str("composite");
            out.write_str(&key.to_string());
        }
        ValueType::Collection(collection_type) => {
            out.write_str("collection");
            out.write_str(collection_kind_tag(collection_type.kind()));
            encode_value_type(out, collection_type.element());
            out.write_u64(collection_type.bound().minimum());
            out.write_u64(collection_type.bound().maximum());
        }
        ValueType::Reference(key) => {
            out.write_str("reference");
            out.write_str(&key.to_string());
        }
        ValueType::Population(maximum) => {
            out.write_str("population");
            out.write_u64(*maximum);
        }
    }
}

fn encode_field_initializer(out: &mut Preimage, initializer: &FieldInitializer) {
    match initializer {
        FieldInitializer::Value(expression) => {
            out.write_str("value");
            encode_expression(out, expression);
        }
        FieldInitializer::Null => out.write_str("null"),
    }
}

fn encode_expression(out: &mut Preimage, expr: &Expression) {
    match expr {
        Expression::Boolean(value) => {
            out.write_str("boolean");
            out.write_bool(*value);
        }
        Expression::Integer(value) => {
            out.write_str("integer");
            out.write_str(&value.to_string());
        }
        Expression::Rational(numerator, denominator) => {
            out.write_str("rational");
            out.write_str(&numerator.to_string());
            out.write_str(&denominator.to_string());
        }
        Expression::Name(name) => {
            out.write_str("name");
            out.write_str(name);
        }
        Expression::Let { name, value, body } => {
            out.write_str("let");
            out.write_str(name);
            encode_expression(out, value);
            encode_expression(out, body);
        }
        Expression::If {
            condition,
            then,
            otherwise,
        } => {
            out.write_str("if");
            encode_expression(out, condition);
            encode_expression(out, then);
            encode_expression(out, otherwise);
        }
        Expression::Binary {
            operator,
            left,
            right,
        } => {
            out.write_str("binary");
            out.write_str(binary_operator_tag(*operator));
            encode_expression(out, left);
            encode_expression(out, right);
        }
        Expression::Negate(operand) => {
            out.write_str("negate");
            encode_expression(out, operand);
        }
        Expression::Not(operand) => {
            out.write_str("not");
            encode_expression(out, operand);
        }
        Expression::Field { operand, field } => {
            out.write_str("field");
            encode_expression(out, operand);
            out.write_str(field);
        }
        Expression::Present(operand) => {
            out.write_str("present");
            encode_expression(out, operand);
        }
        Expression::Value(operand) => {
            out.write_str("value");
            encode_expression(out, operand);
        }
        Expression::Deref(operand) => {
            out.write_str("deref");
            encode_expression(out, operand);
        }
        Expression::Call { name, arguments } => {
            out.write_str("call");
            out.write_str(name);
            out.write_u64(arguments.len() as u64);
            for argument in arguments {
                encode_expression(out, argument);
            }
        }
        Expression::Record { name, fields } => {
            out.write_str("record");
            out.write_str(name);
            out.write_u64(fields.len() as u64);
            for (field_name, initializer) in fields {
                out.write_str(field_name);
                encode_field_initializer(out, initializer);
            }
        }
        Expression::Collection { kind, elements } => {
            out.write_str("collection");
            out.write_str(collection_kind_tag(*kind));
            out.write_u64(elements.len() as u64);
            for element in elements {
                encode_expression(out, element);
            }
        }
        Expression::Convert { target, operand } => {
            out.write_str("convert");
            encode_value_type(out, target);
            encode_expression(out, operand);
        }
        Expression::Query {
            query,
            binder,
            source,
            body,
        } => {
            out.write_str("query");
            out.write_str(binder_query_tag(*query));
            out.write_str(binder);
            encode_expression(out, source);
            encode_expression(out, body);
        }
        Expression::Flatten(operand) => {
            out.write_str("flatten");
            encode_expression(out, operand);
        }
        Expression::Accumulate {
            form,
            accumulator_type,
            accumulator,
            binder,
            source,
            step,
            identity,
        } => {
            out.write_str("accumulate");
            out.write_str(accumulation_tag(*form));
            out.write_str(accumulator_type);
            out.write_str(accumulator);
            out.write_str(binder);
            encode_expression(out, source);
            encode_expression(out, step);
            out.write_bool(identity.is_some());
            if let Some(identity) = identity {
                encode_expression(out, identity);
            }
        }
        Expression::Count {
            result_type,
            binder,
            source,
            predicate,
        } => {
            out.write_str("count");
            out.write_str(result_type);
            out.write_str(binder);
            encode_expression(out, source);
            encode_expression(out, predicate);
        }
        Expression::Sum {
            result_type,
            binder,
            source,
            summand,
        } => {
            out.write_str("sum");
            out.write_str(result_type);
            out.write_str(binder);
            encode_expression(out, source);
            encode_expression(out, summand);
        }
        Expression::Size(operand) => {
            out.write_str("size");
            encode_expression(out, operand);
        }
        Expression::Contains { collection, item } => {
            out.write_str("contains");
            encode_expression(out, collection);
            encode_expression(out, item);
        }
        Expression::AllInstances { target, population } => {
            out.write_str("all-instances");
            encode_value_type(out, target);
            encode_expression(out, population);
        }
        Expression::Lookup {
            target,
            population,
            reference,
            absence,
        } => {
            out.write_str("lookup");
            encode_value_type(out, target);
            encode_expression(out, population);
            encode_expression(out, reference);
            out.write_str(absence_mode_tag(*absence));
        }
        Expression::Dispatch {
            receiver,
            member,
            arguments,
        } => {
            out.write_str("dispatch");
            encode_expression(out, receiver);
            out.write_str(member);
            out.write_u64(arguments.len() as u64);
            for argument in arguments {
                encode_expression(out, argument);
            }
        }
        Expression::Pre(operand) => {
            out.write_str("pre");
            encode_expression(out, operand);
        }
    }
}

/// Mint a function declaration's identity (FR-062: "content-addressed...
/// over the node's structure and its declaring package's `name@version`"):
/// a SHA-256 over the package identity and the declaration's own **parsed**
/// structure -- name, parameters, result, measure and body, as authored,
/// before any name is resolved to a `Vec` index.
///
/// Hashing the parsed form, not the checked/typed tree, is what makes this
/// identity independent of unrelated declarations' order (FR-065-AC-2): a
/// typed body's `NodeKind::Call { function: usize, .. }` names a callee by
/// its position in the package's function list, which shifts when unrelated
/// declarations are reordered; the parsed `Expression::Call { name, .. }` a
/// declaration was authored with never does, so nothing this preimage reads
/// changes when a declaration elsewhere in the package moves.
///
/// This is a pragmatic content-address, not a claim of interop with the
/// external `quire.checked-package-id/v2` `ApplicationNode`/`PreimageTerm`
/// schema (`resources/complete-value/.../node-identity-preimage.schema.
/// json`): that schema's `Operation` identity for an arbitrary applied
/// operator has no landed implementation this ticket could follow for a
/// user-declared function, and building one from scratch is out of this
/// migration's scope (`crate::family`'s module doc). Structural identity
/// within one check run -- what FR-062-AC-2 and FR-065-AC-2 actually test --
/// holds regardless.
///
/// The preimage is [`Preimage`]'s explicit, length-prefixed byte encoding
/// (PR #262 review, finding F2), not `{:?}` (`Debug`) formatting -- see
/// [`Preimage`]'s own doc for why.
pub(crate) fn mint_declaration_identity(
    package_identity: &str,
    declaration: &FunctionDeclaration,
) -> NodeKey {
    let mut preimage = Preimage(Vec::new());
    preimage.write_str("value.function-declaration");
    preimage.write_str(package_identity);
    preimage.write_str(&declaration.name);
    preimage.write_u64(declaration.parameters.len() as u64);
    for (name, value_type) in &declaration.parameters {
        preimage.write_str(name);
        encode_value_type(&mut preimage, value_type);
    }
    encode_value_type(&mut preimage, &declaration.result);
    preimage.write_bool(declaration.measure.is_some());
    if let Some(measure) = &declaration.measure {
        encode_expression(&mut preimage, measure);
    }
    encode_expression(&mut preimage, &declaration.body);
    NodeKey::from_digest(sha256(&preimage.0))
}

/// Mint a function-application occurrence's identity, from the call's own
/// parsed structure -- the callee's syntactic name and its arguments' parsed
/// form -- never from a resolved `Vec` index. See
/// [`mint_declaration_identity`]'s doc for why an index would be unsafe
/// here, and [`Preimage`]'s doc for why this is not `{:?}` formatting.
pub(crate) fn mint_call_identity(
    package_identity: &str,
    callee_name: &str,
    arguments: &[Expression],
) -> NodeKey {
    let mut preimage = Preimage(Vec::new());
    preimage.write_str("value.function-application");
    preimage.write_str(package_identity);
    preimage.write_str(callee_name);
    preimage.write_u64(arguments.len() as u64);
    for argument in arguments {
        encode_expression(&mut preimage, argument);
    }
    NodeKey::from_digest(sha256(&preimage.0))
}

/// One source occurrence of a migrated form, keyed by (identity, role,
/// ordinal) (ADR-013 O-07) and mapped to its source span (ADR-013 O-12).
/// QSL is the only minter (FR-062 "Provenance").
///
/// `S` is the span type. Complete-V1's own function forms have no lexed
/// byte offsets to report (there is no text parser for this API-constructed
/// family -- `super::check`'s own [`super::refusal::Location`] is its
/// existing span analogue: a declaration plus a child-index path). A test
/// exercising this generically with a `(u32, u32)` byte-offset stand-in is
/// still exercising the real mechanism: ordinal assignment and lookup by
/// (identity, role, ordinal) do not depend on what a span actually is.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Occurrence<S> {
    pub(crate) location: Location,
    pub(crate) span: S,
}

/// A checked node's identity together with every source occurrence recorded
/// for it so far (its own declaration occurrence, plus one "reference"
/// occurrence per call site that resolves to it).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OccurrenceMap<S> {
    entries: Vec<Occurrence<S>>,
}

// A hand-written `Default`, not `#[derive(Default)]`: the derive macro adds
// an `S: Default` bound even though `Vec::default()` needs none -- a known
// derive-macro imprecision, not a real requirement on the span type.
impl<S> Default for OccurrenceMap<S> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

impl<S: Clone + PartialEq> OccurrenceMap<S> {
    /// Record one occurrence of `identity` under `role`, at `span`. Ordinals
    /// are assigned by (identity, role) insertion order (ADR-013 O-07: "an
    /// ordinal disambiguating repeated occurrences of that role on the same
    /// node") -- reordering *other* nodes' occurrences never changes this
    /// one's ordinal, only its own role's own repeat count does.
    pub(crate) fn record(&mut self, identity: NodeKey, role: &str, span: S) -> Origin {
        // PR #262 review, finding F5: `role().as_str() == role` compared the
        // newtype's lexical spelling as a bare string; `Role` derives
        // `PartialEq` itself, so build it once and compare the newtype
        // directly -- an occurrence-role key, not a `string_edge` (ADR-012
        // §9's target is a string selecting semantics; this compares one
        // already-typed value to another).
        let role = Role::new(role);
        let ordinal = self
            .entries
            .iter()
            .filter(|occurrence| {
                occurrence.location.node() == identity
                    && occurrence.location.occurrence().role() == &role
            })
            .count() as u64;
        let origin = Origin::new(role, ordinal);
        self.entries.push(Occurrence {
            location: Location::new(identity, origin.clone()),
            span,
        });
        origin
    }

    /// The span recorded for `identity` at exactly `origin`, if any.
    pub(crate) fn resolve(&self, identity: NodeKey, origin: &Origin) -> Option<&S> {
        self.entries
            .iter()
            .find(|occurrence| {
                occurrence.location.node() == identity && occurrence.location.occurrence() == origin
            })
            .map(|occurrence| &occurrence.span)
    }
}

/// One entry in the checked-package producer's minimal v2 encoding: a
/// declared function's qualified name and its checked identity.
///
/// `deny_unknown_fields` (PR #262 review, finding F16): decoded through the
/// `pub` [`decode_function_package_v2`], from bytes an external caller
/// supplies, not only from this crate's own `emit_v2` output -- an unknown
/// field should refuse, not silently disappear.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FunctionEntryV2 {
    /// Typed on `QualifiedName` (PR #262 review, finding F6), not a bare
    /// `String`: an entry whose wire spelling is not a valid qualified name
    /// fails `serde_json::from_slice` itself (via `QualifiedName`'s own
    /// `#[serde(try_from = "String")]`), which `decode_v2` already maps to
    /// `DecodeV2Error::Malformed` -- no separate malformed-name variant
    /// needed.
    name: QualifiedName,
    identity: String,
}

/// `quire.checked-function-package/v2`: this ticket's self-consistent v2
/// encoding for function-declaration identity. It is not a claim of
/// conformance to the external `quire.checked-package-id/v2` schema (see
/// [`mint_declaration_identity`]'s doc) -- it exists to demonstrate, and let
/// a test assert, that identity survives check, S4 linking and a v2
/// emit/decode round trip unchanged (FR-065-AC-2).
const FUNCTION_PACKAGE_V2_VERSION: &str = "quire.checked-function-package/v2";

/// `deny_unknown_fields` (PR #262 review, finding F16): see
/// [`FunctionEntryV2`]'s own doc -- same reason, same externally-decoded
/// input.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct FunctionPackageV2 {
    version: String,
    functions: Vec<FunctionEntryV2>,
}

/// S4 linking, for this migration's minimal scope: a checked function's
/// identity carried forward unchanged. Complete-V1 has no cross-package
/// import to resolve for function declarations, so linking here is
/// genuinely a pass-through -- named and exercised as its own step (rather
/// than folded into `check`) so FR-065-AC-2's three checkpoints (after
/// `check`, after linking, after v2 decode) are three real, distinct calls.
pub(crate) fn link_function_identity(identity: NodeKey) -> NodeKey {
    identity
}

/// Emit v2 bytes for a linked set of (qualified name, identity) pairs.
pub(crate) fn emit_v2(functions: &[(QualifiedName, NodeKey)]) -> Vec<u8> {
    let package = FunctionPackageV2 {
        version: FUNCTION_PACKAGE_V2_VERSION.to_owned(),
        functions: functions
            .iter()
            .map(|(name, identity)| FunctionEntryV2 {
                name: name.clone(),
                identity: identity.to_string(),
            })
            .collect(),
    };
    serde_json::to_vec(&package).expect("FunctionPackageV2 always serializes")
}

/// Why v2 bytes could not be decoded.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum DecodeV2Error {
    /// The bytes did not parse as `FunctionPackageV2`'s JSON shape at all
    /// (`serde_json::from_slice` failed).
    #[error("malformed quire.checked-function-package/v2 bytes")]
    Malformed,
    /// The bytes parsed, but the package's `version` field did not match
    /// this module's `FUNCTION_PACKAGE_V2_VERSION`.
    #[error("unrecognised version")]
    Version,
    // PR #262 review (F16): this used to say "64 lowercase hex digits",
    // which `decode_hex_32` never enforced -- it accepts either case via
    // `is_ascii_hexdigit`. `emit_v2` always emits lowercase
    // (`NodeKey`'s `Display` impl, `quire-exact/src/node.rs`), so nothing
    // this crate produces is ever uppercase, but decode is genuinely
    // case-insensitive; the message now says what the code does.
    /// An entry's `identity` string was not 64 hex digits (either case
    /// accepted; see the note above on why the message doesn't say
    /// "lowercase").
    #[error("identity is not 64 hex digits")]
    InvalidIdentity,
}

/// Decode v2 bytes back into (qualified name, identity) pairs, refusing an
/// unrecognised version or a malformed identity rather than guessing.
///
/// `#[string_edge]` (PR #262 review, finding F5): the `package.version !=
/// FUNCTION_PACKAGE_V2_VERSION` wire-version gate below is one of ADR-012
/// §9's own listed edges ("the typed v2 reader"), but compares against a
/// named `const`, not a literal, so `xtask string-edge`'s literal-operand
/// scan cannot see it -- it was unmarked and undetected until this pass.
/// Marking it declares this reader as the sanctioned edge ADR-012 §9
/// already lists it as, independent of whatever the scanner's own
/// const-operand blind spot does or does not catch.
#[string_edge]
pub(crate) fn decode_v2(bytes: &[u8]) -> Result<Vec<(QualifiedName, NodeKey)>, DecodeV2Error> {
    let package: FunctionPackageV2 =
        serde_json::from_slice(bytes).map_err(|_| DecodeV2Error::Malformed)?;
    if package.version != FUNCTION_PACKAGE_V2_VERSION {
        return Err(DecodeV2Error::Version);
    }
    package
        .functions
        .into_iter()
        .map(|entry| {
            let bytes = decode_hex_32(&entry.identity).ok_or(DecodeV2Error::InvalidIdentity)?;
            Ok((entry.name, NodeKey::from_digest(bytes)))
        })
        .collect()
}

fn decode_hex_32(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let (pairs, []) = hex.as_bytes().as_chunks::<2>() else {
        return None;
    };
    let mut out = [0_u8; 32];
    for (slot, pair) in out.iter_mut().zip(pairs) {
        let text = std::str::from_utf8(pair).ok()?;
        *slot = u8::from_str_radix(text, 16).ok()?;
    }
    Some(out)
}

/// FR-062/FR-065: `Value`'s function-declaration and function-application
/// forms, reached through [`crate::family::FamilyContract`] and
/// [`crate::family::ReferenceEvaluation`]. A marker type -- every method is
/// a bare associated function over `Self::Form`/`Self::Checked`, with no
/// instance state (ADR-012 §2's contract is static, dispatched through
/// closed enums, not through an object).
pub(crate) struct ValueFunctionFamily;

impl crate::family::FamilyContract for ValueFunctionFamily {
    type Form = FunctionDeclaration;
    /// The minted identity is this contract's checked payload for a
    /// declaration: everything an `evaluate` caller needs to re-find the
    /// declaration's own checked body is already in `CheckedFunction`
    /// (unchanged by this contract); what `check` adds is the identity, so
    /// that is what it hands back.
    type Checked = NodeKey;
    /// The declaring package's `name@version` (see
    /// [`mint_declaration_identity`]'s doc for why Complete-V1 has no real
    /// one of its own yet).
    type Declarations = String;

    fn check(
        form: &Self::Form,
        cx: &mut crate::family::CheckContext<'_, String>,
    ) -> crate::family::CheckOutcome<NodeKey> {
        // FR-062 "Explicit limits bound every stage entry, including
        // recursion": `check` charges one nesting-entry before minting,
        // and refuses with a `Limit` outcome rather than reading `form` at
        // all once the configured depth is reached (FR-062-AC-7).
        cx.enter_nesting()
            .map_err(crate::family::StageFailure::Limit)?;
        // FR-062-AC-3 "no side door": the scope stack is pushed and popped
        // around this one check (`cx.scopes.enter`/`leave` below), not just
        // read.
        cx.scopes
            .enter(format!("value.function-declaration:{}", form.name));
        let identity = mint_declaration_identity(cx.declarations(), form);
        // PR #262 review (F7): an earlier version of this function
        // recomputed `mint_declaration_identity` a second time here and
        // returned `StageFailure::Fault` on a mismatch, framed as a
        // "defensive" internal-invariant check. It was not: comparing a
        // pure function's output against itself, called twice with the
        // same arguments, cannot fail -- the two calls are definitionally
        // equal, not equal because anything was verified. Deleted along
        // with `StageFailure::Fault`/`InternalFault` themselves (see
        // `crate::family::outcome::StageFailure`'s own doc).
        cx.diagnostics.record(
            cx.scopes,
            format!(
                "{}: checked function declaration {} (limit={}, meter admissions={})",
                crate::family::FamilyKind::Value.catalog_code_prefix(),
                form.name,
                cx.limits().nesting_depth,
                cx.meter.admitted_charges().len(),
            ),
        );
        cx.scopes.leave();
        // PR #262 review (coordinator round 3): an earlier version of this
        // function also asserted `cx.scopes.depth() == depth_before` here.
        // In this straight-line body, one `enter` four lines above is
        // followed by exactly one `leave`, with nothing between them that
        // could push or pop again -- the assertion restated what the two
        // calls already guarantee by construction, not something a broken
        // implementation could trip. `ScopeStack::depth`, that assertion's
        // only reader, is deleted with it.
        cx.leave_nesting();
        Ok(crate::family::Staged::new(identity))
    }
}

/// `ValueFunctionFamily`'s real evaluation environment (ADR-012 §2's
/// `evaluate` hook, FR-062-AC-1/AC-6): the checked package `checked`'s
/// identity resolves against, the caller's object environment and its own
/// accounting meter -- all borrowed for the one call, never owned by the
/// family marker type. [`super::CheckedPackage::call`] is this environment's
/// one real (non-test) constructor.
pub(crate) struct EvaluationEnv<'a> {
    pub(crate) package: &'a super::CheckedPackage,
    pub(crate) objects: &'a super::super::reference::ObjectEnvironment,
    pub(crate) arguments: Option<Vec<super::super::composite::Value>>,
    pub(crate) local_meter: &'a mut super::super::accounting::Meter,
}

impl crate::family::ReferenceEvaluation for ValueFunctionFamily {
    type Observed = super::Evaluation;
    type Env<'a> = EvaluationEnv<'a>;

    /// FR-062-AC-6: reads only `checked` (a bare identity) and `env`'s
    /// checked package/object environment/meter -- no CST, token or
    /// display string. `_meter` (the shared kernel meter every family's
    /// `evaluate` takes) is accepted but not charged: `quire_exact::Meter`'s
    /// `charge`/`charge_plan` are `pub(crate)` inside `quire-exact`, not
    /// exported here (see [`crate::family::EvaluateRefusal`]'s doc) --
    /// `Value`'s own evaluation charges `env.local_meter` (its pre-existing
    /// accounting meter) instead, unchanged by this contract.
    fn evaluate<'a>(
        checked: &NodeKey,
        env: &mut EvaluationEnv<'a>,
        _meter: &mut quire_exact::Meter,
    ) -> Result<super::Evaluation, crate::family::EvaluateRefusal> {
        let function = env
            .package
            .function_by_identity(*checked)
            .ok_or(crate::family::EvaluateRefusal::UnknownIdentity { identity: *checked })?;
        // PR #262 review (F17): `.take().unwrap_or_default()` used to
        // silently evaluate with zero arguments if `arguments` were ever
        // `None` -- which, since `take()` itself leaves it `None`, is
        // exactly what a second `evaluate` call on the same `env` would
        // hit, and it would look like a legitimate zero-argument call
        // rather than the reused-env bug it actually is.
        // `EvaluationEnv`'s one real constructor (`CheckedPackage::call`)
        // always builds a fresh env with `Some(arguments)` and calls
        // `evaluate` exactly once, so this refusal is unreached today; it
        // exists so a future second call surfaces as a typed refusal
        // instead of a wrong, silent answer. Its own distinct variant, not
        // `UnknownIdentity`'s (PR #262 review, finding F3, this round): the
        // two are different conditions, and the one caller mapping this
        // into a public `InputRefusal` (`value::expression::mod.rs`'s
        // `CheckedPackage::call`) must be able to tell them apart.
        let arguments = env
            .arguments
            .take()
            .ok_or(crate::family::EvaluateRefusal::EnvironmentAlreadyConsumed)?;
        let callables = env.package.callables();
        Ok(super::evaluate::Machine::new(
            &env.package.scope,
            &callables,
            env.objects,
            env.local_meter,
            &env.package.dispatch_tables,
        )
        .run(&function.body, function.slots, arguments, true))
    }
}

// FR-062-AC-8/FR-063-AC-6 (ADR-012 §5.1 S4, "each family Cause enum's
// catalog_code()") is deferred, not delivered by this ticket, and there is
// deliberately no `DeclarationCause` type here to carry it.
//
// An earlier version of this file kept an uninhabited `DeclarationCause`
// with a `#[cfg(seam_probe)]` probe variant, on the theory that the probe
// build would demonstrate S4 the same way it demonstrates S1 (`FamilyKind`).
// It does not: `FamilyKind`'s probe works because six real variants and
// real production `match`es already exist, so adding a seventh variant
// breaks matches nothing else could reach otherwise -- a real author has to
// notice. `DeclarationCause` had no real variant and no production `match`
// beside its own `catalog_code()`; the only thing that could ever fail
// under the probe was that same function gaining one more arm. That is the
// construct testing itself, not a seam. Value's function-declaration
// `check` mints identity unconditionally once nesting is charged, so it has
// no typed refusal cause -- a family with zero causes cannot demonstrate a
// per-family cause seam, the same shape `Requirements`, `PackageRefusal` and
// `StageFailure::Refused` were. QSL-152 owns adding a real `Cause` enum,
// `catalog_code()` and the S4 probe variant for the first family that has
// one, validated against real content instead of guessed here
// (FR-062-AC-8's S4 seam-probe coverage).

#[cfg(test)]
mod family_contract_tests {
    use super::*;
    use crate::family::{
        CheckContext, DiagnosticSink, EvaluateRefusal, FamilyContract, ReferenceEvaluation,
        ScopeStack, StageLimits,
    };
    use crate::value::accounting::{Meter as ValueMeter, ScalarLimits as ValueScalarLimits};
    use crate::value::composite::{TypeEnvironment, ValueType};
    use crate::value::expression::{CheckingLimits, PackageDeclarations};
    use crate::value::reference::ObjectEnvironment;
    use quire_exact::Meter;

    // `EvaluationEnv::local_meter` is this crate's own `value::accounting::
    // Meter` (`ValueFunctionFamily::evaluate`'s pre-existing accounting
    // path); `ReferenceEvaluation::evaluate`'s own `_meter` parameter is
    // the shared kernel `quire_exact::Meter` (`Meter`, imported above) --
    // two distinct types with the same name in different crates, both
    // needed by `evaluate_refuses_a_second_call_on_the_same_env`.
    const VALUE_SCALAR_UNLIMITED: ValueScalarLimits = ValueScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    };

    fn limits() -> StageLimits {
        StageLimits { nesting_depth: 128 }
    }

    fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(name, Vec::new(), ValueType::Boolean, None, body)
    }

    /// `Value`'s function-declaration family is a real `FamilyContract`
    /// implementation, reachable through the trait, not a free-standing
    /// function with no shared associated-type binding -- and its minted
    /// identity survives a real v2 emit/decode round trip through
    /// `family::emit_v2`/`decode_v2` directly (PR #262 review, F1/F2:
    /// no longer routed through the deleted `FamilyContract::package`,
    /// which nothing consumed). Untagged for FR-062-AC-1 (PR #262 review,
    /// finding F3): AC-1 requires all six contract parts as compile-time
    /// obligations, and this trait now has only `check` -- see FR-062's own
    /// amended Acceptance Criteria for why AC-1 is recorded unbacked rather
    /// than retagged onto a narrower claim.
    #[test]
    fn value_function_family_checks_through_the_contract() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &package_identity,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        let expected = mint_declaration_identity(&package_identity, &form);
        let staged = ValueFunctionFamily::check(&form, &mut cx).unwrap();
        assert_eq!(staged.value, expected);
        assert_eq!(diagnostics.entries().len(), 1);
        let declaration_name = QualifiedName::unqualified("declaration").unwrap();
        let v2 = emit_v2(&[(declaration_name.clone(), staged.value)]);
        assert_eq!(decode_v2(&v2).unwrap(), vec![(declaration_name, expected)]);
    }

    /// PR #262 review, finding F17 (round 3, item 8): a second `evaluate`
    /// call on the same `EvaluationEnv` -- whose `arguments` the first call
    /// already consumed via `.take()` -- refuses with a typed
    /// `EvaluateRefusal` rather than silently evaluating with zero
    /// arguments. `EvaluationEnv`'s one real (non-test) constructor
    /// (`CheckedPackage::call`) never reaches this: it always builds a
    /// fresh env with `Some(arguments)` and calls `evaluate` exactly once,
    /// so the guard is unreached through that path. This test bypasses
    /// that constructor -- the same thing the F17 finding's own fix
    /// verified by hand and then reverted, leaving the fix itself
    /// unguarded -- and lands the guard with a real second call.
    #[test]
    fn evaluate_refuses_a_second_call_on_the_same_env() {
        let package = PackageDeclarations {
            functions: vec![declaration("f", Expression::Boolean(true))],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("one boolean-literal function checks cleanly");
        let identity = package
            .function_identity("f")
            .expect("f is declared in this package");
        let objects = ObjectEnvironment::new(&TypeEnvironment::default(), []).unwrap();
        let mut local_meter = ValueMeter::new(VALUE_SCALAR_UNLIMITED);
        let mut env = EvaluationEnv {
            package: &package,
            objects: &objects,
            arguments: Some(Vec::new()),
            local_meter: &mut local_meter,
        };
        let mut contract_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
            .expect("first call, with real arguments still present, evaluates cleanly");
        let refusal = ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
            .expect_err("second call on the same env, arguments already consumed, must refuse");
        assert_eq!(refusal, EvaluateRefusal::EnvironmentAlreadyConsumed);
    }

    /// Two independently constructed contexts each observe exactly one
    /// diagnostic from checking the same form -- if `check` wrote through
    /// any shared/global state instead of `cx.diagnostics`, one of the two
    /// independent sinks would show zero or more than one entry (PR #262
    /// review, finding F6: the previous version of this test compared
    /// `diagnostics_a`'s count against an unrelated, freshly constructed
    /// `DiagnosticSink::default()` rather than against `diagnostics_b`, so
    /// it never actually observed `cx_b`'s own state, and separately
    /// asserted a pure function's output against itself by comparing
    /// `staged_a.value` to `staged_b.value` -- both deleted).
    ///
    /// **Untagged (PR #262 review, coordinator round 3, finding 5).** This
    /// test was tagged `FR-062-AC-3`, whose central clause is that two
    /// typing contexts checking the same declarations produce *identical
    /// checked output* -- F6 correctly deleted the `staged_a.value ==
    /// staged_b.value` self-comparison that used to (fabricatedly) stand in
    /// for that, but kept the tag on what remained: two counts, each
    /// asserted only against the literal `1` the loop below guarantees by
    /// construction, not against each other's checked output. That is a
    /// real isolation test, not an identical-output test, so it is untagged
    /// rather than left claiming to back a criterion it does not; see
    /// FR-062's own amended Acceptance Criteria for AC-3's current status.
    #[test]
    fn two_contexts_from_the_same_declarations_check_identically() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let scalar_limits = SCALAR_LIMITS_UNLIMITED;
        let form = declaration("f", Expression::Boolean(true));

        let mut meter_a = Meter::new(scalar_limits);
        let mut diagnostics_a = DiagnosticSink::default();
        let mut scopes_a = ScopeStack::default();
        let mut cx_a = CheckContext::new(
            &package_identity,
            limits(),
            &mut meter_a,
            &mut diagnostics_a,
            &mut scopes_a,
        );
        ValueFunctionFamily::check(&form, &mut cx_a).unwrap();

        let mut meter_b = Meter::new(scalar_limits);
        let mut diagnostics_b = DiagnosticSink::default();
        let mut scopes_b = ScopeStack::default();
        let mut cx_b = CheckContext::new(
            &package_identity,
            limits(),
            &mut meter_b,
            &mut diagnostics_b,
            &mut scopes_b,
        );
        ValueFunctionFamily::check(&form, &mut cx_b).unwrap();

        // Each independently constructed sink shows exactly its own one
        // entry -- a shared/global sink would leak entries into whichever
        // one ran second, or show two entries in one and zero in the other.
        assert_eq!(diagnostics_a.entries().len(), 1);
        assert_eq!(diagnostics_b.entries().len(), 1);
    }

    /// **Untagged.** This test varies only the
    /// nesting-depth limit (0 vs 1) against `check`, which calls
    /// `enter_nesting` exactly once per top-level declaration -- `check`
    /// performs no recursive descent of its own, so `depth` never exceeds
    /// 1 and the predicate this test exercises reduces to `0 >=
    /// nesting_depth`. Mutation confirms it: deleting `self.depth += 1`
    /// from `CheckContext::enter_nesting` (removing the nesting bound
    /// entirely) leaves this test passing unchanged, because it never
    /// calls `enter_nesting` more than once to observe the increment.
    /// FR-062-AC-7 requires a fixture nested to depth D and a limit varied
    /// by exactly one at D -- that needs a real recursive-descent fixture,
    /// which does not exist against today's `check` (QSL-148 owns moving
    /// real recursive checking into `ValueFunctionFamily::check`; see its
    /// own scope note). This test still guards a real, narrower property
    /// (the nesting-depth limit is `>=`-checked at all) and stays for
    /// that, untagged rather than claiming AC-7.
    #[test]
    fn nesting_depth_limit_is_the_proximate_cause() {
        let package_identity = DEFAULT_PACKAGE_IDENTITY.to_owned();
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut tight = StageLimits { nesting_depth: 0 };
        let mut cx = CheckContext::new(
            &package_identity,
            tight,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        let refused = ValueFunctionFamily::check(&form, &mut cx);
        assert!(matches!(
            refused,
            Err(crate::family::StageFailure::Limit(_))
        ));

        tight.nesting_depth = 1;
        let mut cx = CheckContext::new(
            &package_identity,
            tight,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let admitted = ValueFunctionFamily::check(&form, &mut cx);
        assert!(admitted.is_ok());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::composite::ValueType;
    use ix_trace_rs::trace;

    fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(name, Vec::new(), ValueType::Boolean, None, body)
    }

    /// FR-062-AC-2/FR-065-AC-2: two structurally identical declarations mint
    /// one identity; a name change mints a different one.
    #[trace("TC-160", "FR-062-AC-2")]
    #[test]
    fn identical_declarations_share_one_identity() {
        let a = declaration("f", Expression::Boolean(true));
        let b = declaration("f", Expression::Boolean(true));
        let c = declaration("g", Expression::Boolean(true));
        assert_eq!(
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &a),
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &b)
        );
        assert_ne!(
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &a),
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &c)
        );
    }

    // `identity_ignores_unrelated_declarations` (PR #262 review, coordinator
    // round 3) is deleted from here. It minted `DEFAULT_PACKAGE_IDENTITY`'s
    // identity for the same `target` twice and compared the result to
    // itself -- no second declaration was ever constructed, so FR-065-AC-2's
    // reordering clause had nothing to be independent *of*; its own comment
    // conceded "this is definitionally true". A real reordering test needs
    // two actual declarations checked in two actual orders, which
    // `mint_declaration_identity`'s single-declaration signature cannot
    // exercise -- see
    // `tests/dispatch_calls.rs`'s
    // `function_identity_survives_reordering_check_linking_and_a_v2_round_trip`,
    // built at the `PackageDeclarations::check` level instead, where
    // position could actually leak.

    /// FR-062-AC-2: two occurrences of one identity get distinct ordinals;
    /// a different identity's occurrence does not consume an ordinal from
    /// this one.
    #[trace("TC-160", "FR-062-AC-2")]
    #[test]
    fn occurrence_ordinals_are_per_identity_and_role() {
        let mut map = OccurrenceMap::default();
        let a = NodeKey::from_digest([1; 32]);
        let b = NodeKey::from_digest([2; 32]);
        let first = map.record(a, "reference", (0, 3));
        let second = map.record(a, "reference", (4, 7));
        let other = map.record(b, "reference", (8, 11));
        assert_eq!(first.ordinal(), 0);
        assert_eq!(second.ordinal(), 1);
        assert_eq!(other.ordinal(), 0);
        assert_eq!(map.resolve(a, &first), Some(&(0, 3)));
        assert_eq!(map.resolve(a, &second), Some(&(4, 7)));
        assert_eq!(map.resolve(b, &other), Some(&(8, 11)));
    }

    /// FR-065-AC-2: identity read after `check` survives a real v2
    /// emit/decode round trip unchanged. Does not exercise a distinct
    /// "after S4 linking" checkpoint (PR #262 review, finding F6):
    /// `link_function_identity` is `fn(x) -> x` for this migration's real
    /// scope (`link_function_identity`'s own doc), so comparing its input
    /// to its output is comparing a value to itself, not something a
    /// broken implementation could fail. QSL-154 owns the real before/
    /// after linking assertion, for a family whose linking is a real
    /// transformation (FR-065-AC-1/AC-3, occurrence-span survival across
    /// S4 linking) -- against this family's pass-through linking, that
    /// assertion would not have been testing anything.
    #[trace("TC-163", "FR-065-AC-2")]
    #[test]
    fn identity_survives_v2_round_trip() {
        let declaration = declaration("f", Expression::Boolean(true));
        let after_check = mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &declaration);
        let name = QualifiedName::unqualified("f").unwrap();
        let bytes = emit_v2(&[(name.clone(), after_check)]);
        let decoded = decode_v2(&bytes).unwrap();
        assert_eq!(decoded, vec![(name, after_check)]);
    }

    /// F16 (rust-review, pre-handoff pass): `#[serde(deny_unknown_fields)]`
    /// on `FunctionPackageV2`/`FunctionEntryV2` (decoded from externally
    /// supplied bytes through the `pub` `decode_function_package_v2`)
    /// actually refuses an unknown field, rather than silently dropping it.
    #[test]
    fn decode_v2_refuses_an_unknown_top_level_field() {
        let bytes =
            br#"{"version":"quire.checked-function-package/v2","functions":[],"extra":true}"#;
        assert_eq!(decode_v2(bytes), Err(DecodeV2Error::Malformed));
    }

    /// F16: same, for an unknown field on one entry rather than the
    /// top-level package.
    #[test]
    fn decode_v2_refuses_an_unknown_entry_field() {
        let bytes = br#"{"version":"quire.checked-function-package/v2","functions":[{"name":"f","identity":"00000000000000000000000000000000000000000000000000000000000000","extra":true}]}"#;
        assert_eq!(decode_v2(bytes), Err(DecodeV2Error::Malformed));
    }

    /// Existing (pre-#262-review) coverage this pass confirmed is real: an
    /// unrecognised version and a non-hex identity are each refused with
    /// their own distinct variant, not `Malformed`.
    #[test]
    fn decode_v2_distinguishes_version_and_identity_refusals() {
        let wrong_version = br#"{"version":"quire.checked-function-package/v1","functions":[]}"#;
        assert_eq!(decode_v2(wrong_version), Err(DecodeV2Error::Version));

        let bad_identity = br#"{"version":"quire.checked-function-package/v2","functions":[{"name":"f","identity":"not-hex"}]}"#;
        assert_eq!(decode_v2(bad_identity), Err(DecodeV2Error::InvalidIdentity));
    }

    /// F6 (rust-review, PR #262 review): `FunctionEntryV2.name` is typed on
    /// `QualifiedName` (`#[serde(try_from = "String")]`), so a wire `name`
    /// that is not a valid qualified name fails to deserialize at all --
    /// `serde_json::from_slice` itself returns `Err`, which `decode_v2`
    /// already maps to `Malformed`. A bare `String` field would have
    /// accepted this silently.
    #[test]
    fn decode_v2_refuses_a_non_identifier_name() {
        let identity = NodeKey::from_digest([7; 32]);
        let bytes = format!(
            r#"{{"version":"{FUNCTION_PACKAGE_V2_VERSION}","functions":[{{"name":"not an identifier","identity":"{identity}"}}]}}"#
        );
        assert_eq!(decode_v2(bytes.as_bytes()), Err(DecodeV2Error::Malformed));
    }
}

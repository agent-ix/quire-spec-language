// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checking-stage identity minting for `Value`'s function-declaration family
//! (FR-062, FR-065), split out of `value::expression::family` by ADR-011
//! §7.3 M-5.
//!
//! FR-068 itself names only `check`, `evaluate`, `facts`, `ir`, `refusal`,
//! `termination` and `mod.rs` as `value::expression`'s move surface; it does
//! not mention `family.rs` at all. `check.rs`'s own `Typer::call`, however,
//! already called `super::family::mint_call_identity` and
//! `super::family::DEFAULT_PACKAGE_IDENTITY` directly before this move (an
//! intra-module dependency that predates this ticket) -- moving `check.rs`
//! without this identity-minting content would leave `check`'s real import
//! graph reaching back into `value::expression::family`, exactly the reverse
//! edge FR-068-AC-3 forbids. This module carries the minimum needed to keep
//! that edge from opening: everything `PackageDeclarations::check` (via
//! [`ValueFunctionFamily`]'s [`crate::family::FamilyContract`] half) and
//! `check.rs`'s own `Typer::call` need to mint a checked identity.
//!
//! `value::expression::family` keeps `QualifiedName`, S4 linking and the v2
//! emit/decode codec: `CheckedPackage::call`'s public signature and the v2
//! wire format are evaluation/emission-side concerns FR-068 leaves at layer
//! 5, and this module does not duplicate them. `ValueFunctionFamily`'s
//! evaluation half ([`crate::family::ReferenceEvaluation`]) stays there too,
//! importing this module's [`ValueFunctionFamily`] back through
//! `crate::check` -- layer 5 depending on layer 3 is the permitted
//! direction (ADR-011 §6.1).

use sha2::{Digest, Sha256};

use quire_exact::{CollectionKind, Location, NodeKey, Origin, Role};

use crate::absence::AbsenceMode;
use crate::forms::{
    Accumulation, BinaryOperator, BinderQuery, Expression, FieldInitializer, FunctionDeclaration,
};
// `QuantityUnit` and `TextProfile` are a genuine gap in FR-068-AC-6/TC-175's
// exhaustive two-tier `value::` allow-list for files under `check`: tier (a)
// is nine named modules (`collection`, `comparison`, `composite`, `decimal`,
// `equality`, `ieee`, `node`, `numeric`, `rational`) and tier (b) is exactly
// five named items (`EnumDeclaration`, `EnumValue` from `enumeration`;
// `check_comparable`, `result_unit`, `UnitOperation` from `quantity`).
// `QuantityUnit` is a sixth item from `quantity`, over tier (b)'s five;
// `TextProfile` comes from `text`, a module neither list names (and TC-175's
// own group (c) -- "everything else" -- must be empty). Both are real,
// pre-existing dependencies of `encode_value_type`'s exhaustive match over
// `ValueType::Quantity`/`ValueType::Text` (this function, and the need for
// both types, predate this ticket -- they were already imported by the
// original, unsplit `value::expression::family.rs`); AC-6/TC-175 only
// starts constraining them because this ticket moves that code under
// `check`. Reported to the ticket owner rather than silently worked around;
// see this module's own doc for the larger context (`family.rs` was never
// named in FR-068's move surface either).
use crate::value::{IeeeWidth, QuantityUnit, RoundingMode, TextProfile, ValueType};

/// The declaring package's `name@version` a checked node's identity
/// preimage includes (ADR-013 O-04). Complete-V1's `PackageDeclarations` has
/// no package name/version of its own (unlike the outer domain-package
/// layer); every caller of `PackageDeclarations::check` (including its
/// ~40 existing test call sites) keeps using the unchanged `check` entry
/// point and gets this default.
pub(crate) const DEFAULT_PACKAGE_IDENTITY: &str = "value.function-package@0.0.0-unversioned";

/// The contract-level `quire_exact::Meter`'s limits, for every call site in
/// this module and [`super`] that builds one just to satisfy
/// [`crate::family::CheckContext::new`]'s signature without itself wanting
/// to bound anything (`ValueFunctionFamily::check`/`evaluate` are not
/// metered against this limit today -- see `crate::family::contract`'s own
/// doc on `_meter`). One `u64::MAX`-in-every-field literal, not six (PR #262
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
///
/// An exhaustive `match` only catches a *new* variant, though (PR #262
/// review, round 2): it forces nothing about the order two existing writes
/// happen in, or the spelling of an existing tag, and every test in this
/// file until round 2 only ever compared two identities minted in the same
/// process or round-tripped through `emit_v2`/`decode_v2`, so a reordered
/// write or a renamed tag would have recompiled clean and passed every one
/// of them while silently changing every minted identity. See
/// `mint_declaration_identity_matches_a_checked_in_digest`
/// (`value::expression::family`'s `tests` module) for the golden-digest test
/// that closes that gap.
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

// PR #262 review, round 2: this function and `encode_expression` write
// several `quire_exact::Integer` leaves through `.to_string()`
// (`Int`/`Rational`/`Decimal` bounds below; `Expression::Integer`/
// `Rational` literals in `encode_expression`) -- `Display`, the same
// mechanism `{:?}` (`Debug`) was rejected for elsewhere in this file. The
// two are not equivalent here: `Integer`'s `Display` is not incidental
// formatting `std` warns is unstable, it is `quire_exact::integer::
// Integer`'s own documented "canonical wire spelling used by complete-V1
// schemas" (`quire-exact/src/integer.rs`'s `FromStr` doc), paired with a
// `FromStr` that refuses any non-canonical spelling (leading zeros, `+`,
// etc.) -- a real, enforced, round-tripping contract, not a Debug-style
// dump of whatever fields happen to exist. `src/value/node.rs`'s own
// `CanonicalRational` already relies on exactly this contract for this
// codebase's other content-addressed digest (RFC 8785 JCS preimages).
// Repointing these leaves at a bespoke byte encoding would introduce a
// second, parallel integer serialization where one canonical, tested one
// already exists and is already trusted for identity purposes -- so they
// are left on `Integer::to_string()` deliberately, not as an oversight.
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

// `Expression::Integer`/`Rational`'s `.to_string()` below: same
// `Integer::Display` canonical-wire-spelling contract, same reasoning --
// see `encode_value_type`'s own doc comment above.
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
/// family -- `check`'s own [`crate::check::Location`] is its existing span
/// analogue: a declaration plus a child-index path). A test exercising this
/// generically with a `(u32, u32)` byte-offset stand-in is still exercising
/// the real mechanism: ordinal assignment and lookup by (identity, role,
/// ordinal) do not depend on what a span actually is.
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

/// `Value`'s checked-package producer for the function-declaration form
/// (FR-062, FR-065): the one family slice migrated onto the
/// `crate::family` contract. A marker type -- every method is a bare
/// associated function over `Self::Form`/`Self::Checked`, with no instance
/// state (ADR-012 §2's contract is static, dispatched through closed enums,
/// not through an object). Its evaluation half
/// ([`crate::family::ReferenceEvaluation`]) is implemented in
/// `value::expression::family`, over this type re-exported through
/// `crate::check`.
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

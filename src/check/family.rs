// SPDX-License-Identifier: AGPL-3.0-or-later
//! `Value`'s function-declaration family (FR-062, FR-065): identity minting,
//! split out of `value::expression::family` by ADR-011 §7.3 M-5, **and**
//! (QSL-148) the real typing, coercion and static-definedness checking for
//! both a function declaration and a function application.
//!
//! **QSL-148 widens this module's role.** Before this ticket,
//! `ValueFunctionFamily::check` (below) only minted a declaration's identity
//! and charged the contract's own nesting-depth limit once; the real typing
//! and definedness verdict was made separately, by a `Typer` `check::mod`'s
//! per-declaration loop constructed and drove directly (a parallel path
//! alongside the contract, not through it) -- so calling the contract's own
//! `check` on an ill-typed declaration returned `Ok` regardless. `check` now
//! calls [`check_declaration_body`] itself, inside the one `FamilyContract`
//! entry point, and returns the crate's real, located [`CheckRefusal`]
//! through [`crate::family::StageFailure::Refused`] when the body or measure
//! does not type or is not statically defined; `PackageDeclarations::check`
//! reaches that verdict only by calling the contract, not by a second,
//! parallel call of its own. [`check_application`] is `Value`'s
//! function-application family check code the same way, relocated from
//! `check.rs`'s deleted `Typer::call`; it is reached transitively through
//! `check_declaration_body`'s own `Typer` pass (a call nested anywhere in a
//! checked body recurses back into `infer_form`'s `Call` arm, which is
//! exactly one call into `check_application`), not through a second,
//! separate `CheckContext`-driven entry -- see [`check_application`]'s own
//! doc for why threading `CheckContext` into every recursive step of the
//! general `Typer` engine (not just `Call`) is out of this ticket's scope.
//!
//! **Amended, PR #282 review F4:** FR-068's move surface names `family.rs`
//! explicitly, alongside `check`, `evaluate`, `facts`, `ir`, `refusal`,
//! `termination` and `mod.rs` -- an omission in the requirement's original
//! text, not a deliberate exclusion. This module carries the minimum needed
//! to keep `check`'s real import graph from reaching back into
//! `value::expression::family`, the reverse edge FR-068-AC-3 forbids:
//! everything `PackageDeclarations::check` (via [`ValueFunctionFamily`]'s
//! [`crate::family::FamilyContract`] half) needs to mint a checked identity
//! and check a declaration's body and applications.
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

use crate::forms::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, Expression, FieldInitializer,
    FunctionDeclaration,
};
use qsl_foundation::absence::AbsenceMode;
// QSL-148: the relocated function-application/-declaration checking code
// below needs `check.rs`'s own `Typer`/`Signature`/`bind_parameters` (the
// general recursive typer this family delegates to for a body or a call's
// arguments -- FR-065-CON-1 forbids reimplementing that engine here, not
// calling into it) and `check::refusal`'s located-refusal vocabulary. Both
// are sibling submodules of `crate::check`, reached the same way this
// module's own pre-existing `use crate::check::ValueFunctionFamily` already
// crosses that boundary.
use super::check::{bind_parameters, Signature, Typer};
use super::facts::{CallSite, Definedness};
use super::ir::Node;
use super::refusal::{CheckCause, CheckRefusal, Location as CheckLocation};
use super::{CheckingLimits, DispatchTable, Scope};
use crate::value::comparison::IllTypedCause;
use crate::value::composite::CompositeShape;
// `QuantityUnit` and `TextProfile`: PR #282 review, F3. FR-068-AC-6/TC-175's
// tier (b) originally bounded to five items across two modules
// (`EnumDeclaration`, `EnumValue` from `enumeration`; `check_comparable`,
// `result_unit`, `UnitOperation` from `quantity`) and could not pass as
// written against any conforming implementation: `encode_value_type`'s
// exhaustive match over `ValueType::Quantity`/`ValueType::Text` has always
// needed both (this function, and the need for both types, predate this
// ticket -- they were already imported by the original, unsplit
// `value::expression::family.rs`). FR-068 now amends tier (b) to seven items
// across three modules, admitting `QuantityUnit` (`value::quantity`) and
// `TextProfile` (`value::text`) explicitly -- see FR-068's Behavior section,
// "The layer-3 sibling imports `check.rs` keeps," and this module's own doc.
use crate::value::composite::ValueType;
use crate::value::decimal::RoundingMode;
use crate::value::ieee::IeeeWidth;
use crate::value::quantity::QuantityUnit;
use crate::value::text::TextProfile;

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
///
/// **`bytes`, `write_bytes` and `write_str` are `pub(super)` (PR #300 review
/// finding 10, QSL-158 S-3b):** `check::identity` (a sibling submodule under
/// `check`) shares this exact length-prefixed preimage writer for its own
/// O-04 type-declaration/variant identities, instead of carrying a
/// byte-identical copy. Both live under `check`, so `pub(super)` (visible to
/// `check` and everything under it) is exactly the scope this sharing
/// needs, no wider; `nodes`/`writes`/`input_bytes`/`input_bytes_limit` stay
/// private -- `identity.rs`'s minters have no `StageLimits` budget to
/// report against (they pass `u64::MAX`, see [`Self::new`]'s own doc) and
/// read only the finished bytes, through [`Self::finish`].
///
/// **`nodes`/`writes`/`input_bytes` (QSL-153).** Running counters alongside
/// the byte buffer, read back only by [`mint_declaration_identity`] as
/// [`IdentityPreimageMetrics`] -- never written into the buffer itself, so
/// they cannot change a minted digest. `nodes` counts each
/// [`encode_expression`] call (one per visited `Expression` node); `writes`
/// counts each base write (`write_bytes`/`write_u64`/`write_bool` --
/// `write_str` is `write_bytes`, not counted twice), a finer-grained count
/// than `nodes` because encoding one node writes several fields (a tag plus
/// its operands/labels); `input_bytes` is the buffer's own logical length.
///
/// **`input_bytes`/`input_bytes_limit` short-circuit `bytes`' own growth
/// (PR #302 review finding 4).** An earlier version measured the minted
/// buffer's size only after the whole pass finished
/// (`preimage.bytes.len()`), so an oversize declaration paid for its full,
/// unbounded allocation before `check` ever compared anything against a
/// limit. `input_bytes` is instead accumulated -- via
/// [`quire_exact::length_amount`], never a bare `as` cast -- on every
/// write, before `bytes` itself grows; once it passes `input_bytes_limit`,
/// every write becomes a no-op against `bytes` (still counted, so
/// `input_bytes` stays the true, uncapped total `check` refuses on) rather
/// than extending an already-over-budget buffer further. `bytes` is safe to
/// leave capped at that point because a capped preimage's identity is never
/// used: `check` refuses before hashing it (`ValueFunctionFamily::check`'s
/// own doc).
pub(super) struct Preimage {
    bytes: Vec<u8>,
    nodes: u64,
    writes: u64,
    input_bytes: u64,
    input_bytes_limit: u64,
}

impl Preimage {
    /// `input_bytes_limit` bounds this preimage's own buffer growth (PR
    /// #302 review finding 4): pass `u64::MAX` for the pre-QSL-153,
    /// unbounded behavior every caller but `ValueFunctionFamily::check`
    /// keeps -- that includes every `check::identity` minter (PR #300
    /// review finding 10), which has no `StageLimits` budget of its own to
    /// report against.
    pub(super) fn new(input_bytes_limit: u64) -> Self {
        Self {
            bytes: Vec::new(),
            nodes: 0,
            writes: 0,
            input_bytes: 0,
            input_bytes_limit,
        }
    }

    /// Whether `bytes`' own growth is still within `input_bytes_limit` --
    /// `false` once a write has already pushed `input_bytes` past it, so
    /// every later write short-circuits rather than growing an
    /// already-over-budget buffer.
    fn within_bytes_limit(&self) -> bool {
        self.input_bytes <= self.input_bytes_limit
    }

    pub(super) fn write_bytes(&mut self, bytes: &[u8]) {
        self.writes += 1;
        // A `u64` length prefix, plus the bytes themselves.
        self.input_bytes = self
            .input_bytes
            .saturating_add(quire_exact::length_amount(std::mem::size_of::<u64>()))
            .saturating_add(quire_exact::length_amount(bytes.len()));
        if !self.within_bytes_limit() {
            return;
        }
        self.bytes
            .extend_from_slice(&quire_exact::length_amount(bytes.len()).to_le_bytes());
        self.bytes.extend_from_slice(bytes);
    }

    pub(super) fn write_str(&mut self, text: &str) {
        self.write_bytes(text.as_bytes());
    }

    /// The finished buffer, for a minter (`check::identity`'s own,
    /// PR #300 review finding 10) that has no `IdentityPreimageMetrics` of
    /// its own to report and only wants the bytes to hash.
    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }

    fn write_u64(&mut self, value: u64) {
        self.writes += 1;
        self.input_bytes = self
            .input_bytes
            .saturating_add(quire_exact::length_amount(std::mem::size_of::<u64>()));
        if !self.within_bytes_limit() {
            return;
        }
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    fn write_bool(&mut self, value: bool) {
        self.writes += 1;
        self.input_bytes = self.input_bytes.saturating_add(1);
        if !self.within_bytes_limit() {
            return;
        }
        self.bytes.push(u8::from(value));
    }

    /// One [`encode_expression`] visit of an `Expression` node.
    fn enter_node(&mut self) {
        self.nodes += 1;
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
// PR #300 review finding 3 (QSL-158 S-3b): `pub(super)` so `check::identity`
// can encode a package type declaration's own declared shape (composite
// field types, a bounded domain's element type, and so on) into its O-04
// preimage through this exact, already-tested encoding, rather than
// inventing a second one.
pub(super) fn encode_value_type(out: &mut Preimage, value_type: &ValueType) {
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
    out.enter_node();
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
///
/// `input_bytes_limit` bounds `Preimage`'s own buffer growth (PR #302
/// review finding 4): pass `u64::MAX` for the pre-QSL-153, unbounded
/// behavior every caller but `ValueFunctionFamily::check` keeps.
pub(crate) fn mint_declaration_identity(
    package_identity: &str,
    declaration: &FunctionDeclaration,
    input_bytes_limit: u64,
) -> (NodeKey, IdentityPreimageMetrics) {
    let mut preimage = Preimage::new(input_bytes_limit);
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
    let metrics = IdentityPreimageMetrics {
        input_bytes: preimage.input_bytes,
        node_count: preimage.nodes,
        work_budget: preimage.writes,
    };
    let identity = NodeKey::from_digest(sha256(&preimage.bytes));
    (identity, metrics)
}

/// [`StageLimits`](crate::family::StageLimits)'s restored real producer
/// values (QSL-153), read back from one real [`mint_declaration_identity`]
/// pass rather than a second, parallel traversal. `input_bytes` and
/// `node_count` are `StageLimits` fields, compared by
/// `crate::family::CheckContext::check_input_bytes`/`check_node_count`;
/// `work_budget` is charged against the shared kernel meter instead (PR
/// #302 review finding 3) -- see `StageLimits`'s own doc.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct IdentityPreimageMetrics {
    /// The minted preimage's own logical byte length (`Preimage::
    /// input_bytes`, not `bytes.len()` -- see `Preimage`'s own doc).
    pub(crate) input_bytes: u64,
    /// The number of `Expression` nodes [`encode_expression`] visited.
    pub(crate) node_count: u64,
    /// The number of base preimage writes performed.
    pub(crate) work_budget: u64,
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
    // Not `StageLimits`-bound (that mechanism is `check`'s own,
    // per-declaration bound, not this call-occurrence identity mint), so
    // `Preimage`'s own short-circuit never engages here.
    let mut preimage = Preimage::new(u64::MAX);
    preimage.write_str("value.function-application");
    preimage.write_str(package_identity);
    preimage.write_str(callee_name);
    preimage.write_u64(arguments.len() as u64);
    for argument in arguments {
        encode_expression(&mut preimage, argument);
    }
    NodeKey::from_digest(sha256(&preimage.bytes))
}

/// QSL-148: `Value`'s family check code for function application
/// (FR-065-AC-4's "one call into `Value`'s family check code"), relocated
/// here from `check.rs`'s deleted `Typer::call`. The algorithm is unchanged
/// -- name resolution against `typer.signatures()`, an arity check, a
/// `typer.check_as` pass over every argument, then a tuple-constructor
/// fallback -- only its *location* moved, from a `Typer`-owned method into
/// this family module, so `infer_form`'s `Expression::Call` arm now
/// dispatches to family-owned code instead of deciding admission itself.
///
/// Takes `typer: &mut Typer<'_>` rather than a `CheckContext`: an ordinary
/// call's own admission decision (does this name resolve, does the arity
/// match, do the arguments type) needs exactly what `Typer` already carries
/// -- the package's `Scope` and every declared `Signature`, for a call
/// possibly nested arbitrarily deep inside the declaration currently being
/// typed. A call is reached from `infer_form`, which recurses through every
/// other `Value` expression form (`Let`, `If`, `Binary`, ... ), so this is
/// itself already reached, transitively, through the one
/// [`FamilyContract::check`](crate::family::FamilyContract::check) call that starts a declaration's real
/// typing (QSL-148: `check` now calls [`check_declaration_body`], which
/// drives the same `Typer`) -- an application inside a declaration's own
/// body is checked "through the contract" in that sense already.
///
/// **An application inside a clause expression is not (PR #303 review,
/// finding N4; type name corrected, round 3 finding F5).**
/// `CheckedGraph::check_clause_expression` (`check/mod.rs`) -- the entry
/// every precondition, postcondition and operation body clause checks
/// through -- builds its own `Typer` directly, with its own declaration-local
/// `nodes` counter, and never constructs a `CheckContext` at all; it is a
/// standalone public entry point on `CheckedGraph`, not something
/// `ValueFunctionFamily::check` (or anything else reached from it) calls. An
/// `Expression::Call` inside such a clause still reaches this same
/// `check_application`, through that `Typer`'s own `infer_form` -- the
/// algorithm is identical either way -- but that call is not, today, reached
/// "through the contract" the way a declaration body's own application is;
/// only the `CheckedGraph` caller that invoked `check_clause_expression`
/// knows it happened at all.
///
/// What this function does *not* do, on either path, is charge the
/// contract's own `CheckContext`/`StageLimits.nesting_depth` once
/// per real recursive step the way [`FamilyContract::check`](crate::family::FamilyContract::check)'s
/// top-level entry charge does: that would mean threading `&mut
/// CheckContext` through every recursive arm of `Typer::infer_form`, not
/// just `Call` (`Let`'s body, `If`'s three arms, `Binary`'s operands, and so
/// on all recurse too) -- reworking the general engine's own recursion
/// signature for every form it checks, which is the reimplementation-scale
/// change FR-065-CON-1 rules out here, not a small addition to this one
/// function. That is real, reported `Typer` entanglement (QSL-148's own open
/// question), not a gap this function papers over: `Typer`'s pre-existing,
/// separate [`super::CheckingLimits`] depth bound (unchanged, checked at
/// every real recursive step already) is what actually keeps a call's own
/// nesting off the host stack today.
pub(crate) fn check_application(
    typer: &mut Typer<'_>,
    name: &str,
    arguments: &[Expression],
    location: &CheckLocation,
) -> Result<Node, CheckRefusal> {
    let signatures = typer.signatures();
    if let Some(function) = signatures
        .iter()
        .position(|signature| signature.name == name && signature.callable_by_name)
    {
        let signature = &signatures[function];
        if signature.parameters.len() != arguments.len() {
            return Err(CheckRefusal::ill_typed(
                location,
                IllTypedCause::TypeMismatch,
            ));
        }
        let mut typed = Vec::with_capacity(arguments.len());
        for (index, (argument, (_, parameter))) in
            arguments.iter().zip(&signature.parameters).enumerate()
        {
            typed.push(typer.check_as(argument, parameter, &location.child(index))?);
        }
        let result = signature.result.clone();
        // FR-062/FR-065: identity is minted from the call's *parsed*
        // structure (`name`, `arguments` before typing), never from
        // `function` (a position-dependent index into `typer.signatures()`
        // that shifts when unrelated declarations reorder) -- see
        // `mint_call_identity`'s doc.
        let identity = mint_call_identity(DEFAULT_PACKAGE_IDENTITY, name, arguments);
        return Ok(Node {
            kind: super::ir::NodeKind::Call {
                identity,
                function,
                arguments: typed,
            },
            value_type: result,
            location: location.clone(),
        });
    }
    let declared = match typer.type_named(name, location) {
        Ok(value_type) => Some(value_type),
        Err(CheckRefusal {
            cause: CheckCause::MissingName(_),
            ..
        }) => None,
        Err(refusal) => return Err(refusal),
    };
    match declared {
        Some(ValueType::Composite(key)) => {
            let Some(CompositeShape::Tuple(positions)) = typer
                .scope()
                .types
                .composite(key)
                .map(|declaration| declaration.shape())
            else {
                return Err(CheckRefusal::ill_typed(
                    location,
                    IllTypedCause::OperatorIneligible,
                ));
            };
            if positions.len() != arguments.len() {
                return Err(CheckRefusal::ill_typed(
                    location,
                    IllTypedCause::TypeMismatch,
                ));
            }
            let mut typed = Vec::with_capacity(arguments.len());
            for (index, (argument, position)) in arguments.iter().zip(positions).enumerate() {
                typed.push(typer.check_as(argument, position, &location.child(index))?);
            }
            Ok(Node {
                kind: super::ir::NodeKind::Tuple {
                    declaration: key,
                    arguments: typed,
                },
                value_type: ValueType::Composite(key),
                location: location.clone(),
            })
        }
        Some(_) => Err(CheckRefusal::ill_typed(
            location,
            IllTypedCause::OperatorIneligible,
        )),
        None if typer
            .scope()
            .model_operations
            .iter()
            .any(|operation| operation == name) =>
        {
            Err(CheckRefusal::ill_typed(
                location,
                IllTypedCause::OperatorIneligible,
            ))
        }
        None => Err(CheckRefusal {
            location: location.clone(),
            cause: CheckCause::MissingName(name.to_owned()),
        }),
    }
}

/// One function declaration's real typing and static-definedness verdict
/// (QSL-148), returned by [`check_application`]'s sibling entry point for
/// declarations. Termination is not included -- see this function's own
/// doc below for why it cannot be.
#[derive(Debug)]
pub(crate) struct CheckedDeclarationBody {
    /// The typed body.
    pub(crate) body: Node,
    /// The typed `decreases` measure, when the declaration wrote one.
    pub(crate) measure: Option<Node>,
    /// The evaluation slot count the body's own parameter/local bindings
    /// allocated.
    pub(crate) slots: usize,
    /// Every call reachable from the body (not the measure -- unchanged
    /// from the pre-migration behavior, see the call site's own doc), for
    /// `check::mod`'s whole-package termination pass.
    pub(crate) calls: Vec<CallSite>,
    /// PR #303 review round 3, finding F1: [`ValueDeclarations::nodes_used`]
    /// (the package's running node total *before* this declaration), plus
    /// every `Expression` node this declaration's own body and measure
    /// admitted -- the same package-wide running total, carried forward.
    /// `check::mod`'s loop reads this back as the next declaration's own
    /// `nodes_used`, restoring the cumulative, package-wide `nodes` bound
    /// [`CheckingLimits::new`] documents ("at most `nodes` expression nodes
    /// per checked package"), through this ordinary `Ok` payload rather than
    /// a side channel.
    pub(crate) nodes_used: u64,
}

/// QSL-148: `Value`'s family check code for a function declaration's
/// typing, coercion and static definedness (FR-065's "the typing,
/// definedness... checking decision"), relocated here from the closure
/// `PackageDeclarations::check` (`check::mod`) used to build inline. The
/// algorithm is unchanged: `bind_parameters`, `check_declared_type` on the
/// declared result, a `check_as` pass over the body, the same treatment for
/// a `decreases` measure (always [`ClauseKind::Body`], regardless of the
/// declaration's own clause kind -- FR-151's dispatch-call restriction
/// gates on the *body's* context, per this function's own inline note
/// below), and a [`Definedness`] pass over the typed body. Only the
/// *entry point* moved: `check::mod`'s `PackageDeclarations::check` no
/// longer constructs a `Typer` or calls any of these functions directly for
/// a declaration -- it calls this one function instead, and this is now the
/// only place that does.
///
/// **Termination stays a whole-package pass, not a per-declaration one.**
/// `check::termination::check` decides a *recursive component of the call
/// graph across every declaration in the package* -- whether `f` calling
/// `g` calling `f` terminates cannot be judged from `f`'s own declaration in
/// isolation, especially since [`crate::family::FamilyContract::check`]
/// (and this function) are called once per declaration and FR-065-AC-2
/// requires each declaration's own identity to be independent of every
/// other declaration's position, so nothing here may assume any other
/// declaration has already been checked. Termination is therefore *not*
/// moved into the family: it stays exactly where `check::mod` already runs
/// it, over every declaration's [`CheckedDeclarationBody::calls`] collected
/// after every declaration's own family check succeeds. This is a reported
/// finding, not a silent narrowing: QSL-148's own ticket text asks to move
/// "typing, definedness and termination checking" into this family, and
/// termination's own whole-package shape makes that specific part
/// impossible under the per-node contract ADR-012 §2 defines (each `check`
/// call sees one form and one `Declarations`, never every sibling
/// declaration at once).
///
/// Takes `input: &ValueDeclarations<'_>` rather than its constituent fields
/// spelled out as separate parameters (PR #303 review, finding 12): this is
/// [`ValueFunctionFamily`]'s own `FamilyContract::Declarations`, the same
/// bundle `check` reads via `cx.declarations()`, reused here rather than
/// unpacked into a `#[allow(clippy::too_many_arguments)]` signature.
///
/// **`Typer`'s own node-budget counter is package-wide again (PR #303 review
/// round 3, finding F1).** An earlier round of this fix gave `Typer` a fresh
/// `let mut nodes = 0_u64` here, on the reasoning that QSL-153's
/// `CheckContext::check_node_count` already bounded the same underlying
/// concern one step earlier in [`FamilyContract::check`](crate::family::FamilyContract::check). That reasoning
/// was wrong: `check_node_count` compares one declaration's own preimage node
/// count against `limits.node_count` -- it is a real, but deliberately
/// *per-declaration-only* bound (`StageLimits::node_count`'s own doc), not a
/// substitute for a *cumulative* one. Resetting `Typer`'s counter to zero for
/// every declaration silently dropped `CheckingLimits::new`'s documented,
/// package-wide contract ("at most `nodes` expression nodes per checked
/// package") down to a per-declaration one -- a package of many small,
/// individually-tiny declarations could exceed the caller's configured
/// `nodes` budget by an unbounded factor.
///
/// The fix restores the pre-QSL-148 shape -- one node counter shared across
/// every declaration in the package -- without a `Cell`, a second call into
/// the caller, or any mutation through this function's read-only
/// `&ValueDeclarations<'_>` parameter: `Typer` is seeded from
/// [`ValueDeclarations::nodes_used`] (the running total every earlier
/// declaration has already admitted, owned and advanced by `check::mod`'s
/// own loop) instead of zero, and this declaration's own final count is
/// carried back out through [`CheckedDeclarationBody::nodes_used`], an
/// ordinary field on this function's ordinary `Ok` payload. `Typer`'s cap
/// itself never changes -- it is still exactly `input.checking_limits.nodes()`,
/// the caller's own original configured limit -- so a refusal it raises
/// already names that limit, never a partial or remaining figure.
pub(crate) fn check_declaration_body(
    input: &ValueDeclarations<'_>,
    form: &FunctionDeclaration,
) -> Result<CheckedDeclarationBody, CheckRefusal> {
    let mut nodes = input.nodes_used;
    let mut typer = Typer::new(
        input.scope,
        input.signatures,
        input.checking_limits,
        &mut nodes,
        form.clause_kind,
    );
    bind_parameters(&mut typer, &form.parameters, input.location)?;
    typer.check_declared_type(&form.result, input.location)?;
    let body = typer.check_as(&form.body, &form.result, input.location)?;
    let slots = typer.slots();
    let measure = match &form.measure {
        Some(measure) => {
            // A `decreases` measure is always checked as `ClauseKind::Body`
            // (`syntax.rs`'s own doc: "A function body, an operation body,
            // or a `decreases` measure"), never `form.clause_kind`:
            // FR-151's dispatch-call restriction gates on the *body's*
            // context, and a measure is its own, always-`Body` context
            // regardless of what the declaration's own body is checked as.
            let mut measure_typer = Typer::new(
                input.scope,
                input.signatures,
                input.checking_limits,
                &mut nodes,
                ClauseKind::Body,
            );
            bind_parameters(&mut measure_typer, &form.parameters, input.measure_location)?;
            Some(measure_typer.infer(measure, None, input.measure_location)?)
        }
        None => None,
    };
    let mut definedness = Definedness::new(
        form.parameters.len(),
        input.dispatch_tables,
        &input.scope.dispatch_operations,
    );
    definedness.check(&body)?;
    // The measure's own definedness is checked, but (unchanged from the
    // pre-migration behavior) its calls are not folded into `calls` below:
    // termination's call graph is built from each function's *body*, not
    // its measure.
    if let Some(measure) = &measure {
        Definedness::new(
            form.parameters.len(),
            input.dispatch_tables,
            &input.scope.dispatch_operations,
        )
        .check(measure)?;
    }
    Ok(CheckedDeclarationBody {
        body,
        measure,
        slots,
        calls: definedness.calls,
        nodes_used: nodes,
    })
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

/// [`ValueFunctionFamily`]'s [`crate::family::FamilyContract::Declarations`]:
/// the package-wide, read-only state one declaration's `check` call needs to
/// run [`check_declaration_body`] for real (QSL-148) -- `Scope`, every
/// declared `Signature` and the checked package's dispatch tables, none of
/// which the shared `CheckContext`/`StageLimits` carry, since those are
/// generic across every family -- plus the two per-declaration locations
/// (`location`, `measure_location`) `check::mod`'s per-declaration loop
/// already computes fresh each iteration, plus [`Self::nodes_used`] (PR #303
/// review round 3, finding F1).
///
/// **No interior mutability (PR #303 review, finding N3).** The real checked
/// body is never smuggled out through a `Cell`-threaded side slot;
/// [`FamilyContract::Checked`](crate::family::FamilyContract::Checked) itself carries it (see
/// [`CheckedDeclaration`]), returned the ordinary way, through `check`'s own
/// `Ok`.
///
/// **The package-wide `nodes` budget travels the same ordinary way (PR #303
/// review round 3, finding F1).** [`Self::nodes_used`] is the running total
/// of `Expression` nodes every earlier declaration in this same package has
/// already admitted -- owned and advanced by `check::mod`'s own loop, not by
/// this struct, exactly the way that loop's pre-QSL-148 version shared one
/// `&mut u64` across every `Typer` it built in turn.
/// [`check_declaration_body`] seeds `Typer`'s own counter from it instead of
/// starting at zero each time, so `Typer` still compares against the one,
/// unmodified `CheckingLimits::nodes` bound this declaration's own
/// `checking_limits` names, but against the *package's* running total, not
/// this declaration's own -- restoring `CheckingLimits::new`'s documented
/// contract ("at most `nodes` expression nodes per checked package") without
/// a `Cell`, a second call into the caller, or a mutation through this
/// struct's own read-only `&D` reference. `CheckContext::check_node_count`
/// (QSL-153) is a separate, deliberately *per-declaration-only* bound over
/// the preimage's own node count (`StageLimits::node_count`'s own doc); it
/// does not substitute for this one and does not accumulate across
/// declarations.
pub(crate) struct ValueDeclarations<'a> {
    pub(crate) package_identity: &'a str,
    pub(crate) scope: &'a Scope,
    pub(crate) signatures: &'a [Signature],
    pub(crate) dispatch_tables: &'a [DispatchTable],
    pub(crate) checking_limits: CheckingLimits,
    pub(crate) location: &'a CheckLocation,
    pub(crate) measure_location: &'a CheckLocation,
    /// See this struct's own doc.
    pub(crate) nodes_used: u64,
}

/// [`ValueFunctionFamily`]'s [`crate::family::FamilyContract::Checked`]
/// (QSL-148; PR #303 review, finding N3): the minted identity together with
/// the real checked body [`check_declaration_body`] produces, returned
/// through `check`'s own `Ok` rather than a side channel.
///
/// This is distinct from [`crate::family::ReferenceEvaluation::Key`], the
/// type `evaluate` is looked up and called by at runtime: `evaluate`'s one
/// real caller (`CheckedPackage::call`, `value::expression::mod.rs`) only
/// ever has a bare identity, resolved out of `CheckedPackage`'s own,
/// separately stored `CheckedFunction` list -- it never has a
/// [`CheckedDeclarationBody`] at that point, only what `check` minted for
/// it. Splitting the two associated types apart is what lets `Checked`
/// carry the richer, check-time-only payload without breaking `evaluate`'s
/// existing calling convention.
#[derive(Debug)]
pub(crate) struct CheckedDeclaration {
    pub(crate) identity: NodeKey,
    pub(crate) body: CheckedDeclarationBody,
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
    /// See [`CheckedDeclaration`]'s own doc (PR #303 review, finding N3):
    /// the minted identity together with the real checked body, returned
    /// through `check`'s ordinary `Ok`, not a side channel.
    type Checked = CheckedDeclaration;
    /// The crate's own located refusal vocabulary (QSL-148): `check` can now
    /// genuinely refuse (an ill-typed or undefined body), and `CheckRefusal`
    /// already carries a closed cause and `catalog_code()` mapping
    /// (`check::CheckCause::code`/`cause`) -- ADR-012 §5.1 S4's "family
    /// `Cause` enum" for this family, not a shape invented to fill this
    /// associated type.
    type Cause = CheckRefusal;
    /// See [`ValueDeclarations`]'s own doc.
    type Declarations<'a> = ValueDeclarations<'a>;

    fn check<'a>(
        form: &Self::Form,
        cx: &mut crate::family::CheckContext<'a, ValueDeclarations<'a>>,
    ) -> crate::family::CheckOutcome<CheckedDeclaration, CheckRefusal> {
        // FR-062 "Explicit limits bound every stage entry, including
        // recursion": `check` charges one nesting-entry before minting,
        // and refuses with a `Limit` outcome rather than reading `form` at
        // all once the configured depth is reached. This is the contract's
        // own per-declaration entry charge, unrelated to (and not a
        // replacement for) `Typer`'s own separate, pre-existing
        // `CheckingLimits.depth` bound that `check_declaration_body`'s real
        // recursive descent below is charged against -- see
        // `check_application`'s own doc for why the two are not unified in
        // this change.
        cx.enter_nesting()
            .map_err(crate::family::StageFailure::Limit)?;
        // FR-062-AC-3 "no side door": the scope stack is pushed and popped
        // around this one check (`cx.scopes.enter`/`leave` below), not just
        // read.
        cx.scopes
            .enter(format!("value.function-declaration:{}", form.name));
        let declarations = cx.declarations();
        let (identity, metrics) =
            mint_declaration_identity(declarations.package_identity, form, cx.limits().input_bytes);
        // PR #262 review (F7): an earlier version of this function
        // recomputed `mint_declaration_identity` a second time here and
        // returned `StageFailure::Fault` on a mismatch, framed as a
        // "defensive" internal-invariant check. It was not: comparing a
        // pure function's output against itself, called twice with the
        // same arguments, cannot fail -- the two calls are definitionally
        // equal, not equal because anything was verified. Deleted along
        // with `StageFailure::Fault`/`InternalFault` themselves (see
        // `crate::family::outcome::StageFailure`'s own doc).
        //
        // QSL-153: the same preimage pass's own byte length and node count
        // are checked against `cx`'s restored `StageLimits` fields before
        // this declaration is admitted -- the first one exceeded refuses
        // with a `Limit` outcome naming it, matching `enter_nesting`'s own
        // `NestingDepth` case above. `check_node_count` is a real, but
        // deliberately per-declaration-only bound over this same
        // declaration's own preimage node count (`StageLimits::node_count`'s
        // own doc); it is not the package-wide `nodes` budget
        // `check_declaration_body`'s own `Typer` counter enforces below,
        // through `ValueDeclarations::nodes_used`/
        // `CheckedDeclarationBody::nodes_used` (PR #303 review round 3,
        // finding F1 -- see `check_declaration_body`'s own doc). PR #303
        // review, finding N1: these checks (and the meter charge below) run
        // *before* `check_declaration_body` -- the real typing/definedness
        // pass -- ever starts, since they are what guards the work that pass
        // is about to do, not a check on its output.
        if let Err(exceeded) = cx
            .check_input_bytes(metrics.input_bytes)
            .and_then(|()| cx.check_node_count(metrics.node_count))
        {
            cx.scopes.leave();
            cx.leave_nesting();
            return Err(crate::family::StageFailure::Limit(exceeded));
        }
        // PR #302 review finding 3: `WorkBudget` is a `Limit` outcome
        // produced by a denied charge against `cx.meter` -- the *shared
        // kernel* budget every family's `check` already receives -- not by
        // comparing the preimage's own write count against a `StageLimits`
        // field (that field's meaning was "how many times the encoder
        // wrote," never a caller-configured budget). One
        // `ChargePoint::DeclarationCheck` per checked declaration, sized by
        // the same preimage pass's field-write count, charged cumulatively
        // against `cx.meter`'s own `work_units` bound (`StageLimits`'s own
        // doc: this is the checking stage's total spend, not one
        // declaration's own shape). This bounds `work_budget` alone --
        // `nodes` has its own separate, package-wide accounting, restored in
        // `check_declaration_body` (PR #303 review round 3, finding F1),
        // not this charge.
        if let Err(incomplete) = cx.meter.charge(
            quire_exact::Charge::new(quire_exact::ChargePoint::DeclarationCheck)
                .work(quire_exact::Integer::from(metrics.work_budget)),
        ) {
            cx.scopes.leave();
            cx.leave_nesting();
            return Err(crate::family::StageFailure::Limit(
                crate::family::LimitExceeded::new(
                    crate::family::StageLimitKind::WorkBudget,
                    incomplete.limit,
                ),
            ));
        }
        // QSL-148: the real typing and static-definedness verdict, made
        // here -- inside the contract's own `check` -- rather than by a
        // second, parallel call `check::mod`'s loop used to make on the
        // side after calling this function. `PackageDeclarations::check`
        // now reaches this verdict only through this one `FamilyContract`
        // entry point (`Self::Checked` itself carries it -- PR #303 review,
        // finding N3 -- not a side-channel `Cell` `cx.declarations()`
        // would otherwise have to expose).
        let checked_body = check_declaration_body(declarations, form);
        // PR #303 review, finding 11: this diagnostic is recorded only once
        // `check_declaration_body` has actually succeeded, not beforehand --
        // an earlier version logged "checked function declaration" right
        // after minting identity, before the real typing/definedness pass
        // ran, so a declaration that the very next line refused (or that a
        // reached limit stopped) had already been diagnosed as checked.
        //
        // PR #303 review round 3, finding F2: recorded here, before
        // `cx.scopes.leave()` below, not after -- `DiagnosticSink::record`
        // reads `cx.scopes.current()` at the moment it is called, so
        // recording it after the scope this check ran in has already been
        // popped reports `<root>` instead of
        // `value.function-declaration:<name>`, silently defeating
        // FR-062-AC-3's "no side door" point of pushing a named scope around
        // this check at all.
        if checked_body.is_ok() {
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
        }
        cx.scopes.leave();
        // PR #262 review (coordinator round 3): an earlier version of this
        // function also asserted `cx.scopes.depth() == depth_before` here.
        // In this straight-line body, one `enter` several lines above is
        // followed by exactly one `leave`, with nothing between them that
        // could push or pop again -- the assertion restated what the two
        // calls already guarantee by construction, not something a broken
        // implementation could trip. `ScopeStack::depth`, that assertion's
        // only reader, is deleted with it.
        cx.leave_nesting();
        // Both cleanup calls above run before this `?` (PR #303 review,
        // finding 12): `check_declaration_body`'s call happens between the
        // paired `enter`/`leave` calls, with the `?` moved after both, so
        // every return path -- success or refusal -- balances the scope
        // stack and the nesting depth identically.
        let body = checked_body.map_err(crate::family::StageFailure::Refused)?;
        Ok(crate::family::Staged::new(CheckedDeclaration {
            identity,
            body,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::collection::CollectionType;
    use quire_exact::CardinalityBound;
    // `TextType`: this golden-digest test was moved here verbatim from
    // `value::expression::family` (PR #282 review F4) and its own doc
    // deliberately exercises `ValueType::Text` as part of the fixture's
    // grammar coverage -- a real, pre-existing dependency this move makes
    // visible under `check` for the first time. FR-068-AC-6's layer rule
    // (the layer-rule ruling, 2026-09-22) permits any item of `value::text`,
    // a K-copy module, so this import needs no special admission at all,
    // in shipped code or here.
    use crate::value::text::TextType;

    /// PR #262 review, round 2 (moved here from `value::expression::family`
    /// under QSL-139's review, since this identity-minting content itself
    /// moved to `check`): the other tests exercising this module either
    /// compare two identities minted in the same process, or round-trip
    /// through `value::expression::family`'s `emit_v2`/`decode_v2` -- none
    /// of them can catch a change to the preimage's own byte grammar.
    /// `encode_expression`/`encode_value_type`'s exhaustive `match`es only
    /// force a compile error for a *new* variant; reordering two
    /// `write_str` calls, or renaming a tag (`"add"` to `"plus"`),
    /// recompiles clean and passes every other test in this file while
    /// silently changing every identity this preimage mints. This fixture
    /// exercises `Let`, `If`, `Binary`, `Call`, `Collection` and `Convert`
    /// on the `Expression` side and `Int`, `Text` and `Collection` (with a
    /// nested `Int` element) on the `ValueType` side -- enough surface that
    /// a reordered write or a renamed tag anywhere in either `match` moves
    /// the digest below. A failure here means the wire preimage grammar
    /// changed; regenerate the constant only when that change is the one
    /// actually intended (and say so in the commit, per this repository's
    /// own digest-freshness rule in `CLAUDE.md`), never to make a red test
    /// green.
    #[test]
    fn mint_declaration_identity_matches_a_checked_in_digest() {
        let element_type = ValueType::Int(
            quire_exact::IntegerInterval::new(
                quire_exact::Integer::from(0_i64),
                quire_exact::Integer::from(10_i64),
            )
            .unwrap(),
        );
        let parameters = vec![
            ("n".to_owned(), element_type.clone()),
            (
                "label".to_owned(),
                ValueType::Text(TextType::new(1, 100, TextProfile::UnicodeScalars).unwrap()),
            ),
        ];
        let result = ValueType::Collection(Box::new(CollectionType::new(
            CollectionKind::Sequence,
            element_type,
            CardinalityBound::new(0, 5).unwrap(),
        )));
        let body = Expression::Let {
            name: "x".to_owned(),
            value: Box::new(Expression::Integer(quire_exact::Integer::from(2_i64))),
            body: Box::new(Expression::If {
                condition: Box::new(Expression::Binary {
                    operator: BinaryOperator::Greater,
                    left: Box::new(Expression::Name("x".to_owned())),
                    right: Box::new(Expression::Integer(quire_exact::Integer::from(1_i64))),
                }),
                then: Box::new(Expression::Call {
                    name: "helper".to_owned(),
                    arguments: vec![Expression::Name("x".to_owned())],
                }),
                otherwise: Box::new(Expression::Convert {
                    target: ValueType::Integer,
                    operand: Box::new(Expression::Collection {
                        kind: CollectionKind::Sequence,
                        elements: vec![Expression::Integer(quire_exact::Integer::from(0_i64))],
                    }),
                }),
            }),
        };
        let declaration = FunctionDeclaration::new("golden", parameters, result, None, body);
        let (identity, _) =
            mint_declaration_identity(DEFAULT_PACKAGE_IDENTITY, &declaration, u64::MAX);
        assert_eq!(
            identity.to_string(),
            "cba9d6dccdc360124ad0823ba8fd5448cb579763ef1b85f9dd0c71182497acfe",
            "the preimage byte grammar changed -- see this test's own doc \
             before regenerating this constant"
        );
    }
}

#[cfg(test)]
pub(crate) mod checking_tests {
    //! QSL-148: behavioral coverage of what `Value`'s relocated family check
    //! code (`check_application`, `check_declaration_body`) accepts and
    //! refuses, per the testing-policy ruling at
    //! <https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837>
    //! (Peter, 2026-09-22, relayed by the QSL team lead) -- these tests
    //! exercise real accept/refuse outcomes over real fixtures, not the
    //! code's shape or where it lives.
    use super::*;
    use crate::check::refusal::Origin as CheckOrigin;
    use ix_trace_rs::trace;

    /// PR #303 review, finding N7b: `pub(crate)`, not private -- this is
    /// the one real definition `check::mod`'s own `#[cfg(test)]`-gated
    /// re-export hands to `value::expression::family`'s
    /// `family_contract_tests` module, which used to keep a second,
    /// byte-for-byte copy of this same fixture instead of importing it.
    pub(crate) fn root_location() -> CheckLocation {
        CheckLocation {
            origin: CheckOrigin::Expression,
            path: Vec::new(),
        }
    }

    fn boolean_signature(name: &str, parameter_count: usize) -> Signature {
        Signature {
            name: name.to_owned(),
            parameters: (0..parameter_count)
                .map(|index| (format!("p{index}"), ValueType::Boolean))
                .collect(),
            result: ValueType::Boolean,
            callable_by_name: true,
        }
    }

    /// See [`root_location`]'s own doc (PR #303 review, finding N7b): the
    /// one real definition, re-exported rather than duplicated.
    pub(crate) fn empty_scope() -> Scope {
        Scope {
            types: crate::value::composite::TypeEnvironment::default(),
            enums: Vec::new(),
            aliases: Vec::new(),
            model_operations: Vec::new(),
            ieee_profile: None,
            dispatch_operations: Vec::new(),
        }
    }

    /// TC-376/FR-065-AC-4: a well-typed call to a one-argument Boolean
    /// function is admitted through `check_application` -- `Value`'s
    /// relocated family check code, not `Typer::call` (deleted) -- and
    /// produces a `NodeKind::Call` node of the declared result type.
    #[trace("TC-376")]
    #[test]
    fn check_application_accepts_a_well_typed_call() {
        let scope = empty_scope();
        let signatures = vec![boolean_signature("f", 1)];
        let mut nodes = 0_u64;
        let mut typer = Typer::new(
            &scope,
            &signatures,
            CheckingLimits::default(),
            &mut nodes,
            ClauseKind::Body,
        );
        let location = root_location();
        let arguments = vec![Expression::Boolean(true)];
        let checked = check_application(&mut typer, "f", &arguments, &location)
            .expect("one Boolean argument against a one-Boolean-parameter signature admits");
        assert_eq!(checked.value_type, ValueType::Boolean);
        assert!(matches!(
            checked.kind,
            super::super::ir::NodeKind::Call { function: 0, .. }
        ));
    }

    /// TC-376/FR-065-AC-4: a call with the wrong number of arguments is
    /// refused (`ill_typed`/`type-mismatch`) by `check_application` itself,
    /// not admitted and caught somewhere else.
    #[trace("TC-376")]
    #[test]
    fn check_application_refuses_wrong_arity() {
        let scope = empty_scope();
        let signatures = vec![boolean_signature("f", 1)];
        let mut nodes = 0_u64;
        let mut typer = Typer::new(
            &scope,
            &signatures,
            CheckingLimits::default(),
            &mut nodes,
            ClauseKind::Body,
        );
        let location = root_location();
        let arguments = vec![Expression::Boolean(true), Expression::Boolean(false)];
        let refusal = check_application(&mut typer, "f", &arguments, &location)
            .expect_err("two arguments against a one-parameter signature must refuse");
        assert!(matches!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::TypeMismatch)
        ));
    }

    /// TC-376/FR-065-AC-4: a call naming no declared function and no tuple
    /// constructor is refused `missing-name`, exactly as `Typer::call` (now
    /// deleted) used to refuse it.
    #[trace("TC-376")]
    #[test]
    fn check_application_refuses_an_unknown_name() {
        let scope = empty_scope();
        let signatures: Vec<Signature> = Vec::new();
        let mut nodes = 0_u64;
        let mut typer = Typer::new(
            &scope,
            &signatures,
            CheckingLimits::default(),
            &mut nodes,
            ClauseKind::Body,
        );
        let location = root_location();
        let refusal = check_application(&mut typer, "nowhere", &[], &location)
            .expect_err("an undeclared name with no tuple-constructor type must refuse");
        assert!(matches!(
            refusal.cause,
            CheckCause::MissingName(name) if name == "nowhere"
        ));
    }

    /// TC-376 step 4: a call whose argument type disagrees with the
    /// declared parameter type -- arity matches, the callee resolves, but
    /// `typer.check_as` refuses the mismatched argument -- is refused
    /// `ill_typed`/`type-mismatch` by `check_application` itself, the same
    /// path `check_application_refuses_wrong_arity` exercises for the
    /// arity case above.
    #[trace("TC-376")]
    #[test]
    fn check_application_refuses_a_type_mismatched_argument() {
        let scope = empty_scope();
        let signatures = vec![boolean_signature("f", 1)];
        let mut nodes = 0_u64;
        let mut typer = Typer::new(
            &scope,
            &signatures,
            CheckingLimits::default(),
            &mut nodes,
            ClauseKind::Body,
        );
        let location = root_location();
        let arguments = vec![Expression::Integer(quire_exact::Integer::from(1_i64))];
        let refusal = check_application(&mut typer, "f", &arguments, &location)
            .expect_err("an Integer argument against a declared Boolean parameter must refuse");
        assert!(matches!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::TypeMismatch)
        ));
    }

    /// A [`ValueDeclarations`] for tests exercising [`check_declaration_body`]
    /// or [`ValueFunctionFamily::check`] directly (PR #303 review round 3,
    /// finding F6: the one real definition, shared the same way
    /// [`empty_scope`]/[`root_location`] are -- this used to be defined a
    /// second time, with a different parameter shape, in
    /// `value::expression::family`'s own `family_contract_tests` module).
    /// `nodes_used` always starts at `0`: every test using this helper
    /// exercises one declaration in isolation, not `check::mod`'s own
    /// running package total.
    pub(crate) fn declarations_for<'a>(
        package_identity: &'a str,
        scope: &'a Scope,
        signatures: &'a [Signature],
        dispatch_tables: &'a [DispatchTable],
        checking_limits: CheckingLimits,
        location: &'a CheckLocation,
    ) -> ValueDeclarations<'a> {
        ValueDeclarations {
            package_identity,
            scope,
            signatures,
            dispatch_tables,
            checking_limits,
            location,
            measure_location: location,
            nodes_used: 0,
        }
    }

    /// TC-377/FR-065's checking-decision half: `check_declaration_body`
    /// admits a well-typed declaration whose body calls another declared
    /// function, and reports that call in `calls` -- the same `CallSite`
    /// list `check::mod`'s whole-package termination pass reads.
    #[trace("TC-377")]
    #[test]
    fn check_declaration_body_accepts_a_well_typed_declaration_and_reports_its_calls() {
        let scope = empty_scope();
        let callee = FunctionDeclaration::new(
            "f",
            vec![("x".to_owned(), ValueType::Boolean)],
            ValueType::Boolean,
            None,
            Expression::Name("x".to_owned()),
        );
        let caller = FunctionDeclaration::new(
            "g",
            Vec::new(),
            ValueType::Boolean,
            None,
            Expression::Call {
                name: "f".to_owned(),
                arguments: vec![Expression::Boolean(true)],
            },
        );
        let signatures = vec![
            Signature {
                name: "f".to_owned(),
                parameters: callee.parameters.clone(),
                result: callee.result.clone(),
                callable_by_name: true,
            },
            Signature {
                name: "g".to_owned(),
                parameters: caller.parameters.clone(),
                result: caller.result.clone(),
                callable_by_name: true,
            },
        ];
        let dispatch_tables: Vec<DispatchTable> = Vec::new();
        let location = root_location();
        let input = declarations_for(
            DEFAULT_PACKAGE_IDENTITY,
            &scope,
            &signatures,
            &dispatch_tables,
            CheckingLimits::default(),
            &location,
        );
        let checked = check_declaration_body(&input, &caller)
            .expect("g's body -- a well-typed call to f -- admits");
        assert_eq!(checked.slots, 0);
        assert_eq!(checked.calls.len(), 1);
        assert_eq!(
            checked.calls[0].callee, 0,
            "the call resolves to f, index 0"
        );
    }

    /// TC-377: `check_declaration_body` refuses a declaration whose body
    /// does not have the declared result type.
    #[trace("TC-377")]
    #[test]
    fn check_declaration_body_refuses_an_ill_typed_body() {
        let scope = empty_scope();
        let signatures: Vec<Signature> = Vec::new();
        let dispatch_tables: Vec<DispatchTable> = Vec::new();
        let location = root_location();
        let form = FunctionDeclaration::new(
            "g",
            Vec::new(),
            ValueType::Boolean,
            None,
            Expression::Integer(quire_exact::Integer::from(1_i64)),
        );
        let input = declarations_for(
            DEFAULT_PACKAGE_IDENTITY,
            &scope,
            &signatures,
            &dispatch_tables,
            CheckingLimits::default(),
            &location,
        );
        let refusal = check_declaration_body(&input, &form)
            .expect_err("an Integer body against a declared Boolean result must refuse");
        assert!(matches!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::TypeMismatch)
        ));
    }

    /// TC-377: `check_declaration_body` refuses a well-*typed* body that is
    /// statically undefined -- `value(o)` on an `Option[Integer]` parameter
    /// with no proved `present(o)` guarding it -- via the same
    /// `Definedness` pass `check_declaration_body` runs after typing, not
    /// only a type mismatch. Distinct from
    /// `check_declaration_body_refuses_an_ill_typed_body` above: this body
    /// types cleanly (`value(o)` on an `Option[Integer]` is `Integer`,
    /// matching the declared result) and is refused only because the
    /// obligation `present(o)` proves is unproved.
    #[trace("TC-377")]
    #[test]
    fn check_declaration_body_refuses_an_undefined_body() {
        let scope = empty_scope();
        let signatures: Vec<Signature> = Vec::new();
        let dispatch_tables: Vec<DispatchTable> = Vec::new();
        let location = root_location();
        let form = FunctionDeclaration::new(
            "v",
            vec![("o".to_owned(), ValueType::option(ValueType::Integer))],
            ValueType::Integer,
            None,
            Expression::Value(Box::new(Expression::Name("o".to_owned()))),
        );
        let input = declarations_for(
            DEFAULT_PACKAGE_IDENTITY,
            &scope,
            &signatures,
            &dispatch_tables,
            CheckingLimits::default(),
            &location,
        );
        let refusal = check_declaration_body(&input, &form).expect_err(
            "value(o) with no proved present(o) is statically undefined, not ill-typed",
        );
        assert!(matches!(
            refusal.cause,
            CheckCause::Unproved(crate::check::refusal::Obligation::Presence)
        ));
    }
}

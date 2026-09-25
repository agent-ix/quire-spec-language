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
//! through [`StageFailure::Refused`] when the body or measure
//! does not type or is not statically defined; `PackageDeclarations::check`
//! reaches that verdict only by calling the contract, not by a second,
//! parallel call of its own. [`Application`] is `Value`'s
//! function-application family check code the same way, relocated from
//! `check.rs`'s deleted `Typer::call`; it is reached transitively through
//! `check_declaration_body`'s own `Typer` pass (a call nested anywhere in a
//! checked body reaches the typer's `Call` arm, which resolves it through
//! [`Application::resolve`] and builds it through [`Application::finish`]),
//! not through a second, separate `CheckContext`-driven entry -- see
//! [`Application`]'s own doc for why threading `CheckContext` into every
//! nesting step of the general `Typer` engine (not just `Call`) is out of
//! this ticket's scope.
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
//! evaluation half (layer 5's `ReferenceEvaluation`, `value::expression::s6a`)
//! stays there too,
//! importing this module's [`ValueFunctionFamily`] back through
//! `crate::check` -- layer 5 depending on layer 3 is the permitted
//! direction (ADR-011 §6.1).

use std::collections::BTreeMap;

use quire_exact::{CollectionKind, NodeKey, Origin, Role};

use qsl_forms::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, Expression, FieldInitializer,
    FunctionDeclaration,
};
use qsl_foundation::absence::AbsenceMode;
use qsl_foundation::diagnostic::{LimitExceeded, LimitKind, Locus, StageFailure, Staged};
// QSL-148: the relocated function-application/-declaration checking code
// below needs `check.rs`'s own `Typer`/`Signature`/`bind_parameters` (the
// general typer this family delegates to for a body or a call's
// arguments -- FR-065-CON-1 forbids reimplementing that engine here, not
// calling into it) and `check::refusal`'s located-refusal vocabulary. Both
// are sibling submodules of `crate::check`, reached the same way this
// module's own pre-existing `use crate::check::ValueFunctionFamily` already
// crosses that boundary.
use super::check::{bind_parameters, Signature, Signatures, Typer};
use super::facts::{CallSite, Definedness};
use super::ir::Node;
use super::refusal::{CheckCause, CheckRefusal, CheckingLimitKind, Location as CheckLocation};
use super::{CheckingLimits, DeclarationRegions, DispatchTable, Origin as CheckOrigin, Scope};
use crate::value::declaration::CompositeShape;
use quire_exact::IeeeWidth;
use quire_exact::IllTypedCause;
use quire_exact::RoundingMode;
use quire_exact::TextProfile;
use quire_exact::ValueType;
use quire_exact::{UnitDomain, UnitId};

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

/// QSL-153's size meter over one parsed declaration: the `StageLimits`
/// figures [`ValueFunctionFamily::check`] compares before it types the
/// declaration (`input_bytes`, `node_count`) and the work it charges
/// (`work_budget`). It hashes nothing and mints no identity: a checked
/// node's identity is its FR-092/FR-093 key (`check::lowering`, QSL-156
/// A4b), minted after typing.
///
/// Each figure is the length-prefixed encoding this walk describes, so the
/// limits keep the meaning QSL-153 gave them: `input_bytes` is the encoding's
/// logical byte length (a `u64` length prefix plus the bytes of each written
/// string, eight bytes per number, one per flag), accumulated via
/// [`quire_exact::length_amount`], never a bare `as` cast; `nodes` counts
/// each [`encode_expression`] visit; `writes` counts each base write.
/// Every tag is an explicit `&'static str` chosen at its `match` arm and
/// every `match` below is exhaustive, so a new `Expression` or `ValueType`
/// variant does not compile until it is measured.
pub(super) struct DeclarationMeter {
    nodes: u64,
    writes: u64,
    input_bytes: u64,
}

impl DeclarationMeter {
    fn new() -> Self {
        Self {
            nodes: 0,
            writes: 0,
            input_bytes: 0,
        }
    }

    fn write_bytes(&mut self, bytes: &[u8]) {
        self.writes += 1;
        // A `u64` length prefix, plus the bytes themselves.
        self.input_bytes = self
            .input_bytes
            .saturating_add(quire_exact::length_amount(std::mem::size_of::<u64>()))
            .saturating_add(quire_exact::length_amount(bytes.len()));
    }

    fn write_str(&mut self, text: &str) {
        self.write_bytes(text.as_bytes());
    }

    fn write_u64(&mut self, _value: u64) {
        self.writes += 1;
        self.input_bytes = self
            .input_bytes
            .saturating_add(quire_exact::length_amount(std::mem::size_of::<u64>()));
    }

    fn write_bool(&mut self, _value: bool) {
        self.writes += 1;
        self.input_bytes = self.input_bytes.saturating_add(1);
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

fn encode_quantity_unit(out: &mut DeclarationMeter, unit: UnitId) {
    // The unit's own content-addressed identity: a declared unit's node key
    // or a compound unit's `quire.value.compound-unit/v1` digest (ADR-013
    // OQ-B), tagged by its domain -- never a re-derivation from the unit's
    // internal dimension/edge graph, and never `Debug`.
    out.write_str(match unit.domain() {
        UnitDomain::Declared => "declared",
        UnitDomain::Compound => "compound",
    });
    out.write_str(&unit.to_string());
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
// dump of whatever fields happen to exist. `src/value/semantic_node.rs`'s own
// `CanonicalRational` already relies on exactly this contract for this
// codebase's other content-addressed digest (RFC 8785 JCS preimages).
// Repointing these leaves at a bespoke byte encoding would introduce a
// second, parallel integer serialization where one canonical, tested one
// already exists and is already trusted for identity purposes -- so they
// are left on `Integer::to_string()` deliberately, not as an oversight.
fn encode_value_type(out: &mut DeclarationMeter, value_type: &ValueType) {
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
            encode_quantity_unit(out, *unit);
        }
        ValueType::Text(text_type) => {
            out.write_str("text");
            out.write_u64(text_type.min());
            out.write_u64(text_type.max());
            out.write_str(text_profile_tag(text_type.profile()));
        }
        ValueType::Enum(shape) => {
            out.write_str("enum");
            out.write_bool(shape.is_ordered());
            for variant in shape.variants() {
                out.write_str(&variant.to_string());
            }
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
            // Bound presence is part of the type (ADR-014 N-3).
            match collection_type.bound() {
                Some(bound) => {
                    out.write_str("bounded");
                    out.write_u64(bound.minimum());
                    out.write_u64(bound.maximum());
                }
                None => out.write_str("unbounded"),
            }
        }
        ValueType::Reference(key) => {
            out.write_str("reference");
            out.write_str(&key.to_string());
        }
        ValueType::Population(maximum) => {
            out.write_str("population");
            match maximum {
                Some(maximum) => {
                    out.write_str("bounded");
                    out.write_u64(*maximum);
                }
                None => out.write_str("unbounded"),
            }
        }
    }
}

// `Expression::Integer`/`Rational`'s `.to_string()` below: same
// `Integer::Display` canonical-wire-spelling contract, same reasoning --
// see `encode_value_type`'s own doc comment above.
//
// QSL-228: the walk keeps its pending sub-expressions on a heap stack, not
// the host stack, so an expression nested past every checking limit is
// measured (and then refused by the typer's depth limit) rather than
// overflowing here first. Sub-expressions are visited in source pre-order,
// so a `Convert`/`AllInstances`/`Lookup` target that does not resolve is
// refused at the same node the recursive walk refused it at. A node's own
// writes all happen at its visit, including the few (a field name, an
// absence mode, a dispatch member, the identity flag) that follow an operand
// in source order: every figure is a sum over the writes, so where a write
// falls in the walk does not change it.
fn encode_expression(
    out: &mut DeclarationMeter,
    root: &Expression,
    targets: &TargetTypes<'_>,
) -> Result<(), CheckRefusal> {
    let mut pending = vec![root];
    while let Some(expr) = pending.pop() {
        out.enter_node();
        // The node's sub-expressions in source order; pushed reversed below
        // so the first is visited next.
        let first = pending.len();
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
                pending.extend([&**value, &**body]);
            }
            Expression::If {
                condition,
                then,
                otherwise,
            } => {
                out.write_str("if");
                pending.extend([&**condition, &**then, &**otherwise]);
            }
            Expression::Binary {
                operator,
                left,
                right,
            } => {
                out.write_str("binary");
                out.write_str(binary_operator_tag(*operator));
                pending.extend([&**left, &**right]);
            }
            Expression::Negate(operand) => {
                out.write_str("negate");
                pending.push(operand);
            }
            Expression::Not(operand) => {
                out.write_str("not");
                pending.push(operand);
            }
            Expression::Field { operand, field } => {
                out.write_str("field");
                out.write_str(field);
                pending.push(operand);
            }
            Expression::Present(operand) => {
                out.write_str("present");
                pending.push(operand);
            }
            Expression::Value(operand) => {
                out.write_str("value");
                pending.push(operand);
            }
            Expression::Deref(operand) => {
                out.write_str("deref");
                pending.push(operand);
            }
            Expression::Call { name, arguments } => {
                out.write_str("call");
                out.write_str(name);
                out.write_u64(arguments.len() as u64);
                pending.extend(arguments);
            }
            Expression::Record { name, fields } => {
                out.write_str("record");
                out.write_str(name);
                out.write_u64(fields.len() as u64);
                for (field_name, initializer) in fields {
                    out.write_str(field_name);
                    match initializer {
                        FieldInitializer::Value(expression) => {
                            out.write_str("value");
                            pending.push(expression);
                        }
                        FieldInitializer::Null => out.write_str("null"),
                    }
                }
            }
            Expression::Collection { kind, elements } => {
                out.write_str("collection");
                out.write_str(collection_kind_tag(*kind));
                out.write_u64(elements.len() as u64);
                pending.extend(elements);
            }
            Expression::Convert { target, operand } => {
                out.write_str("convert");
                encode_value_type(out, &targets.resolve(target)?);
                pending.push(operand);
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
                pending.extend([&**source, &**body]);
            }
            Expression::Flatten(operand) => {
                out.write_str("flatten");
                pending.push(operand);
            }
            Expression::Accumulate {
                accumulator_type_span: _,
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
                out.write_bool(identity.is_some());
                pending.extend([&**source, &**step]);
                pending.extend(identity.as_deref());
            }
            Expression::Count {
                result_type_span: _,
                result_type,
                binder,
                source,
                predicate,
            } => {
                out.write_str("count");
                out.write_str(result_type);
                out.write_str(binder);
                pending.extend([&**source, &**predicate]);
            }
            Expression::Sum {
                result_type_span: _,
                result_type,
                binder,
                source,
                summand,
            } => {
                out.write_str("sum");
                out.write_str(result_type);
                out.write_str(binder);
                pending.extend([&**source, &**summand]);
            }
            Expression::Size(operand) => {
                out.write_str("size");
                pending.push(operand);
            }
            Expression::Contains { collection, item } => {
                out.write_str("contains");
                pending.extend([&**collection, &**item]);
            }
            Expression::AllInstances { target, population } => {
                out.write_str("all-instances");
                encode_value_type(out, &targets.resolve(target)?);
                pending.push(population);
            }
            Expression::Lookup {
                target,
                population,
                reference,
                absence,
            } => {
                out.write_str("lookup");
                encode_value_type(out, &targets.resolve(target)?);
                out.write_str(absence_mode_tag(*absence));
                pending.extend([&**population, &**reference]);
            }
            Expression::Dispatch {
                receiver,
                member,
                arguments,
            } => {
                out.write_str("dispatch");
                out.write_str(member);
                out.write_u64(arguments.len() as u64);
                pending.push(receiver);
                pending.extend(arguments);
            }
            Expression::Pre(operand) => {
                out.write_str("pre");
                pending.push(operand);
            }
            // Not the S2 seam (`Typer::infer_form`'s own doc,
            // `qsl-semantics/src/check/check/typing.rs`): an unconditional
            // probe arm so this match keeps compiling under `--cfg
            // seam_probe`.
            #[cfg(seam_probe)]
            Expression::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
        pending[first..].reverse();
    }
    Ok(())
}

/// Measure one function declaration's parsed size (QSL-153): its name,
/// parameters, result, measure and body, as authored, with every declared
/// type written as its resolved `ValueType` (so two spellings of one type
/// measure alike) and each `Convert`/`AllInstances`/`Lookup` target resolved
/// through `targets`. A target that does not resolve is refused here with
/// the same refusal the checker would give it.
///
/// The figures bound the work [`ValueFunctionFamily::check`] is about to do
/// (see [`DeclarationMeter`]); the declaration's identity is its FR-092
/// function node key, minted by `check::lowering` once every declaration is
/// typed.
pub(crate) fn measure_declaration(
    declaration: &FunctionDeclaration,
    signature: &Signature,
    targets: &TargetTypes<'_>,
) -> Result<DeclarationMetrics, CheckRefusal> {
    let mut meter = DeclarationMeter::new();
    meter.write_str("value.function-declaration");
    meter.write_str(&declaration.name);
    meter.write_u64(signature.parameters.len() as u64);
    for (name, value_type) in &signature.parameters {
        meter.write_str(name);
        encode_value_type(&mut meter, value_type);
    }
    encode_value_type(&mut meter, &signature.result);
    meter.write_bool(declaration.measure.is_some());
    if let Some(measure) = &declaration.measure {
        encode_expression(&mut meter, measure, targets)?;
    }
    encode_expression(&mut meter, &declaration.body, targets)?;
    Ok(DeclarationMetrics {
        input_bytes: meter.input_bytes,
        node_count: meter.nodes,
        work_budget: meter.writes,
    })
}

/// [`StageLimits`](crate::family::StageLimits)'s real producer values
/// (QSL-153), read back from one `measure_declaration` pass.
/// `input_bytes` and `node_count` are `StageLimits` fields, compared by
/// `crate::family::CheckContext::check_input_bytes`/`check_node_count`;
/// `work_budget` is charged against the shared kernel meter instead (PR
/// #302 review finding 3) -- see `StageLimits`'s own doc.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeclarationMetrics {
    /// The measured encoding's logical byte length.
    pub(crate) input_bytes: u64,
    /// The number of `Expression` nodes `encode_expression` visited.
    pub(crate) node_count: u64,
    /// The number of base writes performed.
    pub(crate) work_budget: u64,
}

/// Resolves the `Convert`/`AllInstances`/`Lookup` target type forms
/// [`measure_declaration`] measures, against the package's [`Scope`].
pub(crate) struct TargetTypes<'a> {
    scope: &'a Scope,
    location: &'a CheckLocation,
}

impl<'a> TargetTypes<'a> {
    /// Resolve targets against `scope`, refusing at `location`.
    pub(crate) fn new(scope: &'a Scope, location: &'a CheckLocation) -> Self {
        Self { scope, location }
    }

    fn resolve(&self, target: &qsl_forms::TypeForm) -> Result<ValueType, CheckRefusal> {
        super::type_form::resolve_type_form(self.scope, target, self.location)
    }
}
/// QSL-148: `Value`'s family check code for function application
/// (FR-065-AC-4; the typer's `Call` arm calls it once, FR-065-CON-3), relocated
/// here from `check.rs`'s deleted `Typer::call`. The algorithm is unchanged
/// -- name resolution against `typer.signatures()`, an arity check, a check
/// of every argument against its parameter type, then a tuple-constructor
/// fallback -- only its *location* moved, from a `Typer`-owned method into
/// this family module, so the typer's `Expression::Call` arm dispatches to
/// family-owned code instead of deciding admission itself.
///
/// QSL-228 splits the one function in two around the arguments' typing:
/// [`Self::resolve`] resolves the callee and checks the arity, the typer's
/// explicit-stack loop checks each argument against [`Self::parameter`], and
/// [`Self::finish`] builds the call. The typer's loop, not this module,
/// holds the argument checks, so a call nested in a call's argument costs
/// heap, not host stack.
///
/// Resolved against the `Typer`'s own declarations rather than a
/// `CheckContext`: an ordinary call's own admission decision (does this
/// name resolve, does the arity match, do the arguments type) needs exactly
/// what `Typer` already carries -- the package's `Scope` and every declared
/// `Signature`, for a call possibly nested arbitrarily deep inside the
/// declaration currently being typed. A call is reached from the typer's
/// dispatch over every other `Value` expression form (`Let`, `If`,
/// `Binary`, ... ), so this is itself already reached, transitively,
/// through the one
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
/// application check, through that `Typer`'s own dispatch -- the
/// algorithm is identical either way -- but that call is not, today, reached
/// "through the contract" the way a declaration body's own application is;
/// only the `CheckedGraph` caller that invoked `check_clause_expression`
/// knows it happened at all.
///
/// What this check does *not* do, on either path, is charge the
/// contract's own `CheckContext`/`StageLimits.nesting_depth` once
/// per real nesting step the way [`FamilyContract::check`](crate::family::FamilyContract::check)'s
/// top-level entry charge does: that would mean threading `&mut
/// CheckContext` through every form the typer checks, not just `Call`
/// (`Let`'s body, `If`'s three arms, `Binary`'s operands, and so on all
/// nest too) -- reworking the general engine for every form it checks,
/// which is the reimplementation-scale change FR-065-CON-1 rules out here,
/// not a small addition to this one check. That is real, reported `Typer`
/// entanglement (QSL-148's own open question), not a gap this check papers
/// over: `Typer`'s pre-existing, separate [`super::CheckingLimits`] depth
/// bound (unchanged, checked at every real nesting step already) is what
/// bounds a call's own nesting.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Application<'a> {
    /// A call of the declared function at index `function`.
    Function {
        /// The callee's index among the package's functions.
        function: usize,
        /// The callee's signature.
        signature: &'a Signature,
    },
    /// A tuple constructor of the declared tuple `declaration`.
    Tuple {
        /// The tuple declaration's handle.
        declaration: NodeKey,
        /// Its position types, in order.
        positions: &'a [ValueType],
    },
}

impl<'a> Application<'a> {
    /// Resolve a call of `name` with `arity` arguments: a declared function
    /// callable by name, else a tuple constructor; refuses an arity
    /// mismatch, a name naming neither, and a name that is not callable.
    pub(crate) fn resolve(
        typer: &Typer<'a>,
        name: &str,
        arity: usize,
        location: &CheckLocation,
    ) -> Result<Self, CheckRefusal> {
        if let Some((function, signature)) = typer.signatures().callable(name) {
            if signature.parameters.len() != arity {
                return Err(CheckRefusal::ill_typed(
                    location,
                    IllTypedCause::TypeMismatch,
                ));
            }
            return Ok(Self::Function {
                function,
                signature,
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
                    .types()
                    .composite(key)
                    .map(|declaration| declaration.shape())
                else {
                    return Err(CheckRefusal::ill_typed(
                        location,
                        IllTypedCause::OperatorIneligible,
                    ));
                };
                if positions.len() != arity {
                    return Err(CheckRefusal::ill_typed(
                        location,
                        IllTypedCause::TypeMismatch,
                    ));
                }
                Ok(Self::Tuple {
                    declaration: key,
                    positions,
                })
            }
            Some(_) => Err(CheckRefusal::ill_typed(
                location,
                IllTypedCause::OperatorIneligible,
            )),
            None if typer.scope().declares_model_operation(name) => Err(CheckRefusal::ill_typed(
                location,
                IllTypedCause::OperatorIneligible,
            )),
            None => Err(CheckRefusal {
                location: location.clone(),
                cause: CheckCause::MissingName(name.to_owned()),
            }),
        }
    }

    /// The type the argument at `index` is checked against.
    pub(crate) fn parameter(self, index: usize) -> Option<&'a ValueType> {
        match self {
            Self::Function { signature, .. } => signature
                .parameters
                .get(index)
                .map(|(_, parameter)| parameter),
            Self::Tuple { positions, .. } => positions.get(index),
        }
    }

    /// The application node over its checked `arguments`.
    pub(crate) fn finish(self, arguments: Vec<Node>, location: &CheckLocation) -> Node {
        match self {
            // FR-093: the call's identity is the key of the `expression`
            // node `check::lowering` builds for it once its callee is keyed.
            Self::Function {
                function,
                signature,
            } => Node {
                kind: super::ir::NodeKind::Call {
                    function,
                    arguments,
                },
                value_type: signature.result.clone(),
                location: location.clone(),
            },
            Self::Tuple { declaration, .. } => Node {
                kind: super::ir::NodeKind::Tuple {
                    declaration,
                    arguments,
                },
                value_type: ValueType::Composite(declaration),
                location: location.clone(),
            },
        }
    }
}

/// One function declaration's real typing and static-definedness verdict
/// (QSL-148), returned by [`Application`]'s sibling entry point for
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
    /// The name each body slot was bound under, indexed by slot (FR-092
    /// parameter nodes).
    pub(crate) slot_names: Vec<String>,
    /// The name each measure slot was bound under, indexed by slot.
    pub(crate) measure_slot_names: Vec<String>,
    /// Every compound unit the body's and measure's typing formed (FR-094).
    pub(crate) formed_units: crate::value::quantity::UnitTable,
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
    // The declaration's resolved parameter and result types; `form`'s own
    // are type forms.
    let parameters = &input.own_signature.parameters;
    let result = &input.own_signature.result;
    let mut typer = Typer::new(
        input.scope,
        input.signatures,
        input.checking_limits,
        &mut nodes,
        form.clause_kind(),
    );
    bind_parameters(&mut typer, parameters, input.location)?;
    typer.check_declared_type(result, input.location)?;
    let body = typer.check_as(&form.body, result, input.location)?;
    let slots = typer.slots();
    let slot_names = typer.slot_names().to_vec();
    let mut formed_units = typer.into_formed_units();
    let mut measure_slot_names = Vec::new();
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
            bind_parameters(&mut measure_typer, parameters, input.measure_location)?;
            let measure = measure_typer.infer(measure, None, input.measure_location)?;
            measure_slot_names = measure_typer.slot_names().to_vec();
            formed_units.extend(measure_typer.into_formed_units());
            Some(measure)
        }
        None => None,
    };
    let mut definedness = Definedness::new(
        parameters.len(),
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
            parameters.len(),
            input.dispatch_tables,
            &input.scope.dispatch_operations,
        )
        .check(measure)?;
    }
    Ok(CheckedDeclarationBody {
        body,
        measure,
        slots,
        slot_names,
        measure_slot_names,
        formed_units,
        calls: definedness.calls,
        nodes_used: nodes,
    })
}

/// Every source occurrence of a migrated form recorded so far, keyed by
/// (identity, role, ordinal) (ADR-013 O-07) and mapped to its source span
/// (ADR-013 O-12). QSL is the only minter (FR-062 "Provenance").
///
/// `S` is the span type. Complete-V1's own function forms have no lexed
/// byte offsets to report (there is no text parser for this API-constructed
/// family -- `check`'s own [`crate::check::Location`] is its existing span
/// analogue: a declaration plus a child-index path). A test exercising this
/// generically with a `(u32, u32)` byte-offset stand-in is still exercising
/// the real mechanism: ordinal assignment and lookup by (identity, role,
/// ordinal) do not depend on what a span actually is.
///
/// Keyed by (identity, role) (QSL-205): the key's value holds that role's
/// spans in ordinal order, so the next ordinal is its length and an
/// ordinal's span is an index. Neither `record` nor `resolve` scans another
/// entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct OccurrenceMap<S> {
    spans: BTreeMap<(NodeKey, Role), Vec<S>>,
}

// A hand-written `Default`, not `#[derive(Default)]`: the derive macro adds
// an `S: Default` bound even though `BTreeMap::default()` needs none -- a
// known derive-macro imprecision, not a real requirement on the span type.
impl<S> Default for OccurrenceMap<S> {
    fn default() -> Self {
        Self {
            spans: BTreeMap::new(),
        }
    }
}

impl<S> OccurrenceMap<S> {
    /// Record one occurrence of `identity` under `role`, at `span`. Ordinals
    /// are assigned by (identity, role) insertion order (ADR-013 O-07: "an
    /// ordinal disambiguating repeated occurrences of that role on the same
    /// node") -- reordering *other* nodes' occurrences never changes this
    /// one's ordinal, only its own role's own repeat count does.
    pub(crate) fn record(&mut self, identity: NodeKey, role: &str, span: S) -> Origin {
        let role = Role::new(role);
        // Most (identity, role) pairs occur once; a capacity of one keeps a
        // single occurrence from reserving a vector's default four slots.
        let spans = self
            .spans
            .entry((identity, role.clone()))
            .or_insert_with(|| Vec::with_capacity(1));
        let ordinal = u64::try_from(spans.len()).unwrap_or(u64::MAX);
        spans.push(span);
        Origin::new(role, ordinal)
    }

    /// Whether any occurrence of `identity` is recorded.
    pub(crate) fn has(&self, identity: NodeKey) -> bool {
        // The empty role sorts first, so the first key at or after it is
        // `identity`'s first recorded role, if it has one.
        self.spans
            .range((identity, Role::new(""))..)
            .next()
            .is_some_and(|((node, _), _)| *node == identity)
    }

    /// Every recorded occurrence, ascending by (identity, role), each role's
    /// occurrences in ordinal order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (NodeKey, Origin, &S)> {
        self.spans.iter().flat_map(|((identity, role), spans)| {
            (0_u64..)
                .zip(spans)
                .map(move |(ordinal, span)| (*identity, Origin::new(role.clone(), ordinal), span))
        })
    }

    /// The span recorded for `identity` at exactly `origin`, if any.
    pub(crate) fn resolve(&self, identity: NodeKey, origin: &Origin) -> Option<&S> {
        let ordinal = usize::try_from(origin.ordinal()).ok()?;
        self.spans
            .get(&(identity, origin.role().clone()))?
            .get(ordinal)
    }
}

/// [`ValueFunctionFamily`]'s [`crate::family::FamilyContract::Declarations`]:
/// the package-wide, read-only state one declaration's `check` call needs to
/// run `check_declaration_body` for real (QSL-148) -- `Scope`, every
/// declared `Signature` and the checked package's dispatch tables, none of
/// which the shared `CheckContext`/`StageLimits` carry, since those are
/// generic across every family -- plus the two per-declaration locations
/// (`location`, `measure_location`) `check::mod`'s per-declaration loop
/// already computes fresh each iteration, plus `nodes_used` (PR #303
/// review round 3, finding F1).
///
/// **No interior mutability (PR #303 review, finding N3).** The real checked
/// body is never smuggled out through a `Cell`-threaded side slot;
/// [`FamilyContract::Checked`](crate::family::FamilyContract::Checked) itself carries it (see
/// [`CheckedDeclaration`]), returned the ordinary way, through `check`'s own
/// `Ok`.
///
/// **The package-wide `nodes` budget travels the same ordinary way (PR #303
/// review round 3, finding F1).** `nodes_used` is the running total
/// of `Expression` nodes every earlier declaration in this same package has
/// already admitted -- owned and advanced by `check::mod`'s own loop, not by
/// this struct, exactly the way that loop's pre-QSL-148 version shared one
/// `&mut u64` across every `Typer` it built in turn.
/// `check_declaration_body` seeds `Typer`'s own counter from it instead of
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
pub struct ValueDeclarations<'a> {
    pub(crate) scope: &'a Scope,
    pub(crate) signatures: &'a Signatures,
    /// The resolved signature of the declaration being checked
    /// (`signatures[index]`). `check_declaration_body` types against it, and
    /// `measure_declaration` measures it.
    pub(crate) own_signature: &'a Signature,
    pub(crate) dispatch_tables: &'a [DispatchTable],
    pub(crate) checking_limits: CheckingLimits,
    pub(crate) location: &'a CheckLocation,
    pub(crate) measure_location: &'a CheckLocation,
    /// See this struct's own doc.
    pub(crate) nodes_used: u64,
    /// FR-096: the regions of the unit the declarations were read from,
    /// through which a limit's locus resolves. `None` for declarations not
    /// read from a unit, whose positions no region names.
    pub(crate) regions: Option<&'a DeclarationRegions>,
}

impl ValueDeclarations<'_> {
    /// FR-096: the declaration being checked, as a whole, located at its
    /// form's span; `None` when it was not read from the unit (a function
    /// synthesized for FR-151 dispatch).
    fn declaration_locus(&self) -> Option<Locus> {
        let CheckOrigin::Body { index, .. } = self.location.origin else {
            return None;
        };
        self.regions?.declaration_region(index).map(Locus::Region)
    }

    /// FR-096: the node `location` names, located at its span; `None` for
    /// a position in a tree not read from the unit.
    fn locus(&self, location: &CheckLocation) -> Option<Locus> {
        self.regions?.region(location).map(Locus::Region)
    }
}

/// [`ValueFunctionFamily`]'s [`crate::family::FamilyContract::Checked`]
/// (QSL-148; PR #303 review, finding N3): the real checked body
/// `check_declaration_body` produces, returned through `check`'s own `Ok`
/// rather than a side channel. The declaration's identity is not here: it is
/// the FR-092 function node key, which hashes the keys of the functions its
/// body calls, so `PackageDeclarations::check` mints it once every
/// declaration is checked (`check::lowering`, QSL-156 A4b).
#[derive(Debug)]
pub struct CheckedDeclaration {
    pub(crate) body: CheckedDeclarationBody,
}

/// `Value`'s checked-package producer for the function-declaration form
/// (FR-062, FR-065): the one family slice migrated onto the
/// `crate::family` contract. A marker type -- every method is a bare
/// associated function over `Self::Form`/`Self::Checked`, with no instance
/// state (ADR-012 §2's contract is static, dispatched through closed enums,
/// not through an object). Its evaluation half
/// (layer 5's `ReferenceEvaluation`, `value::expression::s6a`) is implemented in
/// `value::expression::family`, over this type re-exported through
/// `crate::check`.
pub struct ValueFunctionFamily;

impl crate::family::FamilyContract for ValueFunctionFamily {
    type Form = FunctionDeclaration;
    /// See [`CheckedDeclaration`]'s own doc (PR #303 review, finding N3):
    /// the real checked body, returned through `check`'s ordinary `Ok`, not
    /// a side channel.
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

    /// A function declaration requests no FR-057 capability kind (FR-057:
    /// "no kind for an expression nested in a clause, such as a function
    /// application"), so it has no requirements (FR-062-AC-4).
    fn requirements(_checked: &CheckedDeclaration) -> Option<crate::family::Requirements> {
        None
    }

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
        // `Application`'s own doc for why the two are not unified in
        // this change.
        //
        // FR-096: every limit this function reaches for the declaration as
        // a whole is located at the declaration's span; `Typer`'s depth
        // stop, at the node whose entry failed.
        let declarations = cx.declarations();
        let located =
            |limit: LimitExceeded| StageFailure::Limit(limit.at(declarations.declaration_locus()));
        cx.enter_nesting().map_err(located)?;
        // FR-062-AC-3 "no side door": the scope stack is pushed and popped
        // around this one check (`cx.scopes.enter`/`leave` below), not just
        // read.
        cx.scopes
            .enter(format!("value.function-declaration:{}", form.name));
        let measured = measure_declaration(
            form,
            declarations.own_signature,
            &TargetTypes::new(declarations.scope, declarations.location),
        );
        let metrics = match measured {
            Ok(metrics) => metrics,
            Err(refusal) => {
                cx.scopes.leave();
                cx.leave_nesting();
                return Err(StageFailure::Refused(refusal));
            }
        };
        // QSL-153: the measured byte length and node count
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
            return Err(located(exceeded));
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
            // The meter's own report: the charge carries work units only, so
            // the denied counter is `work_units`, and the refused charge
            // would have taken its cumulative spend to what was consumed
            // plus the denied amount -- this declaration's charge, a `u64`.
            debug_assert_eq!(
                incomplete.limit_kind,
                quire_exact::LimitKind::WorkUnits,
                "a declaration-check charge carries work units only"
            );
            let denied = incomplete.next_charge.to_u64();
            debug_assert_eq!(
                denied,
                Some(metrics.work_budget),
                "the denied amount is this declaration's charge"
            );
            return Err(located(LimitExceeded::new(
                LimitKind::WorkBudget,
                incomplete.limit,
                u128::from(incomplete.consumed) + u128::from(denied.unwrap_or(metrics.work_budget)),
            )));
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
                    cx.meter.admission_count(),
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
        // QSL-236 (M1), FR-062-AC-7: the Typer's own nesting-depth limit is
        // the one `CheckCause::ResourceExhausted` cause AC-7 requires a
        // `StageFailure::Limit` for, not a typed `Refused` -- the contract's
        // other checking limits (nodes, work budget, input bytes) reach
        // `Typer::enter` through `CheckContext`, already `Limit`-mapped
        // above (this function's own `cx.enter_nesting()` and
        // `check_declaration_body`'s meter charges); only the Typer's
        // unrelated, pre-existing `CheckingLimits.depth` bound was still
        // surfacing as a typed refusal.
        let body = checked_body.map_err(|refusal| match refusal.cause {
            CheckCause::ResourceExhausted(ref exceeded)
                if exceeded.kind == CheckingLimitKind::Depth =>
            {
                StageFailure::Limit(
                    LimitExceeded::new(LimitKind::NestingDepth, exceeded.limit, exceeded.actual)
                        .at(declarations.locus(&refusal.location)),
                )
            }
            _ => StageFailure::Refused(refusal),
        })?;
        Ok(Staged::new(CheckedDeclaration { body }))
    }
}

#[cfg(test)]
mod tests {
    use super::fixtures::{empty_scope, fixture_source, root_location};
    use super::*;
    use crate::value::declaration::{CompositeDeclaration, FieldDeclaration, TypeEnvironment};
    use ix_trace_rs::trace;
    use qsl_forms::{BuiltinType, TypeForm};
    use quire_exact::{Presence, TextType};

    const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

    fn text(bounds: &[&str]) -> TypeForm {
        TypeForm::builtin(BuiltinType::Text, SPAN)
            .with_bounds(bounds.iter().map(|bound| (*bound).to_owned()).collect())
    }

    /// `declaration`'s checked identity, in a package of `types` and
    /// `aliases` under the fixture owner.
    fn mint(
        types: &TypeEnvironment,
        aliases: &[(String, ValueType)],
        declaration: &FunctionDeclaration,
    ) -> NodeKey {
        let mut package = crate::check::PackageDeclarations::new(fixture_source());
        package.types = types.clone();
        package.aliases = aliases.to_vec();
        package.functions = vec![declaration.clone()];
        package
            .check(CheckingLimits::default())
            .expect("the fixture checks")
            .function_identity(&declaration.name)
            .expect("the fixture declares its function")
    }

    fn unary(parameter: TypeForm) -> FunctionDeclaration {
        FunctionDeclaration::new(
            "f",
            vec![("x".to_owned(), parameter)],
            TypeForm::builtin(BuiltinType::Boolean, SPAN),
            None,
            Expression::Boolean(true),
        )
    }

    /// ADR-013 O-04: equal ids mean structurally identical nodes. Three
    /// spellings of one type -- defaulted profile, explicit profile, and an
    /// alias -- give one parameter type node, so the function over it has
    /// one identity; a different profile gives another.
    #[trace("TC-160", "FR-062-AC-2")]
    #[test]
    fn spellings_of_one_resolved_type_mint_one_identity() {
        let types = TypeEnvironment::default();
        let aliases = [(
            "Label".to_owned(),
            ValueType::Text(TextType::new(1, 100, TextProfile::UnicodeScalars).unwrap()),
        )];
        let mint = |declaration: &FunctionDeclaration| mint(&types, &aliases, declaration);
        let defaulted = mint(&unary(text(&["1", "100"])));
        assert_eq!(
            defaulted,
            mint(&unary(text(&["1", "100", "unicode-scalars"])))
        );
        assert_eq!(defaulted, mint(&unary(TypeForm::name("Label", SPAN))));
        assert_ne!(defaulted, mint(&unary(text(&["1", "100", "nfc"]))));
    }

    /// A function declared over a record carries that record's node key: a
    /// `Point` with different fields gives `f(p: Point)` a different
    /// identity, though `Point` is spelled the same.
    #[trace("TC-160", "FR-062-AC-2")]
    #[test]
    fn a_changed_record_changes_the_identity_of_functions_over_it() {
        let types_with = |fill: u8, field_type: ValueType| {
            TypeEnvironment::new(
                [CompositeDeclaration::new(
                    NodeKey::from_digest([fill; 32]),
                    "Point",
                    CompositeShape::Record(vec![FieldDeclaration::new(
                        "x",
                        field_type,
                        Presence::Required,
                    )]),
                )],
                [],
            )
            .unwrap()
        };
        let over_point = unary(TypeForm::name("Point", SPAN));
        assert_ne!(
            mint(&types_with(1, ValueType::Integer), &[], &over_point),
            mint(&types_with(2, ValueType::Boolean), &[], &over_point)
        );
    }

    /// A `Convert` target that does not resolve is refused while measuring,
    /// with the checker's own missing-name refusal, not measured as spelling.
    #[test]
    fn an_unresolved_target_is_refused_while_measuring() {
        let scope = empty_scope();
        let location = root_location();
        let declaration = FunctionDeclaration::new(
            "g",
            Vec::new(),
            TypeForm::builtin(BuiltinType::Integer, SPAN),
            None,
            Expression::Convert {
                target: TypeForm::name("Nowhere", SPAN),
                operand: Box::new(Expression::Integer(quire_exact::Integer::from(1_i64))),
            },
        );
        let signature = Signature {
            name: "g".to_owned(),
            parameters: Vec::new(),
            result: ValueType::Integer,
            callable_by_name: true,
        };
        let refusal = measure_declaration(
            &declaration,
            &signature,
            &TargetTypes::new(&scope, &location),
        )
        .unwrap_err();
        assert!(matches!(refusal.cause, CheckCause::MissingName(name) if name == "Nowhere"));
    }
}

/// Test fixtures shared by `check`'s own tests and by the layer-5
/// evaluator's tests (`value::expression::family`), which reach them across
/// the QSL-181 crate boundary through `test-support`. Never compiled into a
/// production build.
#[cfg(any(test, feature = "test-support"))]
pub mod fixtures {
    use super::*;
    use crate::check::refusal::Origin as CheckOrigin;
    use crate::family::{CheckContext, DiagnosticSink, ScopeStack, StageLimits};
    use quire_exact::Meter;

    /// The source reference of the empty unit admitted as (`a`, `u`,
    /// `git`, `1`): its owner `(a, u)` is the one FR-092's golden vectors
    /// are keyed under.
    pub fn fixture_source() -> qsl_foundation::source::provenance::RawSourceRef {
        admitted_source(
            qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
            b"",
        )
    }

    /// The `RawSourceRef` S0 mints for `bytes` admitted under `labels`.
    pub fn admitted_source(
        labels: qsl_foundation::SourceIdentity,
        bytes: &[u8],
    ) -> qsl_foundation::source::provenance::RawSourceRef {
        let path = labels.identity.clone();
        qsl_foundation::Source::read(labels, path, bytes, bytes.len())
            .expect("named, admissible fixture bytes")
            .reference()
            .clone()
    }

    /// `check`'s unbounded scalar limits (`pub(crate)`).
    pub const SCALAR_LIMITS_UNLIMITED: quire_exact::ScalarLimits = super::SCALAR_LIMITS_UNLIMITED;

    /// A family check context, through `CheckContext::new` (`pub(crate)`).
    pub fn check_context<'a, D>(
        declarations: &'a D,
        limits: StageLimits,
        meter: &'a mut Meter,
        diagnostics: &'a mut DiagnosticSink,
        scopes: &'a mut ScopeStack,
    ) -> CheckContext<'a, D> {
        CheckContext::new(declarations, limits, meter, diagnostics, scopes)
    }

    /// `declaration`'s [`DeclarationMetrics`] as `ValueFunctionFamily::check`
    /// measures it: over its signature resolved against `scope`.
    pub fn measure_resolved(
        scope: &Scope,
        declaration: &FunctionDeclaration,
    ) -> DeclarationMetrics {
        let location = root_location();
        let (parameters, result) = crate::check::resolve_signature(scope, declaration, &location)
            .expect("the fixture's signature resolves");
        let signature = Signature {
            name: declaration.name.clone(),
            parameters,
            result,
            callable_by_name: true,
        };
        measure_declaration(declaration, &signature, &TargetTypes::new(scope, &location))
            .expect("the fixture's targets resolve")
    }
    /// PR #303 review, finding N7b: shared, not private -- this is
    /// the one real definition `check::mod`'s own test-support
    /// re-export hands to `value::expression::family`'s
    /// `family_contract_tests` module, which used to keep a second,
    /// byte-for-byte copy of this same fixture instead of importing it.
    pub fn root_location() -> CheckLocation {
        CheckLocation {
            origin: CheckOrigin::Expression,
            path: Vec::new(),
        }
    }
    pub fn boolean_signature(name: &str, parameter_count: usize) -> Signature {
        Signature {
            name: name.to_owned(),
            parameters: (0..parameter_count)
                .map(|index| (format!("p{index}"), ValueType::Boolean))
                .collect(),
            result: ValueType::Boolean,
            callable_by_name: true,
        }
    }
    /// A bare `Boolean` type form.
    pub fn boolean_type_form() -> qsl_forms::TypeForm {
        qsl_forms::TypeForm::builtin(
            qsl_forms::BuiltinType::Boolean,
            qsl_foundation::Span { start: 0, end: 0 },
        )
    }
    /// See [`root_location`]'s own doc (PR #303 review, finding N7b): the
    /// one real definition, re-exported rather than duplicated.
    pub fn empty_scope() -> Scope {
        scope_with(
            crate::value::declaration::TypeEnvironment::default(),
            Vec::new(),
        )
    }
    /// A scope declaring only `types` and `aliases`, its by-name lookups
    /// built over them (QSL-205: a `Scope` is built once, never mutated).
    pub fn scope_with(
        types: crate::value::declaration::TypeEnvironment,
        aliases: Vec<(String, quire_exact::ValueType)>,
    ) -> Scope {
        Scope::new(types, Vec::new(), aliases, Vec::new(), None, Vec::new())
    }
    /// A [`ValueDeclarations`] for tests exercising `check_declaration_body`
    /// or `ValueFunctionFamily::check` directly (PR #303 review round 3,
    /// finding F6: the one real definition, shared the same way
    /// [`empty_scope`]/[`root_location`] are -- this used to be defined a
    /// second time, with a different parameter shape, in
    /// `value::expression::family`'s own `family_contract_tests` module).
    /// `nodes_used` always starts at `0`: every test using this helper
    /// exercises one declaration in isolation, not `check::mod`'s own
    /// running package total.
    pub fn declarations_for<'a>(
        scope: &'a Scope,
        signatures: &'a Signatures,
        own_signature: &'a Signature,
        dispatch_tables: &'a [DispatchTable],
        checking_limits: CheckingLimits,
        location: &'a CheckLocation,
    ) -> ValueDeclarations<'a> {
        ValueDeclarations {
            scope,
            signatures,
            own_signature,
            dispatch_tables,
            checking_limits,
            location,
            measure_location: location,
            nodes_used: 0,
            regions: None,
        }
    }
    /// The contract-level stage limits these tests check under: nothing
    /// is bounded except where a test tightens one field.
    pub fn limits() -> StageLimits {
        StageLimits {
            nesting_depth: 128,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        }
    }
    /// A `Boolean`-result, parameterless declaration of `body`.
    pub fn declaration(name: &str, body: Expression) -> FunctionDeclaration {
        FunctionDeclaration::new(name, Vec::new(), boolean_type_form(), None, body)
    }
    /// The resolved signature of a [`declaration`] fixture (no parameters,
    /// `Boolean` result), for `declarations_for`'s `own_signature`.
    pub fn declaration_signature(name: &str) -> Signature {
        boolean_signature(name, 0)
    }
}

#[cfg(test)]
pub(crate) mod checking_tests {
    //! QSL-148: behavioral coverage of what `Value`'s relocated family check
    //! code (`Application`, `check_declaration_body`) accepts and
    //! refuses, per the testing-policy ruling at
    //! <https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837>
    //! (Peter, 2026-09-22, relayed by the QSL team lead) -- these tests
    //! exercise real accept/refuse outcomes over real fixtures, not the
    //! code's shape or where it lives.
    use super::fixtures::{
        boolean_signature, boolean_type_form, declaration, declaration_signature, declarations_for,
        empty_scope, limits, measure_resolved, root_location,
    };
    use super::*;
    use crate::family::{CheckContext, DiagnosticSink, FamilyContract, ScopeStack, StageLimits};
    use ix_trace_rs::trace;
    use quire_exact::Meter;

    /// A call of `name` over `arguments`, typed through the typer's `Call`
    /// arm: [`Application::resolve`], each argument's check, then
    /// [`Application::finish`].
    fn check_application(
        typer: &mut Typer<'_>,
        name: &str,
        arguments: &[Expression],
        location: &CheckLocation,
    ) -> Result<Node, CheckRefusal> {
        let call = Expression::Call {
            name: name.to_owned(),
            arguments: arguments.to_vec(),
        };
        typer.infer(&call, None, location)
    }

    /// An `Option<Integer>` type form.
    fn option_integer_type_form() -> qsl_forms::TypeForm {
        qsl_forms::TypeForm::builtin(
            qsl_forms::BuiltinType::Option,
            qsl_foundation::Span { start: 0, end: 0 },
        )
        .with_arguments(vec![qsl_forms::TypeForm::builtin(
            qsl_forms::BuiltinType::Integer,
            qsl_foundation::Span { start: 0, end: 0 },
        )])
    }

    /// TC-376/FR-065-AC-4: a well-typed call to a one-argument Boolean
    /// function is admitted through [`Application`] -- `Value`'s
    /// relocated family check code, not `Typer::call` (deleted) -- and
    /// produces a `NodeKind::Call` node of the declared result type.
    #[trace("TC-376", "FR-065-AC-4")]
    #[test]
    fn check_application_accepts_a_well_typed_call() {
        let scope = empty_scope();
        let signatures = Signatures::from(vec![boolean_signature("f", 1)]);
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
    /// refused (`ill_typed`/`type-mismatch`) by [`Application::resolve`] itself,
    /// not admitted and caught somewhere else.
    #[trace("TC-376", "FR-065-AC-4")]
    #[test]
    fn check_application_refuses_wrong_arity() {
        let scope = empty_scope();
        let signatures = Signatures::from(vec![boolean_signature("f", 1)]);
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
    #[trace("TC-376", "FR-065-AC-4")]
    #[test]
    fn check_application_refuses_an_unknown_name() {
        let scope = empty_scope();
        let signatures = Signatures::default();
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
    /// `ill_typed`/`type-mismatch` by the application check itself, the same
    /// path `check_application_refuses_wrong_arity` exercises for the
    /// arity case above.
    #[trace("TC-376", "FR-065-AC-4")]
    #[test]
    fn check_application_refuses_a_type_mismatched_argument() {
        let scope = empty_scope();
        let signatures = Signatures::from(vec![boolean_signature("f", 1)]);
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

    /// TC-377/FR-065's checking-decision half: `check_declaration_body`
    /// admits a well-typed declaration whose body calls another declared
    /// function, and reports that call in `calls` -- the same `CallSite`
    /// list `check::mod`'s whole-package termination pass reads.
    #[trace("TC-377")]
    #[test]
    fn check_declaration_body_accepts_a_well_typed_declaration_and_reports_its_calls() {
        let scope = empty_scope();
        // `f` is represented only by its resolved signature below, which is
        // how `check_declaration_body` receives its callees.
        let caller = FunctionDeclaration::new(
            "g",
            Vec::new(),
            boolean_type_form(),
            None,
            Expression::Call {
                name: "f".to_owned(),
                arguments: vec![Expression::Boolean(true)],
            },
        );
        let signatures = Signatures::from(vec![
            Signature {
                name: "f".to_owned(),
                parameters: vec![("x".to_owned(), ValueType::Boolean)],
                result: ValueType::Boolean,
                callable_by_name: true,
            },
            Signature {
                name: "g".to_owned(),
                parameters: Vec::new(),
                result: ValueType::Boolean,
                callable_by_name: true,
            },
        ]);
        let dispatch_tables: Vec<DispatchTable> = Vec::new();
        let location = root_location();
        let input = declarations_for(
            &scope,
            &signatures,
            &signatures.as_slice()[1],
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
        let signatures = Signatures::default();
        let own_signature = Signature {
            name: "g".to_owned(),
            parameters: Vec::new(),
            result: ValueType::Boolean,
            callable_by_name: true,
        };
        let dispatch_tables: Vec<DispatchTable> = Vec::new();
        let location = root_location();
        let form = FunctionDeclaration::new(
            "g",
            Vec::new(),
            boolean_type_form(),
            None,
            Expression::Integer(quire_exact::Integer::from(1_i64)),
        );
        let input = declarations_for(
            &scope,
            &signatures,
            &own_signature,
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
        let signatures = Signatures::default();
        let own_signature = Signature {
            name: "v".to_owned(),
            parameters: vec![("o".to_owned(), ValueType::option(ValueType::Integer))],
            result: ValueType::Integer,
            callable_by_name: true,
        };
        let dispatch_tables: Vec<DispatchTable> = Vec::new();
        let location = root_location();
        let form = FunctionDeclaration::new(
            "v",
            vec![("o".to_owned(), option_integer_type_form())],
            qsl_forms::TypeForm::builtin(
                qsl_forms::BuiltinType::Integer,
                qsl_foundation::Span { start: 0, end: 0 },
            ),
            None,
            Expression::Value(Box::new(Expression::Name("o".to_owned()))),
        );
        let input = declarations_for(
            &scope,
            &signatures,
            &own_signature,
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

    // QSL-181 X-6a: the tests below exercise only `ValueFunctionFamily`'s
    // `check` hook and the identity minting under it, so they live here, in
    // layer-3 `check`, rather than beside the layer-5 evaluator in
    // `value::expression::family`, whose tests keep only what needs the
    // evaluator or the v2 codec. The three fixtures below are shared with
    // those tests through `check::mod`'s `#[cfg(test)]` re-export (PR #303
    // N7b), not copied.

    /// QSL-148's core requirement (PR #303 review, finding 1): calling
    /// `ValueFunctionFamily::check` on an ill-typed declaration -- `g() ->
    /// Boolean = 1`, an `Integer` body against a declared `Boolean` result
    /// -- returns a refusal *through the contract itself*, not `Ok` after
    /// minting an identity that says nothing about whether the body
    /// actually types. Before this ticket, `check` never inspected
    /// `form.body`'s type at all: the real typing decision was made by a
    /// `Typer` `check::mod`'s per-declaration loop constructed and drove
    /// separately, after already calling `check` and discarding nothing --
    /// so this exact fixture, checked through the contract alone, returned
    /// `Ok`. It does not any more.
    #[trace("TC-380", "FR-065-AC-7")]
    #[test]
    fn value_function_family_check_refuses_an_ill_typed_body() {
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("g");
        let signatures = Signatures::default();
        let declarations = declarations_for(
            &scope,
            &signatures,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = FunctionDeclaration::new(
            "g",
            Vec::new(),
            boolean_type_form(),
            None,
            Expression::Integer(quire_exact::Integer::from(1_i64)),
        );
        let refused = ValueFunctionFamily::check(&form, &mut cx).expect_err(
            "an Integer body against a declared Boolean result must refuse through the contract",
        );
        match refused {
            StageFailure::Refused(refusal) => assert!(
                matches!(
                    refusal.cause,
                    CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
                ),
                "expected an ill-typed/type-mismatch refusal, got {refusal:?}"
            ),
            other => panic!("expected StageFailure::Refused, got {other:?}"),
        }
        // PR #303 review, finding 11: no diagnostic is recorded for a
        // refused declaration -- the "checked function declaration"
        // message asserts a real success, not an attempt.
        assert_eq!(diagnostics.entries().len(), 0);
    }

    /// FR-062-AC-3's central clause (QSL-161): two independently
    /// constructed typing contexts, checking the same form, produce
    /// *identical checked output* -- not merely "each observes one
    /// diagnostic" (this test's own isolation half, kept below).
    ///
    /// **Rebuilt (QSL-161).** PR #262 review F6 deleted this test's earlier
    /// `staged_a.value == staged_b.value` assertion as a self-comparison
    /// (both sides came from the same deterministic call, so nothing could
    /// make it fail) and left the tag on what remained -- two diagnostic
    /// counts, each compared only to the literal `1` the loop already
    /// guarantees by construction. Comparing `CheckedDeclaration` by `==`
    /// is not available: its checked `Node` tree deliberately carries no
    /// `PartialEq` (`ir.rs`'s own doc on `Node`'s privacy), and adding one
    /// across the whole IR is a change no ticket here owns. This repo's own
    /// established substitute for exactly this situation --
    /// `qsl-eval/tests/it/collection_algebra.rs` and
    /// `collection_queries.rs` both compare `format!("{:?}", ..)` of two
    /// computed values where no `PartialEq` exists -- applies the same way
    /// here: `CheckedDeclaration` and `StageFailure<CheckRefusal>` both
    /// derive `Debug`, and two independently constructed, differently
    /// shaped contexts producing the same `Debug` text is a real assertion
    /// a divergent implementation could fail.
    ///
    /// **Not a self-comparison this time.** Context `b` is seeded with
    /// `unrelated` ahead of `helper` in `signatures_b` (not the plain
    /// `helper`-only set in `a`) -- an extra declaration `f`'s body never
    /// calls, placed so it also shifts `helper`'s own position in `b`'s
    /// signature table. If `check` leaked *any* shared state (a global
    /// cache keyed by declaration count or position, rather than reading
    /// only `cx`), the two outputs would diverge in some way this
    /// normalization does not account for; if it does not, they match
    /// exactly once each context's own legitimate, positional `function`/
    /// `callee` index is normalized to a name (see `normalize`, below) --
    /// `Signatures::callable`'s own doc records that this index is
    /// "index-aligned with the package's functions," so an extra
    /// declaration ahead of `helper` in `b`'s table is expected to change
    /// it there, without that being the state leak this test looks for.
    /// `f` itself takes a parameter and calls `helper` with it, rather
    /// than a bare literal, so there is a real call to check identically
    /// in both contexts, not just a trivial body neither context can
    /// diverge on.
    #[trace("TC-160", "FR-062-AC-3")]
    #[test]
    fn two_contexts_from_the_same_declarations_check_identically() {
        let scalar_limits = SCALAR_LIMITS_UNLIMITED;
        let form = FunctionDeclaration::new(
            "f",
            vec![("p0".to_owned(), boolean_type_form())],
            boolean_type_form(),
            None,
            Expression::Call {
                name: "helper".to_owned(),
                arguments: vec![Expression::Name("p0".to_owned())],
            },
        );
        let own_signature = boolean_signature("f", 1);

        let scope_a = empty_scope();
        let location_a = root_location();
        let signatures_a = Signatures::from(vec![boolean_signature("helper", 1)]);
        let declarations_a = declarations_for(
            &scope_a,
            &signatures_a,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location_a,
        );
        let mut meter_a = Meter::new(scalar_limits);
        let mut diagnostics_a = DiagnosticSink::default();
        let mut scopes_a = ScopeStack::default();
        let mut cx_a = CheckContext::new(
            &declarations_a,
            limits(),
            &mut meter_a,
            &mut diagnostics_a,
            &mut scopes_a,
        );
        let outcome_a = ValueFunctionFamily::check(&form, &mut cx_a);
        assert!(
            outcome_a.is_ok(),
            "the fixture must check successfully in context a: {outcome_a:?}"
        );

        let scope_b = empty_scope();
        let location_b = root_location();
        // `helper`, which `f`'s body actually calls, plus an extra
        // declaration `f`'s body never calls -- so a correct
        // implementation's output is unaffected, but a leaky one (reading
        // declaration count or position from somewhere other than `cx`)
        // would not be.
        let signatures_b = Signatures::from(vec![
            boolean_signature("unrelated", 0),
            boolean_signature("helper", 1),
        ]);
        let declarations_b = declarations_for(
            &scope_b,
            &signatures_b,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location_b,
        );
        let mut meter_b = Meter::new(scalar_limits);
        let mut diagnostics_b = DiagnosticSink::default();
        let mut scopes_b = ScopeStack::default();
        let mut cx_b = CheckContext::new(
            &declarations_b,
            limits(),
            &mut meter_b,
            &mut diagnostics_b,
            &mut scopes_b,
        );
        let outcome_b = ValueFunctionFamily::check(&form, &mut cx_b);
        assert!(
            outcome_b.is_ok(),
            "the fixture must check successfully in context b too: {outcome_b:?}"
        );

        // `helper`'s own index differs between `a` and `b` (0 and 1) purely
        // because `b`'s table lists `unrelated` first -- a legitimate,
        // positional property of `Signatures::callable` (its own doc: "index-
        // aligned with the package's functions"), not a state leak. Replace
        // each context's own index, wherever a `NodeKind::Call` or
        // `CallSite` embeds it, with a shared placeholder before comparing,
        // so this assertion is still sensitive to a real leak (anything else
        // that diverges) without failing on this expected difference.
        let index_a = signatures_a
            .callable("helper")
            .expect("helper is declared in signatures_a")
            .0;
        let index_b = signatures_b
            .callable("helper")
            .expect("helper is declared in signatures_b")
            .0;
        let normalize = |outcome: &str, index: usize| {
            outcome
                .replace(&format!("function: {index}"), "function: HELPER")
                .replace(&format!("callee: {index}"), "callee: HELPER")
        };
        assert_eq!(
            normalize(&format!("{outcome_a:?}"), index_a),
            normalize(&format!("{outcome_b:?}"), index_b),
            "two independently constructed typing contexts checking the same \
             form must produce identical checked output, once each one's own \
             legitimate signature-table position is normalized away"
        );

        // Each independently constructed sink shows exactly its own one
        // entry -- a shared/global sink would leak entries into whichever
        // one ran second, or show two entries in one and zero in the other.
        assert_eq!(diagnostics_a.entries().len(), 1);
        assert_eq!(diagnostics_b.entries().len(), 1);
    }

    /// Guards the top-level declaration-entry charge alone (limit 0
    /// refuses with actual counter 1, limit 1 admits a leaf-bodied
    /// declaration). Not tagged for FR-062-AC-7, whose own fixture-at-depth-D
    /// requirement [`real_checker_depth_limit_is_the_proximate_cause`] below
    /// addresses (see that test's own doc for why it is untagged, rather than
    /// retagged onto this narrower charge).
    #[trace("TC-432", "FR-062-AC-12")]
    #[test]
    fn nesting_depth_limit_is_the_proximate_cause() {
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let signatures = Signatures::default();
        let declarations = declarations_for(
            &scope,
            &signatures,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut tight = StageLimits {
            nesting_depth: 0,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        };
        let mut cx = CheckContext::new(
            &declarations,
            tight,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let form = declaration("f", Expression::Boolean(true));
        // The refused entry would have reached depth 1 against a bound of 0.
        match ValueFunctionFamily::check(&form, &mut cx) {
            Err(StageFailure::Limit(exceeded)) => {
                assert_eq!(exceeded, LimitExceeded::new(LimitKind::NestingDepth, 0, 1));
            }
            other => panic!("expected a nesting-depth Limit outcome, got {other:?}"),
        }

        tight.nesting_depth = 1;
        let mut cx = CheckContext::new(
            &declarations,
            tight,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let admitted = ValueFunctionFamily::check(&form, &mut cx);
        assert!(admitted.is_ok());
    }

    /// QSL-153: `StageLimits`' restored `input_bytes`/`node_count` each have
    /// a real producer (`measure_declaration`'s own pass) and
    /// a real consumer (`CheckContext::check_input_bytes`/
    /// `check_node_count`, called from `ValueFunctionFamily::check`) that
    /// changes behaviour: a limit configured one below the real, measured
    /// metric refuses with `Limit` naming that exact kind; the same limit
    /// at the metric itself admits -- the same "varies by exactly one"
    /// shape `nesting_depth`'s own test uses, so the limit (not the
    /// fixture) is shown to be the proximate cause. `work_budget` is a real
    /// producer and consumer too, but through the shared kernel meter's own
    /// `work_units` charge (PR #302 review finding 3), not a `StageLimits`
    /// field -- see `work_budget_kind_refuses_from_a_denied_meter_charge`.
    #[trace("TC-160", "FR-062-AC-5")]
    #[trace("TC-432", "FR-062-AC-12")]
    #[test]
    fn stage_limits_restored_kinds_refuse_one_below_the_real_metric() {
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let signatures = Signatures::default();
        let declarations = declarations_for(
            &scope,
            &signatures,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let form = declaration("f", Expression::Boolean(true));
        let metrics = measure_resolved(&empty_scope(), &form);
        assert!(metrics.input_bytes > 0 && metrics.node_count > 0);

        let base = StageLimits {
            nesting_depth: 128,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        };
        let check_kind = |limits: StageLimits, expected: LimitExceeded| {
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let mut diagnostics = DiagnosticSink::default();
            let mut scopes = ScopeStack::default();
            let mut cx = CheckContext::new(
                &declarations,
                limits,
                &mut meter,
                &mut diagnostics,
                &mut scopes,
            );
            match ValueFunctionFamily::check(&form, &mut cx) {
                Err(StageFailure::Limit(exceeded)) => {
                    assert_eq!(exceeded, expected);
                }
                other => panic!("expected {expected:?}, got {other:?}"),
            }
        };
        let admits = |limits: StageLimits| {
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let mut diagnostics = DiagnosticSink::default();
            let mut scopes = ScopeStack::default();
            let mut cx = CheckContext::new(
                &declarations,
                limits,
                &mut meter,
                &mut diagnostics,
                &mut scopes,
            );
            assert!(ValueFunctionFamily::check(&form, &mut cx).is_ok());
        };

        check_kind(
            StageLimits {
                input_bytes: metrics.input_bytes - 1,
                ..base
            },
            LimitExceeded::new(
                LimitKind::InputBytes,
                metrics.input_bytes - 1,
                u128::from(metrics.input_bytes),
            ),
        );
        admits(StageLimits {
            input_bytes: metrics.input_bytes,
            ..base
        });

        check_kind(
            StageLimits {
                node_count: metrics.node_count - 1,
                ..base
            },
            LimitExceeded::new(
                LimitKind::NodeCount,
                metrics.node_count - 1,
                u128::from(metrics.node_count),
            ),
        );
        admits(StageLimits {
            node_count: metrics.node_count,
            ..base
        });

        // A bound of 0, far below a larger declaration's measured counters:
        // the reported counter is the measured metric, not the bound plus
        // one.
        let larger = declaration(
            "f",
            Expression::If {
                condition: Box::new(Expression::Boolean(true)),
                then: Box::new(Expression::Boolean(false)),
                otherwise: Box::new(Expression::Boolean(true)),
            },
        );
        let measured = measure_resolved(&empty_scope(), &larger);
        assert!(measured.input_bytes > 1 && measured.node_count > 1);
        let limit_of = |limits: StageLimits| {
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let mut diagnostics = DiagnosticSink::default();
            let mut scopes = ScopeStack::default();
            let mut cx = CheckContext::new(
                &declarations,
                limits,
                &mut meter,
                &mut diagnostics,
                &mut scopes,
            );
            match ValueFunctionFamily::check(&larger, &mut cx) {
                Err(StageFailure::Limit(exceeded)) => exceeded,
                other => panic!("expected a Limit outcome, got {other:?}"),
            }
        };
        let bytes = limit_of(StageLimits {
            input_bytes: 0,
            ..base
        });
        assert_eq!(bytes.kind(), LimitKind::InputBytes);
        assert_eq!(bytes.configured_bound(), 0);
        assert_eq!(bytes.actual(), u128::from(measured.input_bytes));
        let nodes = limit_of(StageLimits {
            node_count: 0,
            ..base
        });
        assert_eq!(nodes.kind(), LimitKind::NodeCount);
        assert_eq!(nodes.configured_bound(), 0);
        assert_eq!(nodes.actual(), u128::from(measured.node_count));
    }

    /// PR #302 review finding 3: `WorkBudget` is a `Limit` outcome from a
    /// denied `cx.meter` charge, cumulative across every declaration
    /// checked against one meter. A budget one below one declaration's
    /// charge `w`, and a budget of 0, each refuse with actual counter `w`;
    /// a budget of exactly `w` admits the first check and refuses the
    /// second with actual counter `2w`.
    #[trace("TC-432", "FR-062-AC-12")]
    #[test]
    fn work_budget_kind_refuses_from_a_denied_meter_charge() {
        let scope = empty_scope();
        let location = root_location();
        let own_signature = declaration_signature("f");
        let signatures = Signatures::default();
        let declarations = declarations_for(
            &scope,
            &signatures,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let form = declaration("f", Expression::Boolean(true));
        let charge = measure_resolved(&empty_scope(), &form).work_budget;
        assert!(charge > 0);
        let limits = StageLimits {
            nesting_depth: 128,
            input_bytes: u64::MAX,
            node_count: u64::MAX,
        };
        // The stage limit `check` reached against `meter`, or `None` when
        // it admitted the declaration.
        let limit_of = |meter: &mut Meter| {
            let mut diagnostics = DiagnosticSink::default();
            let mut scopes = ScopeStack::default();
            let mut cx =
                CheckContext::new(&declarations, limits, meter, &mut diagnostics, &mut scopes);
            match ValueFunctionFamily::check(&form, &mut cx) {
                Ok(_) => None,
                Err(StageFailure::Limit(exceeded)) => Some(exceeded),
                Err(StageFailure::Refused(refusal)) => panic!("unexpected refusal {refusal:?}"),
            }
        };
        let budget = |work_units| {
            Meter::new(quire_exact::ScalarLimits {
                work_units,
                ..SCALAR_LIMITS_UNLIMITED
            })
        };
        let work_limit = |bound: u64, actual: u128| {
            Some(LimitExceeded::new(LimitKind::WorkBudget, bound, actual))
        };

        assert_eq!(
            limit_of(&mut budget(charge - 1)),
            work_limit(charge - 1, u128::from(charge))
        );
        // A budget of 0, below the charge: the counter is the charge, not
        // the budget plus one.
        let empty = limit_of(&mut budget(0)).expect("a zero budget refuses");
        assert_eq!(empty.kind(), LimitKind::WorkBudget);
        assert_eq!(empty.actual(), u128::from(charge));

        let mut meter = budget(charge);
        assert_eq!(limit_of(&mut meter), None);
        assert_eq!(
            limit_of(&mut meter),
            work_limit(charge, 2 * u128::from(charge))
        );
    }

    /// FR-062-AC-7's fixture-at-depth-D requirement, backed against
    /// `Typer`'s own pre-existing, already-correct
    /// [`crate::check::CheckingLimits`] depth bound -- not the contract's
    /// own `StageLimits.nesting_depth`. A body nested to depth D
    /// (`Not(Not(Not(true)))`, four levels deep counting the `Boolean`
    /// leaf) checked through `ValueFunctionFamily::check` -- reachable now
    /// that QSL-148 makes `check` call `check_declaration_body`, which
    /// drives the real `Typer` -- refuses at a configured depth of D-1 and
    /// admits at D, varying only the limit by exactly one.
    ///
    /// **Tagged for FR-062-AC-7 (QSL-236, M1).** AC-7's own text requires
    /// `check` to return a `Limit` outcome specifically; `check` now maps a
    /// Typer `CheckCause::ResourceExhausted` whose kind is `Depth` onto
    /// `StageFailure::Limit(LimitExceeded::new(LimitKind::NestingDepth,
    /// ..))`, so this test's refusal is that `Limit` outcome, carrying the
    /// configured bound and the actual depth the refused entry would have
    /// reached. Its declarations were not read from a unit, so it carries
    /// no `Locus` (FR-096; `locus_tests` covers the located case).
    #[test]
    #[trace("FR-062-AC-7")]
    fn real_checker_depth_limit_is_the_proximate_cause() {
        let scope = empty_scope();
        let location = root_location();
        let nested = Expression::Not(Box::new(Expression::Not(Box::new(Expression::Not(
            Box::new(Expression::Boolean(true)),
        )))));
        let form = declaration("f", nested);
        let own_signature = declaration_signature("f");

        let tight = CheckingLimits::new(u64::MAX, 3).expect("3 is within MAX_CHECKING_DEPTH");
        let signatures = Signatures::default();
        let declarations =
            declarations_for(&scope, &signatures, &own_signature, &[], tight, &location);
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let refused = ValueFunctionFamily::check(&form, &mut cx)
            .expect_err("a depth limit of 3 must refuse a body nested 4 deep");
        match refused {
            StageFailure::Limit(exceeded) => {
                assert_eq!(exceeded.kind(), LimitKind::NestingDepth);
                assert_eq!(exceeded.configured_bound(), 3);
                assert_eq!(exceeded.actual(), 4);
            }
            other => panic!("expected StageFailure::Limit, got {other:?}"),
        }

        let wide = CheckingLimits::new(u64::MAX, 4).expect("4 is within MAX_CHECKING_DEPTH");
        let signatures = Signatures::default();
        let declarations =
            declarations_for(&scope, &signatures, &own_signature, &[], wide, &location);
        let mut cx = CheckContext::new(
            &declarations,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let admitted = ValueFunctionFamily::check(&form, &mut cx);
        assert!(
            admitted.is_ok(),
            "a depth limit of 4 must admit the identical body nested exactly 4 deep"
        );
    }

    /// FR-062-AC-2/FR-065-AC-2: two structurally identical declarations,
    /// checked in two packages of one owner, mint one identity; a name
    /// change mints a different one.
    #[trace("TC-160", "FR-062-AC-2")]
    #[test]
    fn identical_declarations_share_one_identity() {
        let identity = |name: &str| {
            crate::check::PackageDeclarations {
                functions: vec![declaration(name, Expression::Boolean(true))],
                ..crate::check::PackageDeclarations::new(super::fixtures::fixture_source())
            }
            .check(CheckingLimits::default())
            .expect("the fixture checks")
            .function_identity(name)
            .expect("the fixture declares its function")
        };
        assert_eq!(identity("f"), identity("f"));
        assert_ne!(identity("f"), identity("g"));
    }

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

    /// FR-062-AC-2 (QSL-205): the keyed map gives every occurrence the
    /// ordinal the pre-QSL-205 scan gave it -- the number of earlier
    /// records with the same (identity, role) -- over an interleaved
    /// sequence of identities and roles, and resolves each one back to its
    /// own span.
    #[trace("TC-160", "FR-062-AC-2")]
    #[test]
    fn keyed_ordinals_match_counting_earlier_records() {
        let keys: Vec<NodeKey> = (1..=5)
            .map(|byte| NodeKey::from_digest([byte; 32]))
            .collect();
        let roles = ["declaration", "reference", "generated"];
        let mut map = OccurrenceMap::default();
        let mut recorded: Vec<(NodeKey, &str, Origin)> = Vec::new();
        // A fixed linear congruential sequence: deterministic, and it
        // interleaves every (identity, role) pair many times.
        let mut state = 7_u64;
        for span in 0..2_000_u64 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let key = keys[usize::try_from(state >> 33).unwrap() % keys.len()];
            let role = roles[usize::try_from(state >> 17).unwrap() % roles.len()];
            let expected = recorded
                .iter()
                .filter(|(other, other_role, _)| *other == key && *other_role == role)
                .count();
            let origin = map.record(key, role, span);
            assert_eq!(origin.ordinal(), u64::try_from(expected).unwrap());
            assert_eq!(origin.role().as_str(), role);
            recorded.push((key, role, origin));
        }
        for (span, (key, _, origin)) in (0_u64..).zip(&recorded) {
            assert_eq!(map.resolve(*key, origin), Some(&span));
            assert!(map.has(*key));
        }
        let unrecorded = NodeKey::from_digest([9; 32]);
        assert!(!map.has(unrecorded));
        assert_eq!(
            map.resolve(unrecorded, &Origin::new(Role::new("reference"), 0)),
            None
        );
        let past_the_end = Origin::new(Role::new("reference"), u64::MAX);
        assert_eq!(map.resolve(keys[0], &past_the_end), None);
    }

    /// QSL-205: a name's first signature and its first callable one are
    /// kept apart: a named call resolves to the callable one, while
    /// `position` names the first declared.
    #[test]
    fn signatures_keep_first_and_first_callable_positions() {
        let signature = |callable_by_name| Signature {
            name: "f".to_owned(),
            parameters: Vec::new(),
            result: ValueType::Boolean,
            callable_by_name,
        };
        let signatures = Signatures::new(vec![signature(false), signature(true), signature(true)]);
        assert_eq!(signatures.callable("f").map(|(index, _)| index), Some(1));
        assert_eq!(signatures.position("f"), Some(0));
        assert!(signatures.declares("f"));
        assert_eq!(signatures.callable("g").map(|(index, _)| index), None);
        assert_eq!(signatures.position("g"), None);
        let hidden = Signatures::new(vec![signature(false)]);
        assert!(hidden.callable("f").is_none());
        assert!(hidden.declares("f"));
    }

    /// QSL-205: grouping names in one pass refuses exactly the declarations
    /// whose name repeats, in declaration order, each with every locus of
    /// its name in declaration order -- the refusals the pairwise scan made.
    #[trace("TC-191", "FR-146-AC-8")]
    #[test]
    fn duplicate_names_are_refused_with_every_locus_in_order() {
        use crate::check::refusal::{Location as BodyLocation, Origin as BodyOrigin};
        let names = ["a", "b", "a", "c", "b", "a"];
        let refusals = crate::check::PackageDeclarations {
            functions: names
                .iter()
                .map(|name| super::fixtures::declaration(name, Expression::Boolean(true)))
                .collect(),
            ..crate::check::PackageDeclarations::new(super::fixtures::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect_err("repeated names are refused");
        let at = |index: usize| BodyLocation {
            origin: BodyOrigin::Body {
                function: names[index].to_owned(),
                index,
            },
            path: Vec::new(),
        };
        let expected: Vec<CheckRefusal> = [0, 1, 2, 4, 5]
            .into_iter()
            .map(|index| {
                let loci = (0..names.len())
                    .filter(|other| names[*other] == names[index])
                    .map(at)
                    .collect();
                CheckRefusal {
                    location: at(index),
                    cause: CheckCause::AmbiguousName {
                        name: names[index].to_owned(),
                        loci,
                    },
                }
            })
            .collect();
        assert_eq!(refusals, expected);
    }
}

/// FR-096: a family `check`'s limits name the locus where the charge
/// failed, over declarations S1, S2 and the FR-091 assembler read from real
/// source text.
#[cfg(test)]
mod locus_tests {
    use super::fixtures::{
        declaration, declaration_signature, declarations_for, empty_scope, limits,
        measure_resolved, root_location,
    };
    use super::*;
    use crate::check::refusal::Origin as CheckOrigin;
    use crate::check::refusal::StageLimitCause;
    use crate::check::PackageDeclarations;
    use crate::family::{CheckContext, DiagnosticSink, FamilyContract, ScopeStack, StageLimits};
    use ix_trace_rs::trace;
    use qsl_forms::{build_unit, FormsLimits};
    use qsl_foundation::diagnostic::{CatalogCode, CatalogCoded};
    use qsl_foundation::source::provenance::SourceRegion;
    use qsl_foundation::SourceIdentity;
    use quire_exact::{Meter, ScalarLimits};

    const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\" version \"1\" digest \
        \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
        function f using v(): Boolean pure { not not not true }\n";

    /// [`UNIT`] through S1, S2 and the assembler.
    fn unit() -> PackageDeclarations {
        let parsed = qsl_cst::parse(
            SourceIdentity::new("a", "u", "git", "1"),
            "unit.native",
            UNIT.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let forms = build_unit(&parsed, FormsLimits::default()).expect("S2 builds the unit");
        PackageDeclarations::assemble(parsed.source().reference().clone(), forms, Vec::new())
            .expect("the unit assembles")
    }

    fn body_location() -> CheckLocation {
        CheckLocation {
            origin: CheckOrigin::Body {
                function: "f".into(),
                index: 0,
            },
            path: Vec::new(),
        }
    }

    /// The unit bytes `locus` names, asserting it is a region of the unit.
    fn text(locus: Option<&Locus>, unit: &PackageDeclarations) -> &'static str {
        let Some(Locus::Region(region)) = locus else {
            panic!("expected a region locus, got {locus:?}");
        };
        assert_eq!(region.source(), &unit.source);
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        &UNIT[start..end]
    }

    /// The outcome of checking function 0 of `unit` under `checking`,
    /// `stage` and `meter`, located through `unit`'s regions.
    fn check_unit(
        unit: &PackageDeclarations,
        checking: CheckingLimits,
        stage: StageLimits,
        meter: &mut Meter,
    ) -> crate::family::CheckOutcome<CheckedDeclaration, CheckRefusal> {
        let scope = empty_scope();
        let signatures = Signatures::default();
        let own_signature = declaration_signature("f");
        let location = body_location();
        let regions = unit.regions();
        let declarations = ValueDeclarations {
            regions: Some(&regions),
            ..declarations_for(
                &scope,
                &signatures,
                &own_signature,
                &[],
                checking,
                &location,
            )
        };
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(&declarations, stage, meter, &mut diagnostics, &mut scopes);
        ValueFunctionFamily::check(&unit.functions[0], &mut cx)
    }

    fn limit(
        outcome: crate::family::CheckOutcome<CheckedDeclaration, CheckRefusal>,
    ) -> LimitExceeded {
        match outcome {
            Err(StageFailure::Limit(exceeded)) => exceeded,
            other => panic!("expected a stage limit, got {other:?}"),
        }
    }

    fn unlimited() -> Meter {
        Meter::new(SCALAR_LIMITS_UNLIMITED)
    }

    /// FR-096-AC-11: `Typer`'s depth stop on `not not not true` (four
    /// deep) under depth 3 is a nesting-depth limit with bound 3, actual 4,
    /// located at the span of `true` and coded
    /// `stage_limit_exceeded`/`nesting-depth-exceeded`; depth 4 admits. The
    /// same stop in a function not read from a unit carries no locus.
    #[trace("TC-378", "FR-096-AC-11", "FR-062-AC-7")]
    #[test]
    fn the_typer_depth_stop_is_located_at_the_node_whose_entry_failed() {
        let unit = unit();
        let tight = CheckingLimits::new(u64::MAX, 3).unwrap();
        let exceeded = limit(check_unit(&unit, tight, limits(), &mut unlimited()));
        assert_eq!(exceeded.kind(), LimitKind::NestingDepth);
        assert_eq!(exceeded.configured_bound(), 3);
        assert_eq!(exceeded.actual(), 4);
        assert_eq!(text(exceeded.locus(), &unit), "true");
        assert_eq!(
            exceeded.catalog_code(),
            CatalogCode::new("stage_limit_exceeded", "nesting-depth-exceeded")
        );

        let wide = CheckingLimits::new(u64::MAX, 4).unwrap();
        assert!(check_unit(&unit, wide, limits(), &mut unlimited()).is_ok());

        let scope = empty_scope();
        let signatures = Signatures::default();
        let own_signature = declaration_signature("f");
        let location = body_location();
        let synthesized =
            declarations_for(&scope, &signatures, &own_signature, &[], tight, &location);
        let mut meter = unlimited();
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &synthesized,
            limits(),
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let exceeded = limit(ValueFunctionFamily::check(&unit.functions[0], &mut cx));
        assert_eq!(exceeded.kind(), LimitKind::NestingDepth);
        assert_eq!(exceeded.locus(), None);
    }

    /// FR-096-AC-4: a declaration whose preimage input bytes exceed a bound
    /// `B` stops with kind input bytes, bound `B`, the measured bytes and
    /// the declaration's span. The same limit reached for a function not
    /// read from a unit carries no locus.
    #[trace("TC-427", "FR-096-AC-4")]
    #[test]
    fn a_declaration_input_bytes_limit_is_located_at_the_declaration() {
        let unit = unit();
        let measured = measure_resolved(&empty_scope(), &unit.functions[0]).input_bytes;
        let bound = measured - 1;
        let stage = StageLimits {
            input_bytes: bound,
            ..limits()
        };
        let exceeded = limit(check_unit(
            &unit,
            CheckingLimits::default(),
            stage,
            &mut unlimited(),
        ));
        assert_eq!(exceeded.kind(), LimitKind::InputBytes);
        assert_eq!(exceeded.configured_bound(), bound);
        assert_eq!(exceeded.actual(), u128::from(measured));
        assert_eq!(
            text(exceeded.locus(), &unit),
            "function f using v(): Boolean pure { not not not true }"
        );

        let scope = empty_scope();
        let signatures = Signatures::default();
        let own_signature = declaration_signature("f");
        let location = root_location();
        let synthesized = declarations_for(
            &scope,
            &signatures,
            &own_signature,
            &[],
            CheckingLimits::default(),
            &location,
        );
        let form = declaration("f", Expression::Boolean(true));
        let stage = StageLimits {
            input_bytes: 0,
            ..limits()
        };
        let mut meter = unlimited();
        let mut diagnostics = DiagnosticSink::default();
        let mut scopes = ScopeStack::default();
        let mut cx = CheckContext::new(
            &synthesized,
            stage,
            &mut meter,
            &mut diagnostics,
            &mut scopes,
        );
        let exceeded = limit(ValueFunctionFamily::check(&form, &mut cx));
        assert_eq!(exceeded.kind(), LimitKind::InputBytes);
        assert_eq!(exceeded.locus(), None);
    }

    /// The one stage-limit refusal package checking returns for `unit`
    /// under `limits`, with the region FR-096 locates it at.
    fn package_limit(
        unit: PackageDeclarations,
        limits: CheckingLimits,
    ) -> (StageLimitCause, Option<SourceRegion>) {
        let regions = unit.regions();
        let refusals = unit
            .check(limits)
            .expect_err("the limit stops package checking");
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal, got {refusals:?}");
        };
        let CheckCause::ResourceExhausted(cause) = &refusal.cause else {
            panic!("a stage limit, got {refusal:?}");
        };
        ((**cause).clone(), regions.refusal_region(refusal))
    }

    /// The bytes `region` names in [`UNIT`].
    fn unit_text(region: Option<SourceRegion>) -> &'static str {
        let region = region.expect("the limit is located");
        let start = usize::try_from(region.start()).unwrap();
        let end = usize::try_from(region.end()).unwrap();
        &UNIT[start..end]
    }

    /// FR-096 through package checking, `ValueFunctionFamily::check`'s
    /// production caller: `Typer`'s depth stop on `not not not true` under
    /// depth 3 keeps the region of `true`, and the declaration-level input
    /// bytes and work limits keep the declaration's span.
    #[trace("TC-427", "TC-378", "FR-096-AC-4", "FR-096-AC-5", "FR-096-AC-11")]
    #[test]
    fn package_checking_keeps_the_family_limit_region() {
        let (depth, region) = package_limit(unit(), CheckingLimits::new(u64::MAX, 3).unwrap());
        assert_eq!(depth.kind, CheckingLimitKind::Depth);
        assert_eq!((depth.limit, depth.actual), (3, 4));
        assert_eq!(unit_text(region), "true");

        let declaration = "function f using v(): Boolean pure { not not not true }";
        let bytes = measure_resolved(&empty_scope(), &unit().functions[0]).input_bytes;
        let (input, region) = package_limit(
            unit(),
            CheckingLimits::default().with_input_bytes(bytes - 1),
        );
        assert_eq!(input.kind, CheckingLimitKind::InputBytes);
        assert_eq!(unit_text(region), declaration);

        let charge = measure_resolved(&empty_scope(), &unit().functions[0]).work_budget;
        let (work, region) = package_limit(
            unit(),
            CheckingLimits::default().with_work_budget(charge - 1),
        );
        assert_eq!(work.kind, CheckingLimitKind::WorkBudget);
        assert_eq!(unit_text(region), declaration);
    }

    /// FR-096 through package checking: a function with no form spans (one
    /// synthesized for FR-151 dispatch) in a unit whose regions are
    /// supplied reaches `Typer`'s depth stop with no region.
    #[trace("TC-378", "FR-096-AC-11")]
    #[test]
    fn package_checking_locates_no_limit_in_a_function_without_spans() {
        let nested = Expression::Not(Box::new(Expression::Not(Box::new(Expression::Not(
            Box::new(Expression::Boolean(true)),
        )))));
        let mut unit = unit();
        unit.functions = vec![declaration("f", nested)];
        assert!(unit.functions[0].spans().is_none());
        let (depth, region) = package_limit(unit, CheckingLimits::new(u64::MAX, 3).unwrap());
        assert_eq!(depth.kind, CheckingLimitKind::Depth);
        assert_eq!(depth.region, None);
        assert_eq!(region, None);
    }

    /// FR-096-AC-5: a declaration whose work charge a budget `W` denies
    /// stops with kind work budget, bound `W`, the spend the denied charge
    /// would have reached, and the declaration's span.
    #[trace("TC-427", "FR-096-AC-5")]
    #[test]
    fn a_denied_work_charge_is_located_at_the_declaration() {
        let unit = unit();
        let charge = measure_resolved(&empty_scope(), &unit.functions[0]).work_budget;
        let bound = charge - 1;
        let mut meter = Meter::new(ScalarLimits {
            work_units: bound,
            ..SCALAR_LIMITS_UNLIMITED
        });
        let exceeded = limit(check_unit(
            &unit,
            CheckingLimits::default(),
            limits(),
            &mut meter,
        ));
        assert_eq!(exceeded.kind(), LimitKind::WorkBudget);
        assert_eq!(exceeded.configured_bound(), bound);
        assert_eq!(exceeded.actual(), u128::from(charge));
        let region: &SourceRegion = match exceeded.locus() {
            Some(Locus::Region(region)) => region,
            other => panic!("expected a region, got {other:?}"),
        };
        assert_eq!(unit.regions().declaration_region(0).as_ref(), Some(region));
        assert_eq!(
            text(exceeded.locus(), &unit),
            "function f using v(): Boolean pure { not not not true }"
        );
    }
}

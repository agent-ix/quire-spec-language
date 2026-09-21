// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 value expressions and total pure functions (FR-145, FR-146).
//!
//! [`crate::check::PackageDeclarations::check`] resolves names, types every
//! body and measure, checks every definedness obligation on a reachable
//! path and the `decreases` obligations of every recursive component, all
//! before any charge -- that checking-stage logic lives in the layer-3
//! [`crate::check`] module (ADR-011 §7.3 M-5, QSL-139/FR-068). This module
//! is what remains at layer 5 (S6a): [`CheckedPackage::call`] and
//! [`CheckedPackage::evaluate`] run already-checked code under a
//! [`Meter`](super::Meter), reaching `check`'s checked-output state only
//! through its public accessors, never through a private field (US-009).

mod evaluate;
mod family;

use super::accounting::Meter;
use super::composite::{Value, ValueType};
use super::reference::ObjectEnvironment;
use crate::family::ReferenceEvaluation;
use evaluate::{Callable, Machine};

pub use evaluate::{Evaluation, LocatedLoss, ValueLoss};
pub use family::{DecodeV2Error, InvalidQualifiedName, QualifiedName};

// ADR-011 §4's mechanism (FR-068-AC-10): `check` is the one defining module
// for these two checked-output types; this is the one, closed, non-glob
// re-export naming it, so `value::expression::CheckedPackage::call` remains
// the S6a entry point ADR-011 §7.3's M-5 row names, with `CheckedPackage`
// itself defined exactly once, in `check`.
pub use crate::check::{CheckedExpression, CheckedPackage};

/// A runtime input a call or evaluation refuses before any charge. Stays at
/// layer 5 (FR-068's refusal split): every *check-cause* type moved to
/// `crate::check` (FR-068-AC-4), but this one names an evaluation-time
/// input refusal, relocated here beside [`CheckedPackage::call`]'s and
/// [`CheckedPackage::evaluate`]'s admission code.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum InputRefusal {
    /// No function of this name: `missing_declaration` / `missing-name`.
    #[error("no function named {0}")]
    UnknownFunction(String),
    /// The argument count differs from the parameter count:
    /// `invalid_runtime_input` / `wrong-value-kind`.
    #[error("{supplied} arguments for {declared} parameters")]
    Arity {
        /// Declared parameters.
        declared: usize,
        /// Supplied arguments.
        supplied: usize,
    },
    /// An argument is not a value of its parameter type:
    /// `invalid_runtime_input` / `wrong-value-kind`.
    #[error("argument {parameter} is not a value of its declared type")]
    WrongValueKind {
        /// The parameter index.
        parameter: usize,
    },
    /// An argument holds a reference with no object in the complete
    /// population: `dangling_reference` / `absent-target-in-complete-population`.
    #[error("argument {parameter} holds a reference with no target object")]
    DanglingReference {
        /// The parameter index.
        parameter: usize,
    },
}

impl InputRefusal {
    /// The refusal code.
    pub fn code(&self) -> crate::diagnostic::Code {
        use crate::diagnostic::Code;
        match self {
            Self::UnknownFunction(_) => Code::MissingDeclaration,
            Self::Arity { .. } | Self::WrongValueKind { .. } => Code::InvalidRuntimeInput,
            Self::DanglingReference { .. } => Code::DanglingReference,
        }
    }

    /// The closed cause tag.
    pub fn cause(&self) -> &'static str {
        match self {
            Self::UnknownFunction(_) => "missing-name",
            Self::Arity { .. } | Self::WrongValueKind { .. } => "wrong-value-kind",
            Self::DanglingReference { .. } => "absent-target-in-complete-population",
        }
    }
}

/// Argument admission for [`CheckedPackage::call`] and
/// [`CheckedPackage::evaluate`]: neither touches `CheckedPackage`'s or
/// `CheckedExpression`'s private state, so it needs no `check` accessor.
fn validate(
    parameters: &[(String, ValueType)],
    arguments: &[Value],
    objects: &ObjectEnvironment,
) -> Result<(), InputRefusal> {
    if parameters.len() != arguments.len() {
        return Err(InputRefusal::Arity {
            declared: parameters.len(),
            supplied: arguments.len(),
        });
    }
    for (parameter, ((_, value_type), argument)) in parameters.iter().zip(arguments).enumerate() {
        if !value_type.admits(argument) {
            return Err(InputRefusal::WrongValueKind { parameter });
        }
        let mut pending = vec![argument];
        while let Some(value) = pending.pop() {
            match value {
                Value::Reference(reference) => {
                    if !objects.contains(reference) {
                        return Err(InputRefusal::DanglingReference { parameter });
                    }
                }
                Value::Option(option) => pending.extend(option.payload()),
                Value::Composite(composite) => {
                    pending.extend(composite.slots().iter().filter_map(|slot| match slot {
                        super::composite::FieldValue::Present(value) => Some(value),
                        super::composite::FieldValue::Absent
                        | super::composite::FieldValue::Null => None,
                    }));
                }
                Value::Collection(collection) => pending.extend(collection.elements()),
                Value::Boolean(_)
                | Value::Integer(_)
                | Value::Rational(_)
                | Value::Decimal(_)
                | Value::Float(_)
                | Value::Quantity(_)
                | Value::Text(_)
                | Value::Enum(_)
                | Value::Population(_) => {}
            }
        }
    }
    Ok(())
}

impl CheckedPackage {
    /// Every admitted function's own name, checked body and slot count, as
    /// [`evaluate::Callable`] -- built from `check`'s
    /// [`crate::check::CheckedPackage::function_states`] accessor, since
    /// `Callable` is a layer-5 type `check` itself must never construct
    /// (that would be a `check` -> `value::expression` edge, forbidden by
    /// FR-068-AC-3).
    fn callables(&self) -> Vec<Callable<'_>> {
        self.function_states()
            .map(|state| Callable {
                body: state.body,
                slots: state.slots,
                name: state.name,
            })
            .collect()
    }

    /// FR-062/FR-065: this package's `quire.checked-function-package/v2`
    /// bytes -- the checked-package producer's own public entry point.
    /// S4-links each identity first (`family::link_function_identity`),
    /// then emits directly through `family::emit_v2`. Reads no CST, no
    /// source text, only each function's already-checked identity, through
    /// `check`'s [`crate::check::CheckedPackage::function_identities`]
    /// accessor -- this method itself, not `check`, is what builds
    /// `QualifiedName` and calls the v2 codec, both `value::expression`
    /// types/functions `check` must not import (FR-068-AC-3).
    ///
    /// `Result<_, InvalidQualifiedName>`: the v2 wire format is typed on
    /// `QualifiedName`, matching `call`'s own `&QualifiedName` parameter,
    /// not a bare `String` a decode caller has to re-parse and re-validate
    /// one function over. A declared function name that is not
    /// identifier-shaped refuses here rather than either panicking or
    /// silently emitting v2 bytes that could never decode back into a
    /// `QualifiedName` anyway.
    pub fn emit_function_package_v2(&self) -> Result<Vec<u8>, InvalidQualifiedName> {
        let entries = self
            .function_identities()
            .map(|(name, identity)| {
                let linked = family::link_function_identity(identity);
                QualifiedName::unqualified(name.to_owned()).map(|name| (name, linked))
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(family::emit_v2(&entries))
    }

    /// Decode `quire.checked-function-package/v2` bytes emitted by
    /// [`Self::emit_function_package_v2`] back into (qualified name,
    /// identity) pairs, for a caller verifying identity survived the round
    /// trip (FR-065-AC-2).
    pub fn decode_function_package_v2(
        bytes: &[u8],
    ) -> Result<Vec<(QualifiedName, quire_exact::NodeKey)>, family::DecodeV2Error> {
        family::decode_v2(bytes)
    }

    /// Call the named function: `function.call`, then its body. Refused
    /// `InputRefusal::UnknownFunction` for a name `function` finds
    /// but whose `callable_by_name` is `false` -- the same refusal an
    /// undeclared name gets, not a distinct one -- so this public runtime
    /// entry point cannot reach a crate-internal FR-151 synthesized dispatch
    /// candidate body or effective precondition by name any more than an
    /// ordinary checked `Expression::Call` can (`check`'s own
    /// `callable_by_name` gate, TC-196 D07's bypass this closes at the other
    /// entry point).
    ///
    /// FR-065-AC-6/ADR-013 O-11: `function` is a typed [`QualifiedName`],
    /// never a bare `&str` -- this is the layer-6 `replay` facade's executor
    /// entry for `Value`'s function family. A name this package's
    /// declarations do not resolve refuses with `UnknownFunction`, naming
    /// it; it never falls back to a display-name string comparison.
    pub fn call(
        &self,
        function: &QualifiedName,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InputRefusal> {
        let name = function
            .as_unqualified()
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()))?;
        let callable = self
            .callable(name)
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()))?;
        validate(callable.parameters, &arguments, objects)?;
        // FR-062/FR-065: this family's own `evaluate` hook
        // (`crate::family::ReferenceEvaluation`) is the one path that runs
        // checked function-application code, not a second, parallel
        // `Machine` call beside it.
        let identity = callable.identity;
        // `ReferenceEvaluation::evaluate`'s `meter` parameter is
        // `ValueFunctionFamily`'s own `_meter: &mut quire_exact::Meter`
        // (`family.rs`), unused in its body today: ADR-012 §2/FR-062-AC-5
        // reserve this hook alone for a future meter-budget `Incomplete`
        // outcome, which `EvaluateRefusal`'s own doc states this ticket does
        // not add yet. `contract_meter` is therefore built unlimited and
        // passed to satisfy the trait's signature, not to bound anything
        // here -- this call site charges nothing against it and nothing
        // should observe that as a limit today. `meter` (this method's own
        // parameter, `EvaluationEnv::local_meter` below) is the accounting
        // path that is actually charged.
        let mut contract_meter = quire_exact::Meter::new(crate::check::SCALAR_LIMITS_UNLIMITED);
        let mut env = family::EvaluationEnv {
            package: self,
            objects,
            arguments: Some(arguments),
            local_meter: meter,
        };
        crate::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
            .map_err(|refusal| match refusal {
                // `EnvironmentAlreadyConsumed` cannot be produced by any call
                // through `call()`: this function always builds a fresh
                // `EvaluationEnv` with `Some(arguments)` and calls `evaluate`
                // exactly once. It is not folded into `UnknownFunction`: that
                // would report a purely internal invariant violation as a
                // public "no such function" input error to a caller who
                // supplied nothing wrong.
                crate::family::EvaluateRefusal::UnknownIdentity { .. } => {
                    InputRefusal::UnknownFunction(function.to_string())
                }
                crate::family::EvaluateRefusal::EnvironmentAlreadyConsumed => {
                    unreachable!(
                        "CheckedPackage::call built a fresh EvaluationEnv and must not reuse it"
                    )
                }
            })
    }

    /// Evaluate a checked expression with `arguments` for its parameters.
    pub fn evaluate(
        &self,
        expression: &CheckedExpression,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InputRefusal> {
        validate(expression.parameters(), &arguments, objects)?;
        let callables = self.callables();
        Ok(Machine::new(
            self.scope(),
            &callables,
            objects,
            meter,
            self.dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), arguments, false))
    }
}

#[cfg(test)]
mod tests {
    //! TC-174 (FR-068-AC-5): checking and evaluation produce identical
    //! results before and after QSL-139's split -- specifically, that
    //! `check`'s per-function accessor state (here, each function's own
    //! `slots` count, read directly through
    //! [`crate::check::CheckedPackage::function_states`], never inferred
    //! from whether [`Evaluation`] happens to expose it) is not sensitive to
    //! a function's position in the package's declaration list. This is the
    //! bug class the Description names: `check`'s accessor for one
    //! function's `slots` accidentally returning a different function's
    //! `slots`, or `CheckedPackage::call`'s admission validating against the
    //! wrong parameter list once fields became accessor calls instead of
    //! direct reads. It is unobservable with a single-function package, or
    //! with functions of identical shape (Description's own words), so the
    //! fixture below deliberately declares two functions differing in
    //! parameter count and body shape, and checks both a same-order and a
    //! swapped-order package to rule out position-keyed lookup.
    //!
    //! This module (not `check`) is this test's home: `check` must import
    //! nothing from `value::expression` at all (FR-068-AC-3, TC-172), even
    //! in test code, but `value::expression` legitimately depends on
    //! `check` -- this is the one module that can both call the checked
    //! package's public `call`/`evaluate` entry points and read `check`'s
    //! `pub(crate)` `function_states()` accessor directly in the same test.

    use super::*;
    use crate::check::{CheckCause, CheckRefusal, CheckingLimits, PackageDeclarations};
    use crate::forms::{BinaryOperator, Expression, FunctionDeclaration};
    use crate::value::ScalarLimits;
    use ix_trace_rs::trace;
    use quire_exact::Integer;

    const UNLIMITED: ScalarLimits = ScalarLimits {
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

    /// One parameter, no `let`: the minimal shape.
    fn function_one() -> FunctionDeclaration {
        FunctionDeclaration::new(
            "one",
            vec![("a".to_owned(), ValueType::Integer)],
            ValueType::Integer,
            None,
            Expression::Name("a".to_owned()),
        )
    }

    /// Two parameters and one `let`-bound local: deliberately a different
    /// parameter count and slot shape from [`function_one`], so this
    /// fixture discriminates a `check` accessor that mixes the two
    /// functions' state up (Description's own named requirement).
    fn function_two() -> FunctionDeclaration {
        FunctionDeclaration::new(
            "two",
            vec![
                ("a".to_owned(), ValueType::Integer),
                ("b".to_owned(), ValueType::Integer),
            ],
            ValueType::Integer,
            None,
            Expression::Let {
                name: "x".to_owned(),
                value: Box::new(Expression::Binary {
                    operator: BinaryOperator::Add,
                    left: Box::new(Expression::Name("a".to_owned())),
                    right: Box::new(Expression::Name("b".to_owned())),
                }),
                body: Box::new(Expression::Name("x".to_owned())),
            },
        )
    }

    fn declarations(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
        PackageDeclarations {
            functions,
            ..PackageDeclarations::default()
        }
    }

    fn slots_by_name(package: &CheckedPackage) -> std::collections::BTreeMap<String, usize> {
        package
            .function_states()
            .map(|state| (state.name.to_owned(), state.slots))
            .collect()
    }

    /// TC-174 steps 1-4: each function's own `slots` count, read directly,
    /// is unaffected by the other function's presence or by declaration
    /// order -- and the two functions' fixture is genuinely discriminating
    /// (their slot counts differ).
    #[trace("TC-174", "FR-068-AC-5")]
    #[test]
    fn function_slots_are_stable_across_declaration_order() {
        let solo_one = declarations(vec![function_one()])
            .check(CheckingLimits::default())
            .unwrap();
        let solo_two = declarations(vec![function_two()])
            .check(CheckingLimits::default())
            .unwrap();
        let solo_one_slots = slots_by_name(&solo_one)["one"];
        let solo_two_slots = slots_by_name(&solo_two)["two"];
        assert_ne!(
            solo_one_slots, solo_two_slots,
            "fixture must be discriminating: the two functions must not share a slot count"
        );

        let combo = declarations(vec![function_one(), function_two()])
            .check(CheckingLimits::default())
            .unwrap();
        let combo_slots = slots_by_name(&combo);
        assert_eq!(combo_slots["one"], solo_one_slots);
        assert_eq!(combo_slots["two"], solo_two_slots);

        let swapped = declarations(vec![function_two(), function_one()])
            .check(CheckingLimits::default())
            .unwrap();
        let swapped_slots = slots_by_name(&swapped);
        assert_eq!(swapped_slots["one"], solo_one_slots);
        assert_eq!(swapped_slots["two"], solo_two_slots);
    }

    /// TC-174 steps 1-4: `CheckedPackage::call` resolves each function by
    /// name, not position -- the same fixture, called through the public
    /// runtime entry point in both declaration orders, must produce the
    /// same result each time.
    #[trace("TC-174", "FR-068-AC-5")]
    #[test]
    fn call_resolves_by_name_regardless_of_declaration_order() {
        let objects = ObjectEnvironment::default();
        for functions in [
            vec![function_one(), function_two()],
            vec![function_two(), function_one()],
        ] {
            let package = declarations(functions)
                .check(CheckingLimits::default())
                .unwrap();
            let mut meter = Meter::new(UNLIMITED);
            let one = package
                .call(
                    &QualifiedName::unqualified("one").unwrap(),
                    vec![Value::Integer(Integer::from(5_i64))],
                    &objects,
                    &mut meter,
                )
                .unwrap();
            assert_eq!(
                format!("{:?}", one.outcome),
                format!(
                    "{:?}",
                    crate::value::Outcome::Completed(Value::Integer(Integer::from(5_i64)))
                )
            );
            let mut meter = Meter::new(UNLIMITED);
            let two = package
                .call(
                    &QualifiedName::unqualified("two").unwrap(),
                    vec![
                        Value::Integer(Integer::from(3_i64)),
                        Value::Integer(Integer::from(4_i64)),
                    ],
                    &objects,
                    &mut meter,
                )
                .unwrap();
            assert_eq!(
                format!("{:?}", two.outcome),
                format!(
                    "{:?}",
                    crate::value::Outcome::Completed(Value::Integer(Integer::from(7_i64)))
                )
            );
        }
    }

    /// TC-174 step 1's refusal fixture: a duplicate function name is
    /// refused `CheckCause::AmbiguousName`, unaffected by the split.
    #[trace("TC-174", "FR-068-AC-5")]
    #[test]
    fn duplicate_function_name_is_refused_ambiguous_name() {
        let result =
            declarations(vec![function_one(), function_one()]).check(CheckingLimits::default());
        let refusals = result.expect_err("a duplicate name must be refused, not admitted");
        assert!(refusals.iter().any(|refusal: &CheckRefusal| matches!(
            &refusal.cause,
            CheckCause::AmbiguousName { name, .. } if name == "one"
        )));
    }

    /// TC-174 step 1's `InputRefusal` fixture: calling an undeclared name
    /// refuses `UnknownFunction`, unaffected by the split.
    #[trace("TC-174", "FR-068-AC-5")]
    #[test]
    fn call_to_an_unknown_function_is_refused() {
        let package = declarations(vec![function_one()])
            .check(CheckingLimits::default())
            .unwrap();
        let objects = ObjectEnvironment::default();
        let mut meter = Meter::new(UNLIMITED);
        let result = package.call(
            &QualifiedName::unqualified("missing").unwrap(),
            vec![Value::Integer(Integer::from(1_i64))],
            &objects,
            &mut meter,
        );
        assert!(matches!(result, Err(InputRefusal::UnknownFunction(name)) if name == "missing"));
    }
}

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
//! [`Meter`](quire_exact::Meter), reaching `check`'s checked-output state only
//! through its public accessors, never through a private field (US-009).

mod evaluate;
mod family;

use super::composite::{Value, ValueType};
use super::reference::ObjectEnvironment;
use crate::family::ReferenceEvaluation;
use evaluate::{Callable, Machine};
use quire_exact::Meter;

pub use evaluate::{Evaluation, LocatedLoss, ValueLoss};
pub use family::{DecodeV2Error, InvalidQualifiedName, QualifiedName};

// ADR-011 §4's mechanism (FR-068-AC-10, amended by ADR-013 T-1/FR-087,
// QSL-158 S-3a): `CheckedExpression` is `check`'s own checked-output type,
// re-exported by this one, closed, non-glob line. `CheckedPackage` (S4
// in-process) is no longer `check`'s: it is layer-4 `checked_package`'s own
// canonical type (this package's checked declarations plus the checked
// dependency closure), re-exported by the second line below so that
// `value::expression::CheckedPackage::call` remains the S6a entry point
// ADR-011 §7.3's M-5 row names -- the sole closed re-export FR-087-AC-9/
// TC-256 requires, naming no other path and no glob.
pub use crate::check::CheckedExpression;
pub use crate::checked_package::CheckedPackage;

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
    pub fn code(&self) -> qsl_foundation::diagnostic::Code {
        use qsl_foundation::diagnostic::Code;
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
///
/// FR-089-AC-5's declared-maximum pairing is checked here, not only inside
/// the evaluator's own `Machine::resolve_population`
/// (`evaluate.rs`'s `AllInstances`/`Lookup` sites): `ValueType::admits`
/// (`composite.rs`) cannot perform it -- it has no access to the recorded
/// `PopulationId` -> `PopulationBinding` correspondence `objects` carries --
/// so admitting a `Population` argument by presence alone there would let a
/// parameter the checked body never reads (no `allInstances`/`lookup` call
/// on it) through with an unresolved identity or a mismatched declared
/// maximum, silently, whenever nothing consumes it. This function already
/// receives `objects` (used for the analogous `DanglingReference` check
/// below), so the real check belongs here, at admission, mirroring
/// FR-049-AC-2's "wrong-type entries refuse independently" for this crate's
/// own runtime-input admission boundary. Every `Population<T>[N]` argument
/// reaching a checked call is, by construction, a top-level parameter
/// (FR-153's own restriction: it is never nested, so it always reaches this
/// per-parameter loop directly), so this one check covers every reachable
/// case; `Machine::resolve_population`'s own check is kept as defence in
/// depth for a `Value::Population` the type checker's own structural
/// argument-type matching would otherwise have already ruled out at every
/// nested call site.
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
        if let (ValueType::Population(maximum), Value::Population(population_id)) =
            (value_type, argument)
        {
            let resolved = objects
                .resolve_population(*population_id)
                .is_some_and(|binding| binding.declared_maximum() == Some(*maximum));
            if !resolved {
                return Err(InputRefusal::WrongValueKind { parameter });
            }
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
    /// [`crate::check::CheckedGraph::function_states`] accessor, reached
    /// through this package's own [`Self::graph`] accessor (ADR-013 T-1,
    /// FR-087-AC-9/TC-256: `package` itself imports nothing from `check`
    /// beyond `CheckedGraph`; this module's own, separate,
    /// layer-5-depends-on-layer-3 edge is what reaches `check`-owned state
    /// here), since `Callable` is a layer-5 type `check` itself must never
    /// construct (that would be a `check` -> `value::expression` edge,
    /// forbidden by FR-068-AC-3).
    fn callables(&self) -> Vec<Callable<'_>> {
        self.graph()
            .function_states()
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
    /// `check`'s `CheckedGraph::function_identities` (`pub(crate)`, not
    /// part of this crate's public doc surface) accessor, reached through
    /// [`Self::graph`] -- this method itself, not `check` and not
    /// `package`, is what builds `QualifiedName` and calls the v2 codec,
    /// both `value::expression` types/functions `check` must not import
    /// (FR-068-AC-3).
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
            .graph()
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
            .graph()
            .callable(name)
            .ok_or_else(|| InputRefusal::UnknownFunction(function.to_string()))?;
        validate(callable.parameters, &arguments, objects)?;
        // FR-062/FR-065: this family's own `evaluate` hook
        // (`crate::family::ReferenceEvaluation`) is the one path that runs
        // checked function-application code, not a second, parallel
        // `Machine` call beside it.
        let identity = callable.identity;
        // `ReferenceEvaluation::evaluate`'s `meter` parameter is
        // `ValueFunctionFamily`'s own `meter: &mut quire_exact::Meter`
        // (`family.rs`), genuinely charged now (QSL-153: `Meter::charge` is
        // exported and `evaluate` charges `ChargePoint::FunctionCall` once,
        // per call -- see `ValueFunctionFamily::evaluate`'s own doc for why
        // that is the top-level call's *only* `function.call` charge, PR
        // #302 review finding 2).
        //
        // `contract_meter` is a second, structurally required `Meter`
        // instance -- `meter` (this method's own parameter) is already
        // mutably borrowed by `env.local_meter` below for the whole call,
        // so `evaluate`'s own `meter` parameter cannot alias it -- but it is
        // configured with `meter`'s own limits (`*meter.limits()`), not an
        // unconditionally unlimited stand-in: a caller who configures
        // `meter` with, say, `work_units: 0` genuinely cannot afford even
        // this call's own admission charge, and `EvaluateFailure::
        // Incomplete` is how that surfaces (mapped to a kernel
        // `Outcome::Incomplete` below, the same shape a denied
        // `env.local_meter` charge inside the call's own body already
        // takes -- one uniform way for a caller to observe "this call ran
        // out of budget," regardless of which internal meter denied it).
        let mut contract_meter = Meter::new(*meter.limits());
        let mut env = family::EvaluationEnv {
            package: self,
            objects,
            arguments: Some(arguments),
            local_meter: meter,
        };
        match crate::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
        {
            Ok(evaluation) => Ok(evaluation),
            // `EnvironmentAlreadyConsumed` cannot be produced by any call
            // through `call()`: this function always builds a fresh
            // `EvaluationEnv` with `Some(arguments)` and calls `evaluate`
            // exactly once. It is not folded into `UnknownFunction`: that
            // would report a purely internal invariant violation as a
            // public "no such function" input error to a caller who
            // supplied nothing wrong.
            Err(crate::family::EvaluateFailure::Refused(
                crate::family::EvaluateRefusal::UnknownIdentity { .. },
            )) => Err(InputRefusal::UnknownFunction(function.to_string())),
            Err(crate::family::EvaluateFailure::Refused(
                crate::family::EvaluateRefusal::EnvironmentAlreadyConsumed,
            )) => {
                unreachable!(
                    "CheckedPackage::call built a fresh EvaluationEnv and must not reuse it"
                )
            }
            // PR #302 review finding 2: no `unreachable!()` here. Denying
            // this call's own admission charge is a real, reachable outcome
            // now that `contract_meter` carries the caller's own configured
            // limits -- and it is a budget outcome, not an invalid-input
            // one, so it surfaces the same way a denied `env.local_meter`
            // charge already does: a kernel `Outcome::Incomplete`, inside a
            // successful `Evaluation`, never `InputRefusal` (whose own doc
            // scopes it to refusals made *before* any charge).
            Err(crate::family::EvaluateFailure::Incomplete(record)) => Ok(Evaluation {
                outcome: crate::value::Outcome::Incomplete(record),
                location: None,
                losses: Vec::new(),
            }),
        }
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
            self.graph().scope(),
            &callables,
            objects,
            meter,
            self.graph().dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), arguments))
    }
}

#[cfg(test)]
mod tests {
    //! TC-174 (FR-068-AC-5): checking and evaluation produce identical
    //! results before and after QSL-139's split -- specifically, that
    //! `check`'s per-function accessor state (here, each function's own
    //! `slots` count, read directly through
    //! [`crate::check::CheckedGraph::function_states`], never inferred
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
    //! `check` -- this is the one module that can read `check`'s
    //! `pub(crate)` `function_states()` accessor at all, and that accessor
    //! is same-crate-only (`pub(crate)`, not `pub`), so this one test stays
    //! here rather than moving to `tests/it` (QSL-183 edge audit).
    //!
    //! QSL-183: the other three fixtures formerly in this module
    //! (`call_resolves_by_name_regardless_of_declaration_order`,
    //! `duplicate_function_name_is_refused_ambiguous_name`,
    //! `call_to_an_unknown_function_is_refused`) touched only the public
    //! `PackageDeclarations::check`/`CheckedPackage::call` surface, with no
    //! need for `function_states()`, so they moved to
    //! `tests/it/checked_package_call.rs` -- an integration test, allowed to
    //! reach both `forms` and this crate's checking/evaluation pipeline.
    //! This test alone stays (it cannot leave the crate: `function_states()`
    //! is `pub(crate)`), and it builds its own `FunctionDeclaration`/
    //! `Expression`/`BinaryOperator` fixture through `crate::value`'s own
    //! re-export rather than a direct `crate::forms::` import (QSL-183 edge
    //! audit: "reaching layer 2 is allowed transitively through
    //! qsl-semantics" -- `value` is this crate's stand-in for that surface
    //! today; `crate::check` -- this test's other, `check`-owned import --
    //! is the layer-3 half of the same surface).

    use super::*;
    use crate::check::{CheckingLimits, PackageDeclarations};
    use crate::value::{BinaryOperator, Expression, FunctionDeclaration};
    use ix_trace_rs::trace;

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

    fn slots_by_name(
        graph: &crate::check::CheckedGraph,
    ) -> std::collections::BTreeMap<String, usize> {
        graph
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
}

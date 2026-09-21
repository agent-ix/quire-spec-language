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

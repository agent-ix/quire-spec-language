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
//! [`Meter`], reaching `check`'s checked-output state only
//! through its public accessors, never through a private field (US-009).
//!
//! The parsed-form types (`Expression` and its siblings) are defined once,
//! in the layer-2 `qsl-forms` crate; this module has no `syntax` submodule
//! (FR-067-AC-9, TC-169 step 1). The `use` below fails to compile, but not
//! as proof of absence by itself: this module is private, so the same `use`
//! would fail the same way for a `syntax` submodule that existed but stayed
//! private. `tests::value_expression_syntax_is_absent_from_the_module_tree`
//! carries the real absence check, against the file-per-module convention:
//!
//! ```compile_fail
//! use quire_spec_language::value::expression::syntax::Expression;
//! ```

mod causes;
mod evaluate;
mod family;

use super::composite::{Value, ValueType};
use super::reference::ObjectEnvironment;
use crate::family::{ReferenceEvaluation, S6aFamilyKind};
use evaluate::{Callable, Machine};
use qsl_foundation::diagnostic::InternalFault;
use quire_exact::{Meter, NodeKey};

pub use crate::family::{FamilyOutcome, FamilyResult};
pub use evaluate::{Evaluation, LocatedLoss, ValueLoss};
pub use family::{decode_function_package_v2, DecodeV2Error, InvalidQualifiedName, QualifiedName};

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

/// FR-090 "Bad arguments are refused at admission; a broken invariant is an
/// internal fault" (ADR-013 T-4; ADR-011 §2.3 E6 row): [`CheckedPackage::
/// call`]'s and [`CheckedPackage::evaluate`]'s error, split between an
/// ordinary caller-input refusal admission catches and a broken S6a
/// invariant admission cannot have let through -- never conflated into one
/// shape, since a caller that asks "was my input rejected" needs a different
/// answer than "did the runtime break".
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CallFailure {
    /// An argument refused at admission, before any S6a evaluation runs.
    #[error(transparent)]
    Input(#[from] InputRefusal),
    /// An S6a invariant broke (ADR-013 T-4): never a caller-input refusal,
    /// and never surfaced as a `FamilyResult` or as
    /// `Ok(Evaluation { outcome: Outcome::Refused(_), .. })`.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(qsl_foundation::diagnostic::InternalFault),
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
        // FR-089-AC-6 (QSL-131 V5): kernel `ValueType::admits` refuses every
        // `(Population, Population)` pair outright -- the declared-maximum
        // comparison is this QSL-layer check (FR-089-AC-5), not a
        // generic-admission side effect. `Population<T>[N]` is reachable
        // only as a bare parameter type (FR-153's own restriction), so this
        // is the one call site that needs to special-case it: every other
        // parameter type still goes through kernel `admits()` unchanged.
        if let (ValueType::Population(maximum), Value::Population(population_id)) =
            (value_type, argument)
        {
            let resolved = objects
                .resolve_population(*population_id)
                .is_some_and(|binding| binding.declared_maximum() == Some(*maximum));
            if !resolved {
                return Err(InputRefusal::WrongValueKind { parameter });
            }
        } else if !value_type.admits(argument) {
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

/// Every admitted function's own name, checked body and slot count, as
/// [`evaluate::Callable`] -- built from `check`'s
/// [`crate::check::CheckedGraph::function_states`] accessor, reached
/// through `package`'s own [`CheckedPackage::graph`] accessor (ADR-013 T-1,
/// FR-087-AC-9/TC-256: `package` itself imports nothing from `check`
/// beyond `CheckedGraph`; this module's own, separate,
/// layer-5-depends-on-layer-3 edge is what reaches `check`-owned state
/// here), since `Callable` is a layer-5 type `check` itself must never
/// construct (that would be a `check` -> `value::expression` edge,
/// forbidden by FR-068-AC-3).
///
/// A private free function, not a [`CheckedPackageEvaluation`] method: it is
/// [`CheckedPackageEvaluation::evaluate`]'s own internal plumbing, never a
/// caller-facing entry point.
fn callables(package: &CheckedPackage) -> Vec<Callable<'_>> {
    package
        .graph()
        .function_states()
        .map(|state| Callable {
            body: state.body,
            slots: state.slots,
            name: state.name,
        })
        .collect()
}

/// The ADR-011 S6a seam for a checked declaration (FR-090, ADR-012 §5.1 S1):
/// one hand-written arm per [`S6aFamilyKind`] variant, each calling that
/// family's `evaluate` hook, and no `_` arm. `S6aFamilyKind` has no
/// `Relation` variant, so no `Relation` declaration reaches S6a
/// (FR-090-AC-4).
///
/// The seam passes the hook's result through unchanged
/// (`EvalOutcome::Kernel(o)` as `FamilyOutcome::Evaluated(o)`,
/// `EvalOutcome::Family(r)` as `FamilyOutcome::FamilyEvaluated(r)`,
/// `Err(fault)` as `Err(fault)`) and builds the [`Evaluation`] from it and
/// the `location` and `losses` the hook recorded in `env` (FR-090-OQ-3
/// ruling). An `identity` the package does not resolve is
/// `Err(InternalFault)` naming stage S6a (FR-090-AC-3).
///
/// `identity` and `env` are `Value`'s `ReferenceEvaluation::Key` and `Env`,
/// because `Value` is the one S6a family. The next family to gain an
/// `S6aFamilyKind` variant brings its own `Key` and `Env`, and reshapes these
/// parameters in that change.
///
/// FR-063 seam: adding an `S6aFamilyKind` variant with no arm here fails
/// `--cfg seam_probe` with `E0004`.
#[deny(clippy::wildcard_enum_match_arm)]
#[deny(clippy::match_wildcard_for_single_variants)]
fn evaluate_declaration(
    family: S6aFamilyKind,
    identity: &NodeKey,
    env: &mut family::EvaluationEnv<'_>,
    meter: &mut Meter,
) -> Result<Evaluation, InternalFault> {
    let outcome = match family {
        S6aFamilyKind::Value => crate::check::ValueFunctionFamily::evaluate(identity, env, meter)?,
        // FR-063: no arm for `S6aFamilyKind::__SeamProbe` -- under
        // `--cfg seam_probe` this match is deliberately non-exhaustive
        // (`E0004`). Do not add a catch-all to make it compile.
    };
    Ok(Evaluation {
        outcome: outcome.into(),
        location: env.location.take(),
        losses: std::mem::take(&mut env.losses),
    })
}

/// `call`, `evaluate` and `emit_function_package_v2` over a
/// [`CheckedPackage`] (ADR-011 §4, ADR-013 T-1, AD-016 Owner decision 6).
/// Once `CheckedPackage` is `qsl-package`'s own foreign type (X-7), an
/// inherent `impl CheckedPackage` here is E0116, so layer 5 exposes its
/// evaluator over layer 4's typestate through this trait. Both
/// `pkg.call(..)` and `CheckedPackage::call(&pkg, ..)` resolve through it.
///
/// Sealed: [`CheckedPackage`] is the one implementor.
pub trait CheckedPackageEvaluation: family::sealed::Sealed {
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
    ///
    /// FR-090: returns `Ok(Evaluation { outcome: FamilyOutcome::Evaluated(o),
    /// .. })` for the kernel evaluation outcome unchanged,
    /// `Ok(Evaluation { outcome: FamilyOutcome::FamilyEvaluated(r), .. })`
    /// for a family-owned evaluation-time result, or
    /// `Err(CallFailure::Fault(_))` for a broken S6a invariant -- the same
    /// three outcomes [`Self::evaluate`] returns. `location` and `losses`
    /// are the ones the `ValueFunctionFamily::evaluate` hook records in its
    /// `EvaluationEnv` (FR-090-OQ-3 ruling).
    fn call(
        &self,
        function: &QualifiedName,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// Evaluate a checked expression with `arguments` for its parameters.
    ///
    /// FR-090's S6a entry point for a checked clause expression: it admits
    /// `arguments` before S6a, then returns S6a's `Ok` result unchanged --
    /// `Ok(Evaluation { outcome: FamilyOutcome::Evaluated(o), .. })` for the
    /// kernel evaluation outcome, or `Ok(Evaluation { outcome:
    /// FamilyOutcome::FamilyEvaluated(r), .. })` for a family-owned
    /// evaluation-time result (FR-090-AC-7, AC-8, AC-11, AC-12) -- and an S6a
    /// `Err(InternalFault)` as `Err(CallFailure::Fault(_))`.
    fn evaluate(
        &self,
        expression: &CheckedExpression,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// FR-062/FR-065: this package's `quire.checked-function-package/v2`
    /// bytes -- the checked-package producer's own public entry point.
    /// S4-links each identity first (`family::link_function_identity`),
    /// then emits directly through `family::emit_v2`. Reads no CST, no
    /// source text, only each function's already-checked identity, through
    /// `check`'s `CheckedGraph::function_identities` (`pub(crate)`, not
    /// part of this crate's public doc surface) accessor, reached through
    /// [`CheckedPackage::graph`] -- this method itself, not `check` and not
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
    fn emit_function_package_v2(&self) -> Result<Vec<u8>, InvalidQualifiedName>;
}

impl CheckedPackageEvaluation for CheckedPackage {
    fn call(
        &self,
        function: &QualifiedName,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
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
        // this call's own admission charge, and a denied charge surfaces as
        // `Ok(FamilyOutcome::Evaluated(Outcome::Incomplete(_)))`, the same
        // shape a denied `env.local_meter` charge inside the call's own body
        // already takes -- one uniform way for a caller to observe "this
        // call ran out of budget," regardless of which internal meter
        // denied it.
        let mut contract_meter = Meter::new(*meter.limits());
        let mut env = family::EvaluationEnv::new(self, objects, arguments, meter);
        evaluate_declaration(
            S6aFamilyKind::Value,
            &identity,
            &mut env,
            &mut contract_meter,
        )
        .map_err(CallFailure::Fault)
    }

    fn evaluate(
        &self,
        expression: &CheckedExpression,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        validate(expression.parameters(), &arguments, objects)?;
        let callables = callables(self);
        Machine::new(
            self.graph().scope(),
            &callables,
            objects,
            meter,
            self.graph().dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), arguments)
        .map_err(CallFailure::Fault)
    }

    fn emit_function_package_v2(&self) -> Result<Vec<u8>, InvalidQualifiedName> {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::{CheckingLimits, PackageDeclarations, SCALAR_LIMITS_UNLIMITED};
    use crate::family::EvalOutcome;
    use ix_trace_rs::trace;
    use qsl_forms::{Expression, FunctionDeclaration, TypeForm};
    use qsl_foundation::diagnostic::Category;
    use quire_exact::{Integer, NodeKey};

    /// TC-384's own fixture: `id(x: Integer[0,10]): Integer[0,10] = x`.
    fn identity_function() -> FunctionDeclaration {
        let bound = || {
            TypeForm::builtin(
                qsl_forms::BuiltinType::Int,
                qsl_foundation::Span { start: 0, end: 0 },
            )
            .with_bounds(vec!["0".to_owned(), "10".to_owned()])
        };
        FunctionDeclaration::new(
            "id",
            vec![("x".to_owned(), bound())],
            bound(),
            None,
            Expression::Name("x".to_owned()),
        )
    }

    /// TC-384 (FR-090-AC-3): the two S6a invariant breaks -- a consumed
    /// evaluation environment, and a checked identity the package does not
    /// resolve -- return `Err(InternalFault)` from the S6a seam itself
    /// (`ValueFunctionFamily::evaluate`), name stage `"S6a"`, carry category
    /// `Category::InternalFailure`, and never share one invariant
    /// identifier. Asserts `evaluate`'s own result directly, not
    /// `CheckedPackage::call`'s wrapping: the fault this test is about is
    /// produced entirely inside the seam, not by `call`'s own match arm.
    /// `EvaluationEnv`'s one real (non-test) constructor is `CheckedPackage::
    /// call`, which always builds a fresh one and calls `evaluate` exactly
    /// once, so reaching the consumed-environment and unknown-identity
    /// conditions here bypasses `call` the same way `family.rs`'s own
    /// `evaluate_faults_on_a_second_call_on_the_same_env` does.
    ///
    /// Previously: the consumed-environment case ended `CheckedPackage::
    /// call` in `unreachable!()`, and the unknown-identity case was folded
    /// into `InputRefusal::UnknownFunction` -- a purely internal invariant
    /// reported as an ordinary "no such function" input refusal to a caller
    /// who supplied nothing wrong. Neither survives this change.
    #[trace("FR-090-AC-3", "TC-384")]
    #[test]
    fn s6a_invariant_breaks_are_internal_faults_not_panics() {
        let graph = PackageDeclarations {
            functions: vec![identity_function()],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("id(x: Integer[0,10]): Integer[0,10] = x checks cleanly");
        let identity = graph
            .function_identity("id")
            .expect("id is declared in this package");
        let package = crate::checked_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::default();

        // Steps 2-3: one evaluation environment, carrying the arguments
        // [3], consumed by a first, real S6a call.
        let mut local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut env = family::EvaluationEnv::new(
            &package,
            &objects,
            vec![Value::Integer(Integer::from(3_i64))],
            &mut local_meter,
        );
        let mut contract_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let first =
            crate::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
                .expect("the first call, with real arguments still present, evaluates cleanly");
        assert!(
            matches!(
                &first,
                EvalOutcome::Kernel(quire_exact::Outcome::Completed(Value::Integer(value)))
                    if *value == Integer::from(3_i64)
            ),
            "expected EvalOutcome::Kernel(Outcome::Completed(Integer(3))), got {first:?}",
        );

        // Step 4: a second S6a call on that same, now-consumed environment.
        let consumed_fault =
            crate::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut contract_meter)
                .expect_err(
                    "a second call on the same env, arguments already consumed, must fault",
                );
        assert_eq!(consumed_fault.stage(), "S6a");
        assert_eq!(consumed_fault.category(), Category::InternalFailure);
        assert_eq!(
            consumed_fault.invariant(),
            "evaluation-environment-arguments-already-consumed"
        );

        // Step 5: a fresh environment, called with a NodeKey naming no
        // function in this package.
        let unknown_identity = NodeKey::from_digest([0xAB; 32]);
        let mut fresh_local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut fresh_env =
            family::EvaluationEnv::new(&package, &objects, Vec::new(), &mut fresh_local_meter);
        let mut fresh_contract_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let unknown_fault = crate::check::ValueFunctionFamily::evaluate(
            &unknown_identity,
            &mut fresh_env,
            &mut fresh_contract_meter,
        )
        .expect_err("an identity this package never declared must fault");
        assert_eq!(unknown_fault.stage(), "S6a");
        assert_eq!(unknown_fault.category(), Category::InternalFailure);
        assert_eq!(
            unknown_fault.invariant(),
            "checked-identity-not-resolved-by-package"
        );

        assert_ne!(
            consumed_fault.invariant(),
            unknown_fault.invariant(),
            "the two invariant identifiers must differ"
        );
    }

    /// TC-385 step 1: an exhaustive `match` with no `_` arm over the S6a
    /// family kind, one arm per `ReferenceEvaluation` family and no
    /// `Relation` arm. A new `S6aFamilyKind` variant fails this to compile
    /// (`E0004`), and a `Relation` variant cannot be added without an arm
    /// here naming it.
    fn s6a_family_name(family: S6aFamilyKind) -> &'static str {
        match family {
            S6aFamilyKind::Value => "Value",
        }
    }

    /// TC-385 step 2: an exhaustive `match` with no `_` arm over a
    /// `FamilyOutcome<Value>`, naming exactly `Evaluated` and
    /// `FamilyEvaluated`. A third variant fails this to compile (`E0004`).
    fn outcome_arm(outcome: &FamilyOutcome<Value>) -> &'static str {
        match outcome {
            FamilyOutcome::Evaluated(_) => "Evaluated",
            FamilyOutcome::FamilyEvaluated(_) => "FamilyEvaluated",
        }
    }

    /// TC-385 (FR-090-AC-4): the S6a family kind has no `Relation` variant
    /// and the seam's family parameter has that type; `FamilyOutcome` has
    /// exactly two arms. Each S6a family kind, passed to the seam with a
    /// package that declares no item of that family, returns
    /// `Err(InternalFault)` naming S6a for the unresolved identity, not a
    /// panic. A declared identity passed through the same seam returns
    /// `FamilyOutcome::Evaluated`; `tests/it/model_reference_queries.rs`'s
    /// TC-385 test takes the `FamilyEvaluated` arm through the seam.
    #[trace("FR-090-AC-4", "TC-385")]
    #[test]
    fn s6a_family_kind_admits_no_relation_and_family_outcome_has_two_arms() {
        let empty = crate::checked_package::CheckedPackage::link(
            PackageDeclarations::default()
                .check(CheckingLimits::default())
                .expect("an empty package checks cleanly"),
        );
        let objects = ObjectEnvironment::default();
        let undeclared = NodeKey::from_digest([0xAB; 32]);
        for kind in S6aFamilyKind::ALL {
            let name = s6a_family_name(kind);
            let mut local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let mut env =
                family::EvaluationEnv::new(&empty, &objects, Vec::new(), &mut local_meter);
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let fault = evaluate_declaration(kind, &undeclared, &mut env, &mut meter)
                .expect_err("a package that declares no item of the family resolves no identity");
            assert_eq!(fault.stage(), "S6a", "{name}");
            assert_eq!(fault.category(), Category::InternalFailure, "{name}");
            assert_eq!(
                fault.invariant(),
                "checked-identity-not-resolved-by-package",
                "{name}"
            );
        }

        let graph = PackageDeclarations {
            functions: vec![identity_function()],
            ..PackageDeclarations::default()
        }
        .check(CheckingLimits::default())
        .expect("id(x: Integer[0,10]): Integer[0,10] = x checks cleanly");
        let identity = graph
            .function_identity("id")
            .expect("id is declared in this package");
        let package = crate::checked_package::CheckedPackage::link(graph);
        let mut local_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let mut env = family::EvaluationEnv::new(
            &package,
            &objects,
            vec![Value::Integer(Integer::from(3_i64))],
            &mut local_meter,
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let evaluation =
            evaluate_declaration(S6aFamilyKind::Value, &identity, &mut env, &mut meter)
                .expect("a declared identity evaluates");
        assert_eq!(outcome_arm(&evaluation.outcome), "Evaluated");
        assert!(
            matches!(
                &evaluation.outcome,
                FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Integer(value)))
                    if *value == Integer::from(3_i64)
            ),
            "{:?}",
            evaluation.outcome
        );
    }

    /// TC-169 step 1 / FR-067-AC-9: `value::expression::syntax` is absent
    /// from the module tree, checked directly against this repository's
    /// file-per-module convention (`value::expression::mod`'s own `mod X;`
    /// declarations map 1:1 to `src/value/expression/X.rs`), rather than
    /// only through the `compile_fail` doctest on this module's doc, which
    /// cannot by itself distinguish "absent" from "still present but
    /// private".
    #[trace("TC-169", "FR-067-AC-9")]
    #[test]
    fn value_expression_syntax_is_absent_from_the_module_tree() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        assert!(
            !manifest_dir.join("src/value/expression/syntax.rs").exists(),
            "src/value/expression/syntax.rs still exists on disk"
        );
        let mod_rs = std::fs::read_to_string(manifest_dir.join("src/value/expression/mod.rs"))
            .expect("src/value/expression/mod.rs exists");
        let declares_syntax_module = mod_rs
            .lines()
            .any(|line| line.trim() == "mod syntax;" || line.trim() == "pub mod syntax;");
        assert!(
            !declares_syntax_module,
            "value::expression::mod still declares a syntax submodule"
        );
    }
}

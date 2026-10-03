// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 value expressions and total pure functions (FR-145, FR-146).
//!
//! [`qsl_semantics::check::PackageDeclarations::check`] resolves names, types every
//! body and measure, checks every definedness obligation on a reachable
//! path and the `decreases` obligations of every recursive component, all
//! before any charge -- that checking-stage logic lives in the layer-3
//! [`qsl_semantics::check`] module (ADR-011 §7.3 M-5, FR-068). This module
//! is what remains at layer 5 (S6a): [`CheckedPackage::call`] and
//! [`CheckedPackage::evaluate`] run already-checked code under a
//! [`Meter`], reaching `check`'s checked-output state only
//! through its public accessors, never through a private field (US-009).

mod causes;
mod evaluate;
mod family;
mod s6a;

use evaluate::Machine;
use qsl_foundation::diagnostic::InternalFault;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{FieldValue, Meter, NodeKey, Value, ValueType};
use quire_semantic_value::call::InputRefusal;
use s6a::{ReferenceEvaluation, S6aFamilyKind};

pub use evaluate::Evaluation;
pub use family::{InvalidQualifiedName, QualifiedName};
pub use s6a::separation::{
    ClauseEvaluation, ObservationIdentity, RuntimeValuePath, Separation, SeparationStep,
    StopReport, ValuePathStep, ValuePathSubject, WitnessClaim,
};

// `CheckedExpression` is `check`'s own checked-output type; this module
// imports it from `qsl_semantics::check` and re-exports none of it (FR-068-AC-10 is
// retired: ADR-011 §7.2 forbids a root re-export of an item that
// moves to `qsl-semantics`). `CheckedPackage` (S4 in-process) is layer 4's
// own canonical type, imported from `qsl-package` and re-exported by none of
// this crate (FR-087-AC-9 as amended): callers name it at
// `qsl_package::CheckedPackage` and bring `CheckedPackageEvaluation` into
// scope to call it.
use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedExpression;

/// The F-layer diagnostic code of an [`InputRefusal`], as the catalog enum.
/// The refusal lives in the `no_std` leaf `quire-semantic-value`, which
/// names the same code as a string ([`InputRefusal::code`]) but cannot name
/// `qsl-foundation`'s enum, so the enum mapping is made here, beside
/// [`CheckedPackage::call`]'s and [`CheckedPackage::evaluate`]'s admission
/// code.
pub fn input_refusal_code(refusal: &InputRefusal) -> qsl_foundation::diagnostic::Code {
    use qsl_foundation::diagnostic::Code;
    match refusal {
        InputRefusal::UnknownFunction(_) | InputRefusal::UnknownClause(_) => {
            Code::MissingDeclaration
        }
        InputRefusal::Arity { .. } | InputRefusal::WrongValueKind { .. } => {
            Code::InvalidRuntimeInput
        }
        InputRefusal::DanglingReference { .. } => Code::DanglingReference,
        InputRefusal::ObservationsMismatch => Code::InvalidRuntimeInput,
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

impl CallFailure {
    /// FR-285: an admission refusal is a refusal, a broken S6a invariant an
    /// internal failure (ADR-013 O-16).
    pub fn category(&self) -> qsl_foundation::diagnostic::Category {
        use qsl_foundation::diagnostic::Category;
        match self {
            Self::Input(_) => Category::Refusal,
            Self::Fault(fault) => fault.category(),
        }
    }
}

/// FR-115: [`CheckedPackageEvaluation::evaluate_frame`]'s result: the S6a
/// evaluation, unchanged, and the evaluated frame witness when the frame
/// check found a change outside the frame.
#[derive(Debug)]
pub struct FrameEvaluation {
    /// The S6a evaluation: `Completed(true)` when nothing changed outside
    /// the frame, `Completed(false)` when something did, or the family's
    /// refusal when the frame cannot be evaluated over the invocation.
    pub evaluation: Evaluation,
    /// The frame witness: present exactly when the outcome is
    /// `Completed(false)`.
    pub witness: Option<Box<qsl_semantics::model::observation::FrameWitness>>,
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
        // FR-089-AC-6: kernel `ValueType::admits` refuses every
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
                .is_some_and(|binding| binding.declared_maximum() == *maximum);
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
                    if !objects.objects().contains(reference) {
                        return Err(InputRefusal::DanglingReference { parameter });
                    }
                }
                Value::Option(option) => pending.extend(option.payload()),
                Value::Composite(composite) => {
                    pending.extend(composite.slots().iter().filter_map(|slot| match slot {
                        FieldValue::Present(value) => Some(value),
                        FieldValue::Absent | FieldValue::Null => None,
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

/// The evaluation environment `evaluate_declaration` runs `identity`
/// against: exactly one variant per [`S6aFamilyKind`] family (FR-107),
/// each carrying its own family's real `Env<'a>`. A caller always
/// pairs a `family` with its own matching variant; the mismatched pairs
/// `evaluate_declaration`'s own match handles are a broken invariant, never
/// reachable through [`CheckedPackageEvaluation::call`] or
/// [`CheckedPackageEvaluation::evaluate_clause`].
enum EvaluationTarget<'e, 'a> {
    Value(&'e mut family::EvaluationEnv<'a>),
    ProtocolClause(&'e mut s6a::protocol_clause::ProtocolClauseEnv<'a>),
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
    env: EvaluationTarget<'_, '_>,
    meter: &mut Meter,
) -> Result<Evaluation, InternalFault> {
    let mismatch = || InternalFault::new("S6a", "family-and-evaluation-environment-disagree");
    let (outcome, location, losses) = match (family, env) {
        (S6aFamilyKind::Value, EvaluationTarget::Value(env)) => {
            let outcome =
                qsl_semantics::check::ValueFunctionFamily::evaluate(identity, env, meter)?;
            (
                outcome.into(),
                env.location.take(),
                std::mem::take(&mut env.losses),
            )
        }
        (S6aFamilyKind::ProtocolClause, EvaluationTarget::ProtocolClause(env)) => {
            let outcome =
                qsl_semantics::check::ProtocolClauseFamily::evaluate(identity, env, meter)?;
            (
                outcome.into(),
                env.location.take(),
                std::mem::take(&mut env.losses),
            )
        }
        (S6aFamilyKind::Value, EvaluationTarget::ProtocolClause(_))
        | (S6aFamilyKind::ProtocolClause, EvaluationTarget::Value(_)) => return Err(mismatch()),
        // FR-063: no arm for `S6aFamilyKind::__SeamProbe` -- under
        // `--cfg seam_probe` this match is deliberately non-exhaustive
        // (`E0004`). Do not add a catch-all to make it compile.
        //
        // The arm below exists only in the probe's build of the crates
        // above `qsl-eval` (`--cfg seam_probe_eval_downstream`):
        // `qsl-replay` depends on this crate, so it must compile there
        // for the root crate's own seams to be reached at all.
        #[cfg(seam_probe_eval_downstream)]
        (S6aFamilyKind::__SeamProbe, _) => {
            unreachable!("never constructed outside the probe build")
        }
    };
    Ok(Evaluation {
        outcome,
        location,
        losses,
    })
}

/// `call` and `evaluate` over a
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
    /// FR-090: returns `Ok(Evaluation { outcome: FamilyOutcome::Evaluated(o),.
    /// . })` for the kernel evaluation outcome unchanged -- except a kernel
    /// `Refusal::CheckedInvariant`, which never reaches this `Ok` arm at all
    /// (FR-096-AC-15): S6a's own seam turns it into `Err(InternalFault)`
    /// before an `Outcome` is ever built --,
    /// `Ok(Evaluation { outcome: FamilyOutcome::FamilyEvaluated(r), .. })`
    /// for a family-owned evaluation-time result, or
    /// `Err(CallFailure::Fault(_))` for a broken S6a invariant, including
    /// that `CheckedInvariant` case -- the same three outcomes
    /// [`Self::evaluate`] returns. `location` and `losses` are the ones the
    /// `ValueFunctionFamily::evaluate` hook records in its `EvaluationEnv`
    /// (FR-090-OQ-3 ruling).
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
    /// `Err(InternalFault)` as `Err(CallFailure::Fault(_))`, which is where a
    /// kernel `Refusal::CheckedInvariant` surfaces (FR-096-AC-15): it is
    /// never part of the unchanged `o` above.
    fn evaluate(
        &self,
        expression: &CheckedExpression,
        arguments: Vec<Value>,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// FR-107: evaluate one checked state clause over `observations`
    /// (FR-106's `AdmittedObservations`), the `ProtocolClause` S6a family's
    /// entry point beside [`Self::call`]'s `Value` one.
    ///
    /// Refuses `InputRefusal::UnknownClause` (naming `clause`) when the name
    /// resolves to no state clause, or `InputRefusal::ObservationsMismatch`
    /// when `observations` were admitted for a different clause, in either
    /// case without charging `meter`.
    ///
    /// FR-265: the result carries, beside the evaluation, what the
    /// evaluation recorded on the claim's own level: each decision and each
    /// quantifier stop report ([`ClauseEvaluation`]).
    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &qsl_semantics::model::observation::AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure>;

    /// FR-268 (ADR-031 SW-13): check that `claim` separates the state
    /// clause `clause` over `observations`, charged to `meter`: step 1
    /// resolves the claim's quantifier to a `forall` or `exists` node of the
    /// clause's claim reached through `let` bodies, `if` branches, `not`
    /// and the operands of `and`, `or` and `implies`; step 2 evaluates its
    /// domain in the clause's bindings and the `let` bindings on that path;
    /// step 3 reads the element at the claim's index and compares its value
    /// path and value; step 4 evaluates the body once with the binder bound
    /// to it.
    ///
    /// Refuses as [`Self::evaluate_clause`] does for an unknown clause or
    /// mismatched observations.
    fn check_separation(
        &self,
        clause: &QualifiedName,
        observations: &qsl_semantics::model::observation::AdmittedObservations,
        claim: WitnessClaim<'_>,
        meter: &mut Meter,
    ) -> Result<Separation, CallFailure>;

    /// FR-115: check `invocation` (admitted by FR-106's checks 1 and 3 to
    /// 10) against the operation frame whose frame node identity is
    /// `frame`, through the `ProtocolClause` S6a family's `evaluate` hook.
    ///
    /// Refuses `InputRefusal::ObservationsMismatch` when `invocation` was
    /// admitted for a different frame, without charging `meter`. A frame
    /// identity this package does not resolve is `Err(CallFailure::Fault)`
    /// naming S6a, as for every S6a key.
    fn evaluate_frame(
        &self,
        frame: NodeKey,
        invocation: &qsl_semantics::model::observation::AdmittedInvocation<'_>,
        meter: &mut Meter,
    ) -> Result<FrameEvaluation, CallFailure>;
}

/// What evaluating one state clause over its admitted observations reads
/// (FR-107): the resolved declaration, the clause's own and pre object
/// environments, the `self`/`result`/parameter bindings in slot order, and
/// a fresh trail naming the observations (FR-265).
struct ClauseSetup<'a> {
    declaration: &'a qsl_semantics::check::CheckedStateClause,
    current: &'a ObjectEnvironment,
    pre: Option<&'a ObjectEnvironment>,
    bindings: Vec<Value>,
    trail: s6a::separation::Trail,
}

impl<'a> ClauseSetup<'a> {
    fn of(
        package: &'a CheckedPackage,
        clause: &QualifiedName,
        observations: &'a qsl_semantics::model::observation::AdmittedObservations,
    ) -> Result<Self, CallFailure> {
        let name = clause
            .as_unqualified()
            .ok_or_else(|| InputRefusal::UnknownClause(clause.to_string()))?;
        let declaration = package
            .graph()
            .state_clause(name)
            .ok_or_else(|| InputRefusal::UnknownClause(clause.to_string()))?;
        if declaration.identity() != observations.clause {
            return Err(InputRefusal::ObservationsMismatch.into());
        }
        // The clause's own observation (FR-107): `current` for an
        // invariant, `post` for a postcondition. A precondition's own
        // observation is `pre`, and it reads every model read there
        // (FR-104): an invocation also carries its post snapshot, which the
        // clause never reads apart from `pre`; a pre-call observation
        // carries the pre snapshot alone.
        let current = match declaration.observation() {
            qsl_semantics::check::Observation::Current => observations.current.as_ref(),
            qsl_semantics::check::Observation::Post => observations.post.as_ref(),
            qsl_semantics::check::Observation::Pre => {
                observations.post.as_ref().or(observations.pre.as_ref())
            }
        }
        .ok_or_else(|| {
            CallFailure::Fault(InternalFault::new(
                "S6a",
                "clause-observations-missing-the-clause-observation",
            ))
        })?;

        let mut bindings = Vec::with_capacity(declaration.parameters().len());
        bindings.push(Value::Reference(observations.self_object.clone()));
        let mut remaining = declaration.parameters().get(1..).unwrap_or(&[]).iter();
        // FR-104 "Behavior": slot 1 is `result` exactly when the checker's
        // own typed `binds_result` says so (FR-064's string-edge
        // rule) -- never decided here by comparing a parameter's name.
        if declaration.binds_result() {
            remaining.next();
            let result = observations.result.clone().ok_or_else(|| {
                CallFailure::Fault(InternalFault::new("S6a", "postcondition-result-missing"))
            })?;
            bindings.push(result);
        }
        for (parameter, _) in remaining {
            let value = observations
                .parameters
                .iter()
                .find(|(name, _)| name == parameter)
                .map(|(_, value)| value.clone())
                .ok_or_else(|| {
                    CallFailure::Fault(InternalFault::new("S6a", "clause-parameter-not-admitted"))
                })?;
            bindings.push(value);
        }
        let pre = observations
            .pre
            .as_ref()
            .map(|observation| s6a::separation::observation_identity(&observation.identity));
        // A precondition reads the pre snapshot, so its value paths name it
        // whether the run is pre-call or over an invocation (FR-104).
        let own = match (declaration.observation(), &pre) {
            (qsl_semantics::check::Observation::Pre, Some(pre)) => pre.clone(),
            _ => s6a::separation::observation_identity(&current.identity),
        };
        let trail = s6a::separation::Trail::new(own, pre);
        Ok(Self {
            declaration,
            current: &current.environment,
            pre: observations
                .pre
                .as_ref()
                .map(|observation| &observation.environment),
            bindings,
            trail,
        })
    }
}

/// FR-268 step 1 (ADR-031 SW-13): resolve `quantifier` in `graph` to a
/// `forall` or `exists` node of `clause`'s claim, reached from the claim's
/// root through `let` bodies, `if` branches, `not` and the operands of
/// `and`, `or` and `implies` -- the nodes a decision path passes (SW-2), so
/// no binder of an enclosing quantifier is in scope (SW-4). Returns the
/// `let` nodes on the path, outermost first, and the quantifier node.
/// `None` when the occurrence names no node of the package, a node of
/// another declaration, or a node not reached that way.
fn resolve_quantifier<'g>(
    graph: &'g qsl_semantics::check::CheckedGraph,
    clause: &'g qsl_semantics::check::CheckedStateClause,
    quantifier: &qsl_foundation::source::provenance::OccurrenceKey,
) -> Option<(
    Vec<&'g qsl_semantics::check::Node>,
    &'g qsl_semantics::check::Node,
)> {
    use qsl_semantics::check::{NodeKind, Visit};
    let key = graph.semantic_graph().resolve_wire(quantifier.node())?;
    let location = graph.occurrence(key, quantifier.origin())?;
    let applies_quantify = matches!(
        graph.semantic_graph().node(key).map(|node| node.body()),
        Some(qsl_semantics::check::SemanticTerm::Application {
            operator: qsl_semantics::check::Operator::Quantify,
            ..
        })
    );
    if !applies_quantify {
        return None;
    }
    // Depth-first over the decision-path node kinds, each pending node with
    // the `let` nodes above it.
    let mut pending = vec![(clause.body(), Vec::new())];
    while let Some((node, lets)) = pending.pop() {
        match node.kind() {
            NodeKind::Query {
                visit: Visit::Forall | Visit::Exists,
                ..
            } if node.location() == location => return Some((lets, node)),
            NodeKind::Let { body, .. } => {
                let mut inner = lets;
                inner.push(node);
                pending.push((body, inner));
            }
            NodeKind::If {
                then, otherwise, ..
            } => {
                pending.push((then, lets.clone()));
                pending.push((otherwise, lets));
            }
            NodeKind::Not(operand) => pending.push((operand, lets)),
            NodeKind::Connective(_, left, right) => {
                pending.push((left, lets.clone()));
                pending.push((right, lets));
            }
            // Every other node ends a decision path (ADR-031 SW-2).
            _ => {}
        }
    }
    None
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
        // (`s6a::ReferenceEvaluation`) is the one path that runs
        // checked function-application code, not a second, parallel
        // `Machine` call beside it. It charges this call's own
        // `function.call` to the caller's `meter`, then runs the body
        // against that same meter, so `meter` counts the whole
        // call. A denied charge surfaces as
        // `Ok(FamilyOutcome::Evaluated(Outcome::Incomplete(_)))`, the same
        // shape a denied charge inside the body takes.
        let identity = callable.identity;
        let mut env = family::EvaluationEnv::new(self, objects, arguments);
        evaluate_declaration(
            S6aFamilyKind::Value,
            &identity,
            EvaluationTarget::Value(&mut env),
            meter,
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
        Machine::new(
            self.graph().scope(),
            self.graph(),
            objects,
            meter,
            self.graph().dispatch_tables(),
        )
        .run(expression.root(), expression.slots(), arguments)
        .map_err(CallFailure::Fault)
    }

    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &qsl_semantics::model::observation::AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        let setup = ClauseSetup::of(self, clause, observations)?;
        let identity = setup.declaration.identity();
        let mut env = s6a::protocol_clause::ProtocolClauseEnv::new(
            self.graph(),
            setup.current,
            setup.pre,
            setup.bindings,
            setup.trail,
        );
        let evaluation = evaluate_declaration(
            S6aFamilyKind::ProtocolClause,
            &identity,
            EvaluationTarget::ProtocolClause(&mut env),
            meter,
        )
        .map_err(CallFailure::Fault)?;
        let trail = env.trail.take().ok_or_else(|| {
            CallFailure::Fault(InternalFault::new(
                "S6a",
                "clause-evaluation-environment-carries-a-trail",
            ))
        })?;
        Ok(ClauseEvaluation::new(evaluation, trail))
    }

    fn check_separation(
        &self,
        clause: &QualifiedName,
        observations: &qsl_semantics::model::observation::AdmittedObservations,
        claim: WitnessClaim<'_>,
        meter: &mut Meter,
    ) -> Result<Separation, CallFailure> {
        let setup = ClauseSetup::of(self, clause, observations)?;
        let graph = self.graph();
        let declaration = setup.declaration;
        let Some((lets, quantifier)) = resolve_quantifier(graph, declaration, claim.quantifier)
        else {
            return Ok(Separation::Unmet(SeparationStep::Quantifier));
        };
        let reads: std::collections::BTreeMap<
            quire_semantic_value::location::Location,
            qsl_semantics::check::Observation,
        > = declaration
            .reads()
            .map(|(location, observation)| (location.clone(), observation))
            .collect();
        Machine::with_pre(
            graph.scope(),
            graph,
            setup.current,
            setup.pre,
            Some(&reads),
            meter,
            graph.dispatch_tables(),
        )
        .with_trail(setup.trail)
        .separate(
            declaration.slots(),
            setup.bindings,
            &lets,
            quantifier,
            claim,
        )
        .map_err(CallFailure::Fault)
    }

    fn evaluate_frame(
        &self,
        frame: NodeKey,
        invocation: &qsl_semantics::model::observation::AdmittedInvocation<'_>,
        meter: &mut Meter,
    ) -> Result<FrameEvaluation, CallFailure> {
        if invocation.frame != frame {
            return Err(InputRefusal::ObservationsMismatch.into());
        }
        let mut env = s6a::protocol_clause::ProtocolClauseEnv::frame(self.graph(), invocation);
        let evaluation = evaluate_declaration(
            S6aFamilyKind::ProtocolClause,
            &frame,
            EvaluationTarget::ProtocolClause(&mut env),
            meter,
        )
        .map_err(CallFailure::Fault)?;
        Ok(FrameEvaluation {
            evaluation,
            witness: env.witness.take(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_forms::{Expression, FunctionDeclaration, TypeForm};
    use qsl_foundation::diagnostic::Category;
    use qsl_semantics::check::{PackageDeclarations, SCALAR_LIMITS_UNLIMITED};
    use qsl_semantics::family::{EvalOutcome, FamilyOutcome};
    use quire_exact::{Integer, NodeKey};
    use quire_semantic_value::checking::CheckingLimits;

    /// FR-285-AC-3 (TC-769 step 3): `CallFailure::Input` exits 20 and
    /// `CallFailure::Fault` 30.
    #[trace("TC-769", "FR-285-AC-3")]
    #[test]
    fn tc_769_call_failures_exit_by_their_category() {
        let input = CallFailure::Input(InputRefusal::WrongValueKind { parameter: 0 });
        assert_eq!(input.category().exit_code(), 20);
        let fault = CallFailure::Fault(qsl_foundation::diagnostic::InternalFault::new("S6a", "x"));
        assert_eq!(fault.category().exit_code(), 30);
    }

    /// Each admission refusal maps to its catalog code: a missing name to
    /// `missing_declaration`, a wrong argument or role mapping to
    /// `invalid_runtime_input`, a dangling reference to `dangling_reference`.
    /// The enum's spelling equals the leaf's own `InputRefusal::code` string
    /// for every variant, so a backend reading the string agrees with QSL.
    #[test]
    fn input_refusal_code_maps_each_refusal_to_its_catalog_code() {
        use qsl_foundation::diagnostic::Code;
        let cases = [
            (
                InputRefusal::UnknownFunction("f".to_owned()),
                Code::MissingDeclaration,
            ),
            (
                InputRefusal::UnknownClause("c".to_owned()),
                Code::MissingDeclaration,
            ),
            (
                InputRefusal::Arity {
                    declared: 1,
                    supplied: 2,
                },
                Code::InvalidRuntimeInput,
            ),
            (
                InputRefusal::WrongValueKind { parameter: 0 },
                Code::InvalidRuntimeInput,
            ),
            (
                InputRefusal::DanglingReference { parameter: 0 },
                Code::DanglingReference,
            ),
            (
                InputRefusal::ObservationsMismatch,
                Code::InvalidRuntimeInput,
            ),
        ];
        for (refusal, code) in cases {
            assert_eq!(input_refusal_code(&refusal), code, "{refusal:?}");
            assert_eq!(code.as_str(), refusal.code(), "{refusal:?}");
        }
    }

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
            Expression::name("x".to_owned()),
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
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("id(x: Integer[0,10]): Integer[0,10] = x checks cleanly");
        let identity = graph
            .function_identity("id")
            .expect("id is declared in this package");
        let package = qsl_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::default();

        // Steps 2-3: one evaluation environment, carrying the arguments
        // [3], consumed by a first, real S6a call.
        let mut env = family::EvaluationEnv::new(
            &package,
            &objects,
            vec![Value::Integer(Integer::from(3_i64))],
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let first =
            qsl_semantics::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut meter)
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
            qsl_semantics::check::ValueFunctionFamily::evaluate(&identity, &mut env, &mut meter)
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
        let mut fresh_env = family::EvaluationEnv::new(&package, &objects, Vec::new());
        let mut fresh_meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let unknown_fault = qsl_semantics::check::ValueFunctionFamily::evaluate(
            &unknown_identity,
            &mut fresh_env,
            &mut fresh_meter,
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
            S6aFamilyKind::ProtocolClause => "ProtocolClause",
        }
    }

    /// TC-428 (FR-096-AC-15): `not x` over `x: Boolean`, called through the S6a
    /// seam with an Integer argument that admission would have refused, breaks
    /// the checked-program invariant. The kernel `CheckedInvariant` stop is an
    /// `Err(InternalFault)` naming `S6a`/`checked-program-invariant`, never an
    /// `Ok` evaluation carrying a refusal.
    #[trace("FR-096-AC-15", "TC-428")]
    #[test]
    fn checked_invariant_is_an_internal_fault_at_s6a() {
        let boolean = || {
            TypeForm::builtin(
                qsl_forms::BuiltinType::Boolean,
                qsl_foundation::Span { start: 0, end: 0 },
            )
        };
        let graph = PackageDeclarations {
            functions: vec![FunctionDeclaration::new(
                "flip",
                vec![("x".to_owned(), boolean())],
                boolean(),
                None,
                Expression::logical_not(Expression::name("x".to_owned())),
            )],
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("flip(x: Boolean): Boolean = not x checks cleanly");
        let flip = graph.function_identity("flip").expect("flip is declared");
        let package = qsl_package::CheckedPackage::link(graph);
        let objects = ObjectEnvironment::default();
        let mut env = family::EvaluationEnv::new(
            &package,
            &objects,
            vec![Value::Integer(Integer::from(3_i64))],
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let fault =
            qsl_semantics::check::ValueFunctionFamily::evaluate(&flip, &mut env, &mut meter)
                .expect_err("a checked-program invariant break is an internal fault");
        assert_eq!(fault.stage(), "S6a");
        assert_eq!(fault.category(), Category::InternalFailure);
        assert_eq!(fault.invariant(), "checked-program-invariant");
        assert_eq!(
            fault.catalog_code(),
            qsl_foundation::diagnostic::CatalogCode::new(
                "runtime_invariant",
                "established-invariant-broken"
            ),
            "FR-096-AC-15 names the code runtime_invariant"
        );
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
    /// `FamilyOutcome::Evaluated`; `qsl-eval/tests/it/model_reference_queries.rs`'s
    /// TC-385 test takes the `FamilyEvaluated` arm through the seam.
    ///
    /// Also backs FR-062-AC-6's first sentence: that sentence is
    /// FR-090-AC-4 verbatim ("The Relation family has no evaluation hook,
    /// and S6a's input type admits no Relation node..."), so the test that
    /// demonstrates one demonstrates the other.
    #[trace("FR-090-AC-4", "TC-385", "FR-062-AC-6", "TC-160")]
    #[test]
    fn s6a_family_kind_admits_no_relation_and_family_outcome_has_two_arms() {
        let empty = qsl_package::CheckedPackage::link(
            PackageDeclarations::new(qsl_semantics::check::fixture_source())
                .check(CheckingLimits::default())
                .expect("an empty package checks cleanly"),
        );
        let objects = ObjectEnvironment::default();
        let undeclared = NodeKey::from_digest([0xAB; 32]);
        for kind in S6aFamilyKind::ALL {
            let name = s6a_family_name(kind);
            let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
            let fault = match kind {
                S6aFamilyKind::Value => {
                    let mut env = family::EvaluationEnv::new(&empty, &objects, Vec::new());
                    evaluate_declaration(
                        kind,
                        &undeclared,
                        EvaluationTarget::Value(&mut env),
                        &mut meter,
                    )
                }
                S6aFamilyKind::ProtocolClause => {
                    let mut env = s6a::protocol_clause::ProtocolClauseEnv::new(
                        empty.graph(),
                        &objects,
                        None,
                        Vec::new(),
                        s6a::separation::Trail::new(
                            ObservationIdentity {
                                authority: String::new(),
                                identity: String::new(),
                                revision_namespace: String::new(),
                                revision: String::new(),
                            },
                            None,
                        ),
                    );
                    evaluate_declaration(
                        kind,
                        &undeclared,
                        EvaluationTarget::ProtocolClause(&mut env),
                        &mut meter,
                    )
                }
            }
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
            ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
        }
        .check(CheckingLimits::default())
        .expect("id(x: Integer[0,10]): Integer[0,10] = x checks cleanly");
        let identity = graph
            .function_identity("id")
            .expect("id is declared in this package");
        let package = qsl_package::CheckedPackage::link(graph);
        let mut env = family::EvaluationEnv::new(
            &package,
            &objects,
            vec![Value::Integer(Integer::from(3_i64))],
        );
        let mut meter = Meter::new(SCALAR_LIMITS_UNLIMITED);
        let evaluation = evaluate_declaration(
            S6aFamilyKind::Value,
            &identity,
            EvaluationTarget::Value(&mut env),
            &mut meter,
        )
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
}

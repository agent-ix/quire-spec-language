// SPDX-License-Identifier: AGPL-3.0-or-later
//! FSpec FR-650 and FR-805: the plan search behind the native-run
//! `synthesis` member, for a declaration whose grammar is a model object
//! type.
//!
//! The member is read by [`PlanRequest`], the model and the initial snapshot
//! are admitted into a [`ModelSystem`], the declaration's checked goal and
//! objective judge the plan through [`SourcePlan`], and
//! [`fsl_analyze::plan_search::search`] settles the result, which
//! [`plan_request::plan_outcome`] writes as the `filament.plan-outcome/v1`
//! document.

use std::time::Duration;

use filament_exact::{Cancel, Integer, ValueType};
use fsl_analyze::analyze::document::{DocumentSelection, SubjectSelection};
use fsl_analyze::plan_search::{
    Direction, PlanLengths, PlanResult, PlanSearchError, PlanSearchRequest, search,
};
use fsl_analyze::plan_source::SourcePlan;
use fsl_eval::simulation::model::{
    ExpansionLimits, ModelCheckInput, ModelSimState, ModelSimulationRequest, ModelSystem,
    declaration_identity_text, typed_value_json,
};
use fsl_foundation::Code;
use fsl_package::CheckedPackage;
use fsl_semantics::check::CheckedSynthesis;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::observation::{DocumentProvision, ObservationLimits};
use serde_json::{Map, Value as Json, json};

use super::plan_request::{
    self, NATIVE_RUN_POINTER, OutcomeHeader, PlanObjective, PlanRequest, PlanRequestRefusal,
    RenderedPlan, RenderedStep, ValueForm,
};
use super::{Intake, RunCause};

/// What a plan run reads of the request and the checked package.
pub(super) struct PlanRun<'r, 'i> {
    /// The checked package identity the outcome names.
    pub package_name: &'r str,
    /// The authored name of the selected declaration.
    pub declaration: &'r str,
    /// The checked package the declaration belongs to.
    pub package: &'r CheckedPackage,
    /// The selected declaration.
    pub checked: &'r CheckedSynthesis,
    /// The `synthesis` member as the caller sent it.
    pub member: &'r Json,
    /// The domain packages of the request's models, by identity.
    pub packages: &'r fsl_semantics::model::intake::PackageInput,
    /// The request's files.
    pub intake: &'r mut Intake<'i>,
}

fn refusal(pointer: &str, cause: &'static str, reason: impl Into<String>) -> RunCause {
    RunCause::SynthesisRequest {
        path: pointer.to_owned(),
        code: Code::InvalidRuntimeInput,
        cause,
        reason: reason.into(),
    }
}

fn request_refusal(refusal: PlanRequestRefusal) -> RunCause {
    RunCause::SynthesisRequest {
        reason: refusal.to_string(),
        path: refusal.pointer,
        code: Code::InvalidRuntimeInput,
        cause: refusal.cause.name(),
    }
}

/// The plan lengths of the declaration's domain: a bounded range of
/// non-negative integers, which S3 has checked.
fn lengths(checked: &CheckedSynthesis) -> Result<PlanLengths, RunCause> {
    let ValueType::Int(domain) = checked.domain() else {
        return Err(refusal(
            "/request/synthesis/declaration",
            "invalid-value",
            "the plan lengths are not a finite integer range",
        ));
    };
    let bound = |value: &Integer| {
        value.to_string().parse::<u64>().map_err(|_| {
            refusal(
                "/request/synthesis/declaration",
                "invalid-value",
                "the plan lengths exceed 64 bits",
            )
        })
    };
    PlanLengths::new(bound(domain.lower())?, bound(domain.upper())?).ok_or_else(|| {
        refusal(
            "/request/synthesis/declaration",
            "invalid-value",
            "the plan lengths are empty",
        )
    })
}

/// The objective as the outcome names it.
fn objective(checked: &CheckedSynthesis) -> PlanObjective {
    let (direction, expression, value_type) = match checked.objective() {
        None => (Direction::Minimize, "length", None),
        Some(body) => (
            match checked.objective_direction() {
                Some("maximize") => Direction::Maximize,
                _ => Direction::Minimize,
            },
            checked.objective_source().unwrap_or("length"),
            Some(body.root().value_type().clone()),
        ),
    };
    let form = match &value_type {
        Some(ValueType::Decimal(decimal)) => ValueForm::Decimal {
            scale: decimal.max_scale(),
        },
        Some(ValueType::Rational(_)) => ValueForm::Rational,
        _ => ValueForm::Integer,
    };
    PlanObjective {
        direction,
        expression: expression.to_owned(),
        form,
    }
}

/// Run `plan` over the plan synthesis.
pub(super) fn run(
    plan: PlanRun<'_, '_>,
    accounting: filament_exact::ScalarLimits,
    started: std::time::Instant,
) -> super::Result<(u8, Json)> {
    let PlanRun {
        package_name,
        declaration,
        package,
        checked,
        member,
        packages,
        intake,
    } = plan;
    let request = PlanRequest::read(member, NATIVE_RUN_POINTER).map_err(request_refusal)?;
    request
        .check_grammar(false, NATIVE_RUN_POINTER)
        .map_err(request_refusal)?;
    let lengths = lengths(checked)?;
    let Some(initial) = request.initial.as_deref() else {
        return Err(refusal(
            &format!("{NATIVE_RUN_POINTER}/initial"),
            "missing-member",
            "a plan over a state model needs its initial snapshot",
        ));
    };

    // The initial snapshot is the plan's initial state and its universe.
    let bytes = intake.file(initial, intake.remaining)?;
    let read = fsl_replay::spine::SuppliedObservation::from_document("synthesis.initial", bytes)
        .map_err(|error| {
            refusal(
                &format!("{NATIVE_RUN_POINTER}/initial"),
                "invalid-value",
                error.reason,
            )
        })?;
    let selection = DocumentSelection {
        authority: read.reference.authority.clone(),
        identity: read.reference.identity.clone(),
    };
    let mut documents = DocumentProvision::default();
    documents.insert(read.reference, read.bytes);
    let subject = SubjectSelection {
        initial_states: vec![selection.clone()],
        universes: vec![selection],
        ..SubjectSelection::default()
    };
    let binding = subject.binding(packages, &documents);
    let limits = scalar_limits(&request, accounting);
    let system = ModelSystem::new(ModelSimulationRequest {
        input: ModelCheckInput::source(package),
        packages: binding.packages,
        normalization: ModelNormalizationLimits::default(),
        snapshots: binding.snapshots,
        initial: binding.initial.clone(),
        universes: binding.universes.clone(),
        observation: ObservationLimits::default(),
        expansion: ExpansionLimits {
            max_candidates: u64::MAX,
        },
        meter: accounting,
        emit: fsl_package::EmitLimits::new(u64::MAX),
    })
    .map_err(|failure| {
        refusal(
            &format!("{NATIVE_RUN_POINTER}/initial"),
            "invalid-value",
            format!("the initial snapshot is not a state of the model: {failure:?}"),
        )
    })?;

    let mut policies = Vec::with_capacity(request.policy.len());
    for (position, name) in request.policy.iter().enumerate() {
        let node = package
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.clause().name() == name)
            .ok_or_else(|| RunCause::SynthesisRequest {
                path: format!("{NATIVE_RUN_POINTER}/policy/{position}"),
                code: Code::MissingDeclaration,
                cause: "missing-name",
                reason: format!("no temporal clause is named `{name}`"),
            })?;
        if node.clause().over().1 != *checked.grammar() {
            return Err(RunCause::SynthesisRequest {
                path: format!("{NATIVE_RUN_POINTER}/policy/{position}"),
                code: Code::IllTyped,
                cause: "type-mismatch",
                reason: format!("the clause `{name}` is not over the grammar type of the plan"),
            });
        }
        policies.push(node);
    }
    let mut rules =
        SourcePlan::new(package, checked, &system, policies, limits).map_err(|error| {
            refusal(
                "/request/synthesis/declaration",
                "invalid-value",
                error.to_string(),
            )
        })?;
    let cancel = Cancel::new();
    let deadline = request
        .deadline_ms
        .map(|milliseconds| Duration::from_millis(milliseconds).saturating_sub(started.elapsed()));
    let result = with_deadline(&cancel, deadline, || {
        search(
            &system,
            &mut rules,
            PlanSearchRequest {
                lengths,
                limits: request.limits,
                cancel: &cancel,
                fault: None,
            },
        )
    });
    if let Some(fault) = rules.fault() {
        return Err(RunCause::SynthesisRequest {
            path: "/request/synthesis".to_owned(),
            code: Code::RuntimeInvariant,
            cause: "evaluation-fault",
            reason: format!("a checked plan expression faulted: {fault}"),
        });
    }
    let result = result.map_err(plan_failure)?;

    let objective = objective(checked);
    let rendered = render(package, &system, &rules, &result);
    let header = OutcomeHeader {
        package: package_name,
        declaration,
        objective: &objective,
        limits: &request.limits,
    };
    let document = plan_request::plan_outcome(header, &result, rendered.as_ref());
    Ok((plan_request::exit_code(result.status), document))
}

/// The evaluation meter's limits: the plan request's `accounting` over the
/// defaults.
fn scalar_limits(
    request: &PlanRequest,
    defaults: filament_exact::ScalarLimits,
) -> filament_exact::ScalarLimits {
    let mut limits = fsl_replay::AccountingLimits(defaults);
    for (setting, bound) in &request.accounting {
        limits.set_bound(*setting, bound.get());
    }
    limits.0
}

/// A plan search that could not produce a result: a replay that fails is a
/// method failure (FSpec FR-650), everything else an invariant fault.
fn plan_failure(error: PlanSearchError) -> RunCause {
    RunCause::SynthesisRequest {
        path: "/request/synthesis".to_owned(),
        code: Code::RuntimeInvariant,
        cause: match error {
            PlanSearchError::ReplayFailed { .. } | PlanSearchError::ReplayRejected(_) => {
                "replay-failed"
            }
            _ => "search-failed",
        },
        reason: error.to_string(),
    }
}

/// Run `work` with `cancel` cancelled for its deadline once `remaining`
/// elapses.
fn with_deadline<T>(cancel: &Cancel, remaining: Option<Duration>, work: impl FnOnce() -> T) -> T {
    match remaining {
        None => work(),
        Some(wait) if wait.is_zero() => {
            cancel.cancel(filament_exact::CancelCause::Deadline);
            work()
        }
        Some(wait) => std::thread::scope(|scope| {
            let (done, finished) = std::sync::mpsc::channel::<()>();
            let timer = cancel.clone();
            scope.spawn(move || {
                if let Err(std::sync::mpsc::RecvTimeoutError::Timeout) = finished.recv_timeout(wait)
                {
                    timer.cancel(filament_exact::CancelCause::Deadline);
                }
            });
            let value = work();
            let _ = done.send(());
            value
        }),
    }
}

/// The reported plan as the outcome document writes it.
fn render(
    package: &CheckedPackage,
    system: &ModelSystem<'_>,
    rules: &SourcePlan<'_>,
    result: &PlanResult,
) -> Option<RenderedPlan> {
    let reported = result.plan.as_ref()?;
    let scope = package.graph().scope();
    let steps = reported
        .steps
        .iter()
        .map(|step| {
            let operation = rules.operation(step);
            let arguments = operation
                .map(|operation| {
                    operation
                        .parameters
                        .iter()
                        .zip(step.arguments())
                        .map(|((name, _), text)| {
                            let value = serde_json::from_str(text).unwrap_or(Json::Null);
                            (name.clone(), value)
                        })
                        .collect()
                })
                .unwrap_or_default();
            RenderedStep {
                operation: rules.operation_name(step).to_owned(),
                arguments,
                delay: None,
            }
        })
        .collect();
    let states = reported
        .states
        .iter()
        .map(|state| state_document(system, scope, state))
        .collect::<Option<Vec<_>>>()?;
    Some(RenderedPlan { steps, states })
}

/// A state as a `filament.state.snapshot/v1` object world, its values in
/// FSpec FR-181's typed form.
fn state_document(
    system: &ModelSystem<'_>,
    scope: &fsl_semantics::check::Scope,
    state: &ModelSimState,
) -> Option<Json> {
    let mut populations = Vec::new();
    for (population, objects) in &state.state().populations {
        let mut listed = Vec::new();
        for (key, object) in objects {
            let mut fields = Map::new();
            for field in system.fields_of(&object.type_identity)? {
                if let Some(value) = object.fields.get(&field.key) {
                    fields.insert(
                        field.name.clone(),
                        typed_value_json(value, scope.types(), scope.variant_types())?,
                    );
                }
            }
            listed.push(json!({
                "key": key,
                "type": declaration_identity_text(&object.type_identity)?,
                "fields": fields,
            }));
        }
        populations.push(json!({
            "population": declaration_identity_text(population)?,
            "complete": true,
            "objects": listed,
        }));
    }
    Some(json!({
        "format": "filament.state.snapshot/v1",
        "populations": populations,
    }))
}

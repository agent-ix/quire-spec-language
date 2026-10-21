// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330: the `Temporal` selection of [`super::run_clause`]. It resolves one
//! temporal clause by name, admits each observation of the supplied trace
//! (FR-106), binds the clause's `over` parameter at position 0 and evaluates
//! the clause once through the TemporalTrace evaluator (FR-327, FR-328,
//! FR-329).

use std::collections::BTreeMap;

use qsl_eval::temporal::{
    evaluate_observed, HoldsStop, PositionRefusal, TemporalOutcome, TemporalRefusal, TemporalTrace,
    UndefinedCause, UndefinedEvaluation, WindowRefusal,
};
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::TemporalProfile;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    admit_trace_snapshot, population_universe_for, AdmissionFailure, AdmissionRecord,
    AdmissionUsage, DocumentRef,
};
use quire_exact::{Meter, ValueType};

use super::{
    convert_call_failure, convert_outcome, ClauseDisposition, ClauseRunReport, ClauseRunUsage,
    CompiledRun, RunRefusal, TemporalPositionRefusal, TemporalTraceRefusal, TemporalUndefinedCause,
    TemporalVerdict, TemporalWindowRefusal,
};
use crate::proof_result::SettlementBasis;

/// FR-330 Inputs: the observations a temporal clause is run over, each a
/// snapshot named by digest (FR-106).
#[derive(Clone, Debug)]
pub enum TraceInput {
    /// A finite trace: a closed trace under a bounded profile, a finite
    /// prefix under the infinite-trace profile.
    Finite(Vec<DocumentRef>),
    /// A lasso: `prefix`, then `repeat` forever. Its last position re-enters
    /// the first position of `repeat`.
    Lasso {
        /// The positions before the loop.
        prefix: Vec<DocumentRef>,
        /// The positions of the loop; the loop is empty only in a malformed
        /// lasso, which the evaluator refuses.
        repeat: Vec<DocumentRef>,
    },
}

impl TraceInput {
    /// Every observation, in position order.
    fn documents(&self) -> impl Iterator<Item = &DocumentRef> {
        let (prefix, repeat): (&[DocumentRef], &[DocumentRef]) = match self {
            Self::Finite(positions) => (positions, &[]),
            Self::Lasso { prefix, repeat } => (prefix, repeat),
        };
        prefix.iter().chain(repeat)
    }

    /// The trace's shape, or `None` when its length exceeds `u64`.
    fn shape(&self) -> Option<TemporalTrace> {
        match self {
            Self::Finite(positions) => Some(TemporalTrace::Finite {
                len: u64::try_from(positions.len()).ok()?,
            }),
            Self::Lasso { prefix, repeat } => Some(TemporalTrace::Lasso {
                prefix: u64::try_from(prefix.len()).ok()?,
                loop_len: u64::try_from(repeat.len()).ok()?,
            }),
        }
    }
}

/// FR-330 Inputs: the `{population, key}` object the clause's `over`
/// parameter names, resolved in the observation at position 0.
#[derive(Clone, Debug)]
pub struct OverObject {
    /// The population identity.
    pub population: String,
    /// The object's declared key.
    pub key: String,
}

fn refused(cause: &'static str, fields: BTreeMap<&'static str, String>) -> ClauseDisposition {
    ClauseDisposition::Admit(AdmissionFailure::Refused(AdmissionRecord {
        code: Code::InvalidRuntimeInput,
        cause,
        fields,
        path: Vec::new(),
        kernel: None,
    }))
}

/// FR-330: run the temporal clause `name` over `trace`.
pub(super) fn run_temporal(
    run: &CompiledRun<'_>,
    name: &str,
    trace: &TraceInput,
    over: &OverObject,
) -> ClauseRunReport {
    // A trace is evidence for that trace only: it settles no item and
    // derives no witness (FR-332).
    let report = |disposition, usage: ClauseRunUsage| ClauseRunReport {
        basis: SettlementBasis::Unavailable,
        witness: None,
        package_id: Some(run.package_id),
        disposition,
        usage,
    };
    let early = |disposition| report(disposition, ClauseRunUsage::default());

    let graph = run.package.graph();
    let Some(node) = graph
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == name)
    else {
        return early(ClauseDisposition::MissingName {
            name: name.to_owned(),
        });
    };
    let clause = node.clause();

    // A bounded profile reads a closed finite trace.
    if matches!(trace, TraceInput::Lasso { .. })
        && clause.profile() != TemporalProfile::InfiniteTrace
    {
        return early(refused("invalid-value", BTreeMap::new()));
    }
    let Some(shape) = trace.shape() else {
        return early(refused("invalid-value", BTreeMap::new()));
    };

    // A trace with no position has no observation to evaluate over.
    if trace.documents().next().is_none() {
        return early(ClauseDisposition::TemporalRefusal(match shape {
            TemporalTrace::Finite { .. } => TemporalTraceRefusal::EmptyTrace,
            TemporalTrace::Lasso { .. } => TemporalTraceRefusal::EmptyLoop,
        }));
    }

    let mut admitted = AdmissionUsage::default();
    let mut environments: Vec<ObjectEnvironment> = Vec::new();
    let named = node.named_populations();
    for document in trace.documents() {
        match admit_trace_snapshot(
            graph.model_selections(),
            graph.scope().types(),
            run.packages,
            run.model_limits,
            run.provisions.snapshots,
            document,
            run.observation_limits,
            &named,
        ) {
            Ok((environment, usage)) => {
                admitted.document_bytes =
                    admitted.document_bytes.saturating_add(usage.document_bytes);
                admitted.objects = admitted.objects.saturating_add(usage.objects);
                admitted.values = admitted.values.saturating_add(usage.values);
                environments.push(environment);
            }
            Err(failure) => {
                return report(
                    ClauseDisposition::Admit(failure),
                    ClauseRunUsage {
                        admission_consumed: admitted,
                        ..ClauseRunUsage::default()
                    },
                )
            }
        }
    }

    // From here on admission has run: every report keeps what it consumed.
    let late = |disposition| {
        report(
            disposition,
            ClauseRunUsage {
                admission_consumed: admitted,
                ..ClauseRunUsage::default()
            },
        )
    };

    // `over` resolves in the observation at position 0 (FR-330 Inputs).
    let (_, over_type) = clause.over();
    let ValueType::Reference(type_identity) = over_type else {
        return late(ClauseDisposition::EvaluateFault(
            qsl_foundation::diagnostic::InternalFault::new("temporal", "over-is-a-reference"),
        ));
    };
    let universe = match population_universe_for(
        graph.model_selections(),
        run.packages,
        run.model_limits,
        *type_identity,
    ) {
        Ok(universe) => universe,
        Err(failure) => return late(ClauseDisposition::Admit(failure)),
    };
    let resolved = environments
        .first()
        .and_then(|environment| environment.objects().find(universe, &over.key));
    let Some(reference) = resolved else {
        return late(refused(
            "wrong-role-mapping",
            BTreeMap::from([
                ("population", over.population.clone()),
                ("object", over.key.clone()),
            ]),
        ));
    };

    let mut meter = Meter::new(run.accounting);
    let observed = evaluate_observed(
        run.package,
        node,
        &environments,
        shape,
        reference,
        &mut meter,
    );
    let disposition = match (observed.stopped, observed.result) {
        // A condition's refusal is reported as FR-109 reports a state
        // clause's (`convert_outcome`); a failed call as `check_clause` does.
        (Some(HoldsStop::Refused(evaluation)), _) => {
            match convert_outcome(evaluation, run.package.graph(), run.sources) {
                Ok(outcome) => ClauseDisposition::Evaluate(outcome),
                Err(refusal) => match *refusal {
                    RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                    other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
                },
            }
        }
        (Some(HoldsStop::Failed(failure)), _) => match *convert_call_failure(failure) {
            RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
            other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
        },
        (None, Ok(outcome)) => ClauseDisposition::Temporal(verdict(outcome)),
        (None, Err(TemporalRefusal::Fault(fault))) => ClauseDisposition::EvaluateFault(fault),
        (None, Err(refusal)) => ClauseDisposition::TemporalRefusal(trace_refusal(refusal)),
    };
    report(disposition, ClauseRunUsage::from_meter(&meter, admitted))
}

/// The spine's own form of the evaluator's outcome (FR-100-AC-8).
fn verdict(outcome: TemporalOutcome) -> TemporalVerdict {
    match outcome {
        TemporalOutcome::Holds => TemporalVerdict::Holds,
        TemporalOutcome::Violated { at } => TemporalVerdict::Violated { at: at.index() },
        TemporalOutcome::Undefined(UndefinedEvaluation { at, cause }) => {
            TemporalVerdict::Undefined {
                at: at.index(),
                cause: match cause {
                    UndefinedCause::Kernel(undefined) => TemporalUndefinedCause::Kernel(undefined),
                    UndefinedCause::Family(record) => TemporalUndefinedCause::Family(record),
                    UndefinedCause::InsufficientData(data) => {
                        TemporalUndefinedCause::InsufficientData {
                            count: data.count,
                            min_count: data.min_count,
                        }
                    }
                },
            }
        }
        TemporalOutcome::Pending => TemporalVerdict::Pending,
        TemporalOutcome::MissingFairnessPremise { constraint } => {
            TemporalVerdict::MissingFairnessPremise { constraint }
        }
        TemporalOutcome::Incomplete(incomplete) => TemporalVerdict::Incomplete(incomplete),
        TemporalOutcome::WindowLimitReached(reached) => TemporalVerdict::WindowLimitReached {
            limit: reached.limit,
            value: reached.value,
            setting: reached.setting,
        },
    }
}

/// The spine's own form of the evaluator's trace refusal (FR-100-AC-8).
fn trace_refusal(refusal: TemporalRefusal) -> TemporalTraceRefusal {
    match refusal {
        TemporalRefusal::EmptyTrace => TemporalTraceRefusal::EmptyTrace,
        TemporalRefusal::TupleArity { expected, found } => {
            TemporalTraceRefusal::TupleArity { expected, found }
        }
        TemporalRefusal::AlignSkipNotLockstep => TemporalTraceRefusal::AlignSkipNotLockstep,
        TemporalRefusal::NotProjectionAligned => TemporalTraceRefusal::NotProjectionAligned,
        TemporalRefusal::EmptyLoop => TemporalTraceRefusal::EmptyLoop,
        TemporalRefusal::TraceTooLong => TemporalTraceRefusal::TraceTooLong,
        TemporalRefusal::UncountedLasso => TemporalTraceRefusal::UncountedLasso,
        TemporalRefusal::LassoUnderBoundedProfile { profile } => {
            TemporalTraceRefusal::LassoUnderBoundedProfile { profile }
        }
        TemporalRefusal::ProfileNotEvaluated { profile } => {
            TemporalTraceRefusal::ProfileNotEvaluated { profile }
        }
        TemporalRefusal::Position(position) => TemporalTraceRefusal::Position(match position {
            PositionRefusal::NotDecimal => TemporalPositionRefusal::NotDecimal,
            PositionRefusal::LeadingZero => TemporalPositionRefusal::LeadingZero,
            PositionRefusal::Overflow => TemporalPositionRefusal::Overflow,
            PositionRefusal::OutsideTrace { position } => TemporalPositionRefusal::OutsideTrace {
                position: position.index(),
            },
        }),
        TemporalRefusal::Window(window) => TemporalTraceRefusal::Window(match window {
            WindowRefusal::OutsideTrace { position } => {
                TemporalWindowRefusal::OutsideTrace { position }
            }
            WindowRefusal::WindowKindMismatch => TemporalWindowRefusal::WindowKindMismatch,
            WindowRefusal::TimestampsDecrease => TemporalWindowRefusal::TimestampsDecrease,
        }),
        TemporalRefusal::Fault(fault) => TemporalTraceRefusal::Fault(fault),
        // The timed evaluator's refusals: the spine reads a clause through
        // the step evaluator only, which never returns them.
        TemporalRefusal::NotTimedProfile { .. }
        | TemporalRefusal::NegativeStamp { .. }
        | TemporalRefusal::DecreasingStamp { .. }
        | TemporalRefusal::NonPositiveLoopDelay
        | TemporalRefusal::LoopReentersBeforeLastStamp => TemporalTraceRefusal::Fault(
            qsl_foundation::diagnostic::InternalFault::new("temporal", "step-evaluator-refusal"),
        ),
    }
}

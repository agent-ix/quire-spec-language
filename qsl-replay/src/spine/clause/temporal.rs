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
use quire_semantic_value::catalog::CatalogCode;

use super::progress::{
    ContradictedDimension, LedgerBinding, ObservedPosition, ProgressAssertion, ProgressClosure,
    ProgressCompleteness, ProgressContradiction, ProgressLedger, ProgressValue, Refusal,
};
use super::{
    convert_call_failure, convert_outcome, ClauseDisposition, ClauseRunReport, ClauseRunUsage,
    CompiledRun, ConsequentEventRefusal, ConsequentSourceEvents, RunRefusal,
    TemporalPositionRefusal, TemporalTraceRefusal, TemporalUndefinedCause, TemporalVerdict,
    TemporalWindowRefusal,
};
use crate::proof_result::SettlementBasis;

/// FR-330 Inputs: the observations a temporal clause is run over, each a
/// snapshot named by its labels (FR-106).
#[derive(Clone, Debug)]
pub enum TraceInput {
    /// A finite trace: a closed trace under a bounded profile, a finite
    /// prefix under the infinite-trace profile.
    Finite(Vec<DocumentRef>),
    /// A finite event-position or fixed-sample trace with progress (FR-504).
    Progressed {
        /// The admitted observations.
        trace: Vec<DocumentRef>,
        /// The progress assertion for this request.
        progress: ProgressAssertion,
    },
    /// A timestamped-event trace with progress (FR-501).
    Timestamped {
        /// The observations and their timestamp ordering data.
        observations: Vec<TimestampedObservation>,
        /// The progress assertion for this request.
        progress: ProgressAssertion,
    },
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
    /// Whether the trace carries a progress assertion a ledger can retain
    /// (FR-502 Inputs). A `Finite` trace and a `Lasso` carry none: a lasso is
    /// the whole infinite behavior and a finite trace is its own closure.
    fn carries_progress(&self) -> bool {
        match self {
            Self::Progressed { .. } | Self::Timestamped { .. } => true,
            Self::Finite(_) | Self::Lasso { .. } => false,
        }
    }

    /// The progress assertion and position identities carried by this trace.
    fn progress(&self) -> Option<(ProgressAssertion, Vec<ObservedPosition>)> {
        match self {
            Self::Progressed { trace, progress } => {
                let positions = trace
                    .iter()
                    .enumerate()
                    .map(|(index, document)| {
                        Some(ObservedPosition {
                            identity: document_identity(document),
                            stamp: u64::try_from(index).ok()?,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some((*progress, positions))
            }
            Self::Timestamped {
                observations,
                progress,
            } => Some((
                *progress,
                observations
                    .iter()
                    .map(|observation| ObservedPosition {
                        identity: document_identity(&observation.document),
                        stamp: observation.stamp,
                    })
                    .collect(),
            )),
            Self::Finite(_) | Self::Lasso { .. } => None,
        }
    }

    /// Every observation, in position order.
    fn documents(&self) -> Box<dyn Iterator<Item = &DocumentRef> + '_> {
        match self {
            Self::Finite(positions)
            | Self::Progressed {
                trace: positions, ..
            } => Box::new(positions.iter()),
            Self::Timestamped { observations, .. } => {
                Box::new(observations.iter().map(|observation| &observation.document))
            }
            Self::Lasso { prefix, repeat } => Box::new(prefix.iter().chain(repeat)),
        }
    }

    /// The trace's shape, or `None` when its length exceeds `u64`.
    fn shape(&self) -> Option<TemporalTrace> {
        match self {
            Self::Finite(positions)
            | Self::Progressed {
                trace: positions, ..
            } => Some(TemporalTrace::Finite {
                len: u64::try_from(positions.len()).ok()?,
            }),
            Self::Timestamped { observations, .. } => Some(TemporalTrace::Finite {
                len: u64::try_from(observations.len()).ok()?,
            }),
            Self::Lasso { prefix, repeat } => Some(TemporalTrace::Lasso {
                prefix: u64::try_from(prefix.len()).ok()?,
                loop_len: u64::try_from(repeat.len()).ok()?,
            }),
        }
    }
}

fn document_identity(document: &DocumentRef) -> String {
    format!(
        "{}:{}{}:{}{}:{}{}:{}",
        document.authority.len(),
        document.authority,
        document.identity.len(),
        document.identity,
        document.revision_namespace.len(),
        document.revision_namespace,
        document.revision.len(),
        document.revision,
    )
}

/// One timestamped observation in a progress-carrying trace.
#[derive(Clone, Debug)]
pub struct TimestampedObservation {
    /// The admitted snapshot identity.
    pub document: DocumentRef,
    /// The non-negative tick stamp.
    pub stamp: u64,
    /// An optional order key for equal-stamp positions.
    pub order_key: Option<String>,
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

/// A progress contradiction as the spine reports it (FR-330, FR-502 Outputs):
/// the typed contradiction and the `invalid_runtime_input`/`invalid-value`
/// record that carries `contradicted_dimension`, `clock`, `retained` and
/// `asserted` as fields.
#[derive(Clone, Debug)]
pub struct ProgressRefusal {
    /// The contradiction.
    pub contradiction: ProgressContradiction,
    /// Its record.
    pub record: AdmissionRecord,
}

/// One value of a contradiction as a record field.
fn progress_field(value: &ProgressValue) -> String {
    let closure = |closure: &ProgressClosure| match closure {
        ProgressClosure::Closed => "Closed",
        ProgressClosure::Open => "Open",
    };
    match value {
        ProgressValue::Watermark(watermark) => watermark.to_string(),
        ProgressValue::Completeness(ProgressCompleteness::Complete { through }) => {
            format!("Complete{{through: {through}}}")
        }
        ProgressValue::Completeness(ProgressCompleteness::Incomplete) => "Incomplete".to_owned(),
        ProgressValue::DecisionScope(value) => format!("scope {}", closure(value)),
        ProgressValue::SurroundingExecution(value) => format!("execution {}", closure(value)),
        ProgressValue::Position(position) => {
            format!("position {} stamped {}", position.identity, position.stamp)
        }
    }
}

impl From<ProgressContradiction> for ProgressRefusal {
    fn from(contradiction: ProgressContradiction) -> Self {
        let dimension = match contradiction.dimension {
            ContradictedDimension::Watermark => "Watermark",
            ContradictedDimension::Completeness => "Completeness",
            ContradictedDimension::Closure => "Closure",
        };
        let fields = BTreeMap::from([
            ("contradicted_dimension", dimension.to_owned()),
            ("clock", contradiction.clock.clone()),
            ("retained", progress_field(&contradiction.retained)),
            ("asserted", progress_field(&contradiction.asserted)),
        ]);
        let record = AdmissionRecord {
            catalog: CatalogCode::new(Code::InvalidRuntimeInput.as_str(), "invalid-value"),
            fields,
            path: Vec::new(),
            kernel: None,
        };
        Self {
            contradiction,
            record,
        }
    }
}

/// FR-502: admit one request's progress into its ledger. FR-501 and FR-504
/// call this once per accepted request, with the binding the request runs
/// under, the progress its trace asserts, the identities and stamps of its
/// positions and the request's `progress.covered_positions` setting.
///
/// # Errors
///
/// The disposition the request reports: `invalid_runtime_input`/
/// `invalid-value` carrying the contradiction, or an `Incomplete` verdict when
/// the covered positions outgrew the setting. The ledger is unchanged either
/// way.
pub fn admit_progress(
    ledger: &ProgressLedger,
    binding: &LedgerBinding,
    assertion: ProgressAssertion,
    positions: &[ObservedPosition],
    covered_positions: u64,
) -> Result<ProgressLedger, Box<ClauseDisposition>> {
    ledger
        .admit(binding, assertion, positions, covered_positions)
        .map_err(|refusal| match refusal {
            Refusal::Contradiction(contradiction) => Box::new(
                ClauseDisposition::ProgressContradiction(Box::new(contradiction.into())),
            ),
            Refusal::LimitReached(reached) => Box::new(ClauseDisposition::Temporal(
                TemporalVerdict::ProgressLimitReached(reached),
            )),
        })
}

/// FR-330 Inputs: one `Temporal` selection.
pub(super) struct TemporalRun<'a> {
    /// The clause's declared name.
    pub(super) name: &'a str,
    /// The supplied trace.
    pub(super) trace: &'a TraceInput,
    /// The clause's `over` binding.
    pub(super) over: &'a OverObject,
    /// The caller's ledger, when it carries one.
    pub(super) ledger: Option<&'a ProgressLedger>,
}

fn refused(cause: &'static str, fields: BTreeMap<&'static str, String>) -> ClauseDisposition {
    ClauseDisposition::Admit(AdmissionFailure::Refused(Box::new(AdmissionRecord {
        catalog: CatalogCode::new(Code::InvalidRuntimeInput.as_str(), cause),
        fields,
        path: Vec::new(),
        kernel: None,
    })))
}

/// FR-330: run the temporal clause `name` over `trace`.
pub(super) fn run_temporal(run: &CompiledRun<'_>, request: &TemporalRun<'_>) -> ClauseRunReport {
    let TemporalRun {
        name,
        trace,
        over,
        ledger,
        ..
    } = *request;
    // A trace is evidence for that trace only: it settles no item and
    // derives no witness (FR-332).
    let report = |disposition: ClauseDisposition, usage: ClauseRunUsage| ClauseRunReport {
        basis: SettlementBasis::Unavailable,
        witness: None,
        // A completed temporal verdict names no consequent events: it
        // refuses typed rather than reporting an empty list.
        consequent_source_events: if disposition.truth().is_some() {
            ConsequentSourceEvents::Refused(ConsequentEventRefusal::Temporal)
        } else {
            ConsequentSourceEvents::Unavailable
        },
        disposition,
        usage,
        // A refused request leaves the caller's ledger as it was.
        ledger: ledger.cloned(),
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

    let trace_matches_profile = match trace {
        TraceInput::Progressed { .. } => matches!(
            clause.profile(),
            TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension
        ),
        TraceInput::Timestamped { .. } => {
            clause.profile() == TemporalProfile::TimestampedEventFiniteWindow
        }
        TraceInput::Finite(_) | TraceInput::Lasso { .. } => true,
    };
    if !trace_matches_profile {
        return early(refused("invalid-value", BTreeMap::new()));
    }

    // A ledger retains progress; a trace that asserts none has nothing to
    // retain (FR-502 Behavior).
    if ledger.is_some()
        && (!trace.carries_progress() || clause.profile() == TemporalProfile::InfiniteTrace)
    {
        return early(refused("invalid-value", BTreeMap::new()));
    }

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
            quire_semantic_value::catalog::InternalFault::new("temporal", "over-is-a-reference"),
        ));
    };
    let universe = match population_universe_for(
        graph.model_selections(),
        run.packages,
        run.model_limits,
        type_identity.clone(),
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

    let mut accepted_ledger = None;
    if let Some(ledger) = ledger {
        let Some((assertion, positions)) = trace.progress() else {
            return early(refused("invalid-value", BTreeMap::new()));
        };
        let Some(package_identity) = run.sources.first().map(|source| source.identity().clone())
        else {
            return late(ClauseDisposition::EvaluateFault(
                quire_semantic_value::catalog::InternalFault::new(
                    "temporal-progress",
                    "missing-package-source-identity",
                ),
            ));
        };
        let binding = LedgerBinding {
            package: package_identity,
            clause: node.identity(),
            clock: clause.clock().to_owned(),
            profile: clause.profile().identity().to_owned(),
        };
        accepted_ledger = match admit_progress(
            ledger,
            &binding,
            assertion,
            &positions,
            run.progress_capacity,
        ) {
            Ok(updated) => Some(updated),
            Err(disposition) => return late(*disposition),
        };
    }

    if matches!(trace, TraceInput::Progressed { .. }) {
        // FR-504's progress-aware outcome rules are not part of the current
        // evaluator API. Keep the accepted ledger, but do not pass progress
        // assertions to the closed-finite-trace evaluator as if they were
        // equivalent.
        let mut report = late(ClauseDisposition::TemporalRefusal(
            TemporalTraceRefusal::ProfileNotEvaluated {
                profile: clause.profile(),
            },
        ));
        if accepted_ledger.is_some() {
            report.ledger = accepted_ledger;
        }
        return report;
    }

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
    let accepted = !matches!(
        disposition.category(),
        quire_semantic_value::catalog::Category::Refusal
            | quire_semantic_value::catalog::Category::InternalFailure
    );
    let mut report = report(disposition, ClauseRunUsage::from_meter(&meter, admitted));
    if accepted {
        report.ledger = accepted_ledger.or_else(|| ledger.cloned());
    }
    report
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
        TemporalOutcome::InsufficientData(insufficient) => TemporalVerdict::Undefined {
            at: insufficient.at.index(),
            cause: TemporalUndefinedCause::InsufficientData {
                count: insufficient.data.count,
                min_count: insufficient.data.min_count,
            },
        },
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
            quire_semantic_value::catalog::InternalFault::new("temporal", "step-evaluator-refusal"),
        ),
    }
}

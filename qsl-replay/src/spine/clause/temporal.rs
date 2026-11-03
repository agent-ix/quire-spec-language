// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330: the `Temporal` selection of [`super::run_clause`]. It resolves one
//! temporal clause by name, admits each observation of the supplied trace
//! (FR-106), binds the clause's `over` parameter at position 0 and evaluates
//! the clause once through the TemporalTrace evaluator (FR-327, FR-328,
//! FR-329).

use std::collections::BTreeMap;

use qsl_eval::temporal::{
    AtomReading, HoldsStop, PositionRefusal, TemporalOutcome, TemporalRefusal, TemporalTrace,
    UndefinedCause, UndefinedEvaluation, WindowRefusal,
};
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::TemporalProfile;
use qsl_semantics::check::{BinaryTemporal, FormulaNode, UnaryTemporal};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    admit_trace_snapshot, population_universe_for, AdmissionFailure, AdmissionRecord,
    AdmissionUsage, DocumentRef,
};
use quire_exact::{Integer, Meter, Rational, ValueType};
use quire_semantic_value::catalog::CatalogCode;

use super::{
    convert_call_failure, convert_outcome, ClauseDisposition, ClauseRunReport, ClauseRunUsage,
    CompiledRun, ConsequentEventRefusal, ConsequentSourceEvents, RunRefusal,
    TemporalIndeterminateReason, TemporalPositionRefusal, TemporalTraceRefusal,
    TemporalUndefinedCause, TemporalVerdict, TemporalWindowRefusal,
};
use crate::proof_result::SettlementBasis;

/// FR-330 Inputs: the observations a temporal clause is run over, each a
/// snapshot named by its labels (FR-106).
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
    /// A finite trace whose positions carry non-negative timestamp ticks and
    /// whose watermark and completeness assertion describe window closure.
    Timestamped(TimestampedTraceInput),
}

/// The timestamped observations and progress premises for FR-501.
#[derive(Clone, Debug)]
pub struct TimestampedTraceInput {
    /// Positions in admitted order. Stamps must be non-decreasing.
    pub positions: Vec<TimestampedObservation>,
    /// Inclusive progress watermark.
    pub watermark: u64,
    /// Completeness assertion through an inclusive tick, or no completeness.
    pub complete_through: Option<u64>,
    /// Whether the decision scope has closed.
    pub scope_closed: bool,
    /// Whether the surrounding execution has closed. It changes only the
    /// reported premise, never the verdict (FR-501 "Verdicts").
    pub execution_closed: bool,
    /// The clock binding's point or counter cover for each stamp.
    pub cover: TimestampedCover,
    /// Atom readings absent from an admitted observation. Atom identifies
    /// the zero-based `holds` formula node in the checked clause.
    pub missing_valuations: Vec<TimestampedMissingValuation>,
    /// Whether the first represented position is an authoritative origin.
    pub authoritative_origin: bool,
}

/// One explicitly absent timestamped atom reading.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimestampedMissingValuation {
    /// Zero-based trace position.
    pub position: u64,
    /// Zero-based checked formula node of the `holds` atom.
    pub atom: u64,
}

/// The declared meaning of an integer timestamp stamp.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimestampedCover {
    /// The stamp names an exact instant.
    Exact,
    /// The stamp covers `[k-u, k+1+u)` ticks.
    Counter {
        /// The counter reading's uncertainty in ticks.
        uncertainty: u64,
    },
}

impl TimestampedCover {
    fn uncertainty_margin(self) -> Option<u64> {
        match self {
            Self::Exact => Some(0),
            Self::Counter { uncertainty } => uncertainty.checked_mul(2),
        }
    }

    /// Compare the difference `ack - request` with an inclusive upper bound.
    pub fn within_bound(self, request: u64, ack: u64, bound: i64) -> CoverComparison {
        if self == Self::Exact {
            return if i128::from(ack) - i128::from(request) <= i128::from(bound) {
                CoverComparison::True
            } else {
                CoverComparison::False
            };
        }
        let (request_lower, request_upper) = self.interval(request);
        let (ack_lower, ack_upper) = self.interval(ack);
        let difference_lower = ack_lower - request_upper;
        let difference_upper = ack_upper - request_lower;
        let lower_open = true;
        if difference_upper <= i128::from(bound) {
            CoverComparison::True
        } else if difference_lower > i128::from(bound)
            || (difference_lower == i128::from(bound) && lower_open)
        {
            CoverComparison::False
        } else {
            CoverComparison::StraddlesBound
        }
    }

    /// Whether every instant in `first` precedes every instant in `second`.
    pub fn definitely_before(self, first: u64, second: u64) -> bool {
        if self == Self::Exact {
            return first < second;
        }
        let (_, first_upper) = self.interval(first);
        let (second_lower, _) = self.interval(second);
        first_upper <= second_lower
    }

    fn interval(self, tick: u64) -> (i128, i128) {
        let uncertainty = match self {
            Self::Exact => unreachable!("exact covers use direct comparisons"),
            Self::Counter { uncertainty } => i128::from(uncertainty),
        };
        let tick = i128::from(tick);
        (tick - uncertainty, tick + 1 + uncertainty)
    }
}

/// The possibility result of comparing counter-tick covers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CoverComparison {
    /// Every covered difference is within the bound.
    True,
    /// Every covered difference is outside the bound.
    False,
    /// The covered differences straddle the bound.
    StraddlesBound,
}

/// One timestamped position and its optional same-stamp order key.
#[derive(Clone, Debug)]
pub struct TimestampedObservation {
    /// The admitted snapshot at this position.
    pub document: DocumentRef,
    /// Non-negative timestamp tick in the selected clock's unit.
    pub stamp: u64,
    /// Authority-scoped order for positions sharing a stamp.
    pub order: Option<TimestampedOrderKey>,
}

/// An admitted order key comparable only within one authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimestampedOrderKey {
    /// Identity of the authority that assigned the key.
    pub authority: String,
    /// Key within that authority's ordered domain.
    pub key: i64,
}

impl TraceInput {
    /// Every observation, in position order.
    fn documents(&self) -> Box<dyn Iterator<Item = &DocumentRef> + '_> {
        match self {
            Self::Finite(positions) => Box::new(positions.iter()),
            Self::Lasso { prefix, repeat } => Box::new(prefix.iter().chain(repeat)),
            Self::Timestamped(trace) => Box::new(
                ordered_timestamped_positions(trace)
                    .into_iter()
                    .map(|position| &position.document),
            ),
        }
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
            Self::Timestamped(trace) => Some(TemporalTrace::Finite {
                len: u64::try_from(trace.positions.len()).ok()?,
            }),
        }
    }
}

fn ordered_timestamped_positions(input: &TimestampedTraceInput) -> Vec<&TimestampedObservation> {
    let mut positions: Vec<_> = input.positions.iter().collect();
    positions.sort_by(|left, right| {
        left.stamp
            .cmp(&right.stamp)
            .then_with(|| match (&left.order, &right.order) {
                (Some(left), Some(right)) if left.authority == right.authority => {
                    left.key.cmp(&right.key)
                }
                _ => std::cmp::Ordering::Equal,
            })
    });
    positions
}

fn same_stamp_order_refusal(
    input: &TimestampedTraceInput,
    clause: &qsl_semantics::check::CheckedTemporalClause,
) -> Option<u64> {
    let order_sensitive = clause.formula().nodes().iter().any(|node| {
        matches!(
            node,
            FormulaNode::Binary {
                operator: BinaryTemporal::Until
                    | BinaryTemporal::Release
                    | BinaryTemporal::Since
                    | BinaryTemporal::Triggered,
                ..
            }
        )
    });
    if !order_sensitive {
        return None;
    }
    let mut start = 0;
    while start < input.positions.len() {
        let stamp = input.positions[start].stamp;
        let mut end = start + 1;
        while end < input.positions.len() && input.positions[end].stamp == stamp {
            end += 1;
        }
        if end - start > 1 {
            let Some(first_order) = input.positions[start].order.as_ref() else {
                return Some(u64::try_from(end - 1).unwrap_or(u64::MAX));
            };
            let authority = first_order.authority.as_str();
            let mut keys = std::collections::BTreeSet::new();
            for position in &input.positions[start..end] {
                let Some(order) = position.order.as_ref() else {
                    return Some(u64::try_from(end - 1).unwrap_or(u64::MAX));
                };
                if order.authority != authority || !keys.insert(order.key) {
                    return Some(u64::try_from(end - 1).unwrap_or(u64::MAX));
                }
            }
        }
        start = end;
    }
    None
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
    ClauseDisposition::Admit(AdmissionFailure::Refused(Box::new(AdmissionRecord {
        catalog: CatalogCode::new(Code::InvalidRuntimeInput.as_str(), cause),
        fields,
        path: Vec::new(),
        kernel: None,
    })))
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
        finite_window: None,
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

    let timestamped_profile = clause.profile() == TemporalProfile::TimestampedEventFiniteWindow;
    let timestamped_input = matches!(trace, TraceInput::Timestamped(_));
    if timestamped_profile != timestamped_input {
        return early(refused("invalid-value", BTreeMap::new()));
    }
    if let TraceInput::Timestamped(input) = trace {
        if input.positions.is_empty()
            || input.cover.uncertainty_margin().is_none()
            || input
                .positions
                .windows(2)
                .any(|positions| positions[0].stamp > positions[1].stamp)
        {
            return early(refused("invalid-value", BTreeMap::new()));
        }
        if input.missing_valuations.iter().any(|missing| {
            usize::try_from(missing.position)
                .ok()
                .is_none_or(|position| position >= input.positions.len())
                || usize::try_from(missing.atom).ok().is_none_or(|atom| {
                    !matches!(
                        clause.formula().nodes().get(atom),
                        Some(FormulaNode::Holds { .. })
                    )
                })
        }) {
            return early(refused("invalid-value", BTreeMap::new()));
        }
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

    if let TraceInput::Timestamped(input) = trace {
        if let Some(position) = same_stamp_order_refusal(input, clause) {
            return late(ClauseDisposition::TemporalRefusal(
                TemporalTraceRefusal::SameStampOrderUnavailable { position },
            ));
        }
    }

    let mut meter = Meter::new(run.accounting);
    let observed = match trace {
        TraceInput::Timestamped(input) => {
            let ordered_positions = ordered_timestamped_positions(input);
            let stamps = ordered_positions
                .iter()
                .map(|position| Rational::from_integer(Integer::from(position.stamp)))
                .collect();
            match qsl_eval::temporal::TimedTrace::new(stamps) {
                Ok(timed_trace) => {
                    let margin = Rational::from_integer(Integer::from(
                        input.cover.uncertainty_margin().unwrap_or_default(),
                    ));
                    let watermark = Rational::from_integer(Integer::from(input.watermark));
                    let complete_through = input
                        .complete_through
                        .map(|through| Rational::from_integer(Integer::from(through)));
                    let missing_valuations = input
                        .missing_valuations
                        .iter()
                        .map(|missing| {
                            let original = &input.positions[missing.position as usize];
                            let position = ordered_positions
                                .iter()
                                .position(|candidate| std::ptr::eq(*candidate, original))
                                .unwrap_or(missing.position as usize);
                            (u64::try_from(position).unwrap_or(u64::MAX), missing.atom)
                        })
                        .collect();
                    let progress = qsl_eval::temporal::FiniteWindowProgress::new(
                        watermark,
                        complete_through,
                        margin,
                        missing_valuations,
                    );
                    qsl_eval::temporal::evaluate_observed_finite_window(
                        run.package,
                        node,
                        &environments,
                        &timed_trace,
                        reference,
                        &progress,
                        &mut meter,
                    )
                }
                Err(refusal) => {
                    return late(ClauseDisposition::TemporalRefusal(trace_refusal(refusal)));
                }
            }
        }
        TraceInput::Finite(_) | TraceInput::Lasso { .. } => qsl_eval::temporal::evaluate_observed(
            run.package,
            node,
            &environments,
            shape,
            reference,
            &mut meter,
        ),
    };
    let pending_activation = observed.pending_activation;
    let atom_readings = observed.atom_readings;
    let temporal_visits = observed.temporal_visits;
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
        (None, Ok(outcome)) => {
            let verdict = match trace {
                TraceInput::Timestamped(input) => finite_window_verdict(
                    outcome,
                    input,
                    clause,
                    pending_activation,
                    &atom_readings,
                ),
                TraceInput::Finite(_) | TraceInput::Lasso { .. } => verdict(outcome),
            };
            ClauseDisposition::Temporal(verdict)
        }
        (None, Err(TemporalRefusal::Fault(fault))) => ClauseDisposition::EvaluateFault(fault),
        (None, Err(refusal)) => ClauseDisposition::TemporalRefusal(trace_refusal(refusal)),
    };
    let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
    usage.temporal_visits = temporal_visits;
    // FR-501 "Result report": a completed finite-window verdict carries its
    // basis, decision support, premises and result identity.
    let window = match (trace, &disposition) {
        (TraceInput::Timestamped(input), ClauseDisposition::Temporal(_)) => {
            match finite_window_report(run, node, input, &environments, reference) {
                Ok(window) => Some(window),
                Err(fault) => {
                    let mut failed = report(ClauseDisposition::EvaluateFault(fault), usage);
                    failed.finite_window = None;
                    return failed;
                }
            }
        }
        _ => None,
    };
    let mut finished = report(disposition, usage);
    finished.finite_window = window;
    finished
}

/// FR-501 "Result report": the basis, decision support, premises and result
/// identity of a finite-window run, from the possibility-set evaluator over
/// the same admitted observations and progress the verdict was read from.
fn finite_window_report(
    run: &CompiledRun<'_>,
    node: &qsl_semantics::check::TemporalClauseNode,
    input: &TimestampedTraceInput,
    environments: &[ObjectEnvironment],
    over: &quire_exact::ObjectReference,
) -> Result<FiniteWindowReport, quire_semantic_value::catalog::InternalFault> {
    use qsl_eval::temporal::{
        evaluate_observed_finite_window_report, ClockCover, Closure, Completeness, LowerBoundary,
        TimestampedObservation as WindowObservation, TimestampedTrace, Watermark, WindowClock,
        WindowProgress,
    };
    let fault = |invariant: &'static str| {
        quire_semantic_value::catalog::InternalFault::new("temporal", invariant)
    };
    let closure = |closed: bool| {
        if closed {
            Closure::Closed
        } else {
            Closure::Open
        }
    };
    // The supplied positions in the order admission read them: the same
    // stamp-then-key order the verdict was read in.
    let ordered = ordered_timestamped_positions(input);
    let trace = TimestampedTrace::admit(
        node.clause().name(),
        WindowClock {
            unit: node.clause().clock().to_owned(),
            cover: match input.cover {
                TimestampedCover::Exact => ClockCover::Exact,
                TimestampedCover::Counter { uncertainty } => ClockCover::Counter { uncertainty },
            },
        },
        ordered
            .iter()
            .map(|position| WindowObservation {
                stamp: position.stamp,
                order_key: position.order.as_ref().and_then(|order| {
                    u64::try_from(order.key)
                        .ok()
                        .map(|key| qsl_eval::temporal::OrderKey {
                            authority: order.authority.clone(),
                            key,
                        })
                }),
                observation: format!("{:?}", position.document),
            })
            .collect(),
        WindowProgress {
            watermark: Watermark::Inclusive(input.watermark),
            completeness: input
                .complete_through
                .map_or(Completeness::Incomplete, |through| Completeness::Complete {
                    through,
                }),
            scope: closure(input.scope_closed),
            execution: closure(input.execution_closed),
            lower_boundary: if input.authoritative_origin {
                LowerBoundary::AuthoritativeOrigin
            } else {
                LowerBoundary::HistoryCutoff
            },
        },
    )
    .map_err(|_| fault("finite-window-trace-admits"))?;
    // The evaluator reads the same positions the verdict did: its positions
    // are the admitted order, and the missing valuations were rewritten to it.
    let missing: Vec<(u64, u64)> = input
        .missing_valuations
        .iter()
        .filter_map(|missing| {
            let original = input
                .positions
                .get(usize::try_from(missing.position).ok()?)?;
            let position = ordered
                .iter()
                .position(|candidate| std::ptr::eq(*candidate, original))?;
            Some((u64::try_from(position).ok()?, missing.atom))
        })
        .collect();
    let mut meter = Meter::new(run.accounting);
    let observed = evaluate_observed_finite_window_report(
        run.package,
        node,
        environments,
        &trace,
        over,
        &missing,
        &mut meter,
    );
    match observed.result {
        Ok(evaluation) => Ok(FiniteWindowReport {
            basis: evaluation.basis,
            support: evaluation.support,
            premises: evaluation.premises,
            identity: evaluation.identity,
        }),
        Err(TemporalRefusal::Fault(fault)) => Err(fault),
        Err(_) => Err(fault("finite-window-report-reads-the-trace")),
    }
}

/// FR-330, FR-501 "Result report": the basis, decision support, premises and
/// result identity of a finite-window run, as separate fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteWindowReport {
    /// What the verdict rests on.
    pub basis: qsl_eval::temporal::Basis,
    /// The decision support, as `(position, atom)` pairs.
    pub support: Vec<qsl_eval::temporal::SupportPair>,
    /// The premises: the four independent dimensions and the inputs that
    /// qualify the verdict.
    pub premises: qsl_eval::temporal::Premises,
    /// The result identity.
    pub identity: qsl_eval::temporal::ResultIdentity,
}

fn finite_window_verdict(
    outcome: TemporalOutcome,
    input: &TimestampedTraceInput,
    clause: &qsl_semantics::check::CheckedTemporalClause,
    pending_activation: Option<(qsl_eval::temporal::TemporalPosition, Rational)>,
    atom_readings: &[(
        qsl_semantics::check::FormulaNodeId,
        qsl_eval::temporal::TemporalPosition,
        AtomReading,
    )],
) -> TemporalVerdict {
    let ordered_positions = ordered_timestamped_positions(input);
    let stamps = ordered_positions
        .iter()
        .map(|position| Rational::from_integer(Integer::from(position.stamp)))
        .collect();
    let Ok(trace) = qsl_eval::temporal::TimedTrace::new(stamps) else {
        return verdict(outcome);
    };
    let margin = Rational::from_integer(Integer::from(
        input.cover.uncertainty_margin().unwrap_or_default(),
    ));
    let (position, deadline) = if let Some((position, deadline)) = pending_activation {
        (position, deadline.numerator().to_u64().unwrap_or(u64::MAX))
    } else {
        let deadline = qsl_eval::temporal::finite_window_deadline(
            clause,
            &trace,
            qsl_eval::temporal::TemporalPosition::new(0),
            &margin,
        )
        .ok()
        .and_then(|value| value.numerator().to_u64())
        .unwrap_or(u64::MAX);
        (qsl_eval::temporal::TemporalPosition::new(0), deadline)
    };
    let position = position.index();
    let verdict = match outcome {
        TemporalOutcome::Pending if input.watermark >= deadline => {
            if input
                .complete_through
                .is_none_or(|through| through < deadline)
            {
                TemporalVerdict::Indeterminate {
                    at: position,
                    reason: TemporalIndeterminateReason::IncompleteInput,
                }
            } else {
                TemporalVerdict::PendingWindow {
                    at: position,
                    deadline,
                }
            }
        }
        TemporalOutcome::Pending => TemporalVerdict::PendingWindow {
            at: position,
            deadline,
        },
        TemporalOutcome::Undefined(undefined)
            if input.missing_valuations.iter().any(|missing| {
                missing.position == undefined.at.index()
                    && undefined.cause == UndefinedCause::Kernel(quire_exact::Undefined::NoneValue)
            }) && input.watermark < deadline =>
        {
            TemporalVerdict::PendingWindow {
                at: position,
                deadline,
            }
        }
        TemporalOutcome::Undefined(undefined)
            if input.missing_valuations.iter().any(|missing| {
                missing.position == undefined.at.index()
                    && undefined.cause == UndefinedCause::Kernel(quire_exact::Undefined::NoneValue)
            }) && input
                .complete_through
                .is_none_or(|through| through < deadline) =>
        {
            TemporalVerdict::Indeterminate {
                at: position,
                reason: TemporalIndeterminateReason::IncompleteInput,
            }
        }
        TemporalOutcome::Undefined(undefined) => {
            if let Some(missing) = input.missing_valuations.iter().find(|missing| {
                missing.position == undefined.at.index()
                    && undefined.cause == UndefinedCause::Kernel(quire_exact::Undefined::NoneValue)
            }) {
                TemporalVerdict::Indeterminate {
                    at: position,
                    reason: TemporalIndeterminateReason::MissingValuation {
                        position: missing.position,
                        atom: format!("holds-node@{}", missing.atom),
                    },
                }
            } else {
                verdict(TemporalOutcome::Undefined(undefined))
            }
        }
        other => verdict(other),
    };
    if counter_straddles_bound(input, clause, atom_readings).is_some() {
        return TemporalVerdict::Indeterminate {
            at: 0,
            reason: TemporalIndeterminateReason::StraddlesBound,
        };
    }
    if let Some((conflict_position, atom)) =
        conflicting_timestamped_valuation(input, clause, atom_readings)
    {
        return TemporalVerdict::Indeterminate {
            at: 0,
            reason: TemporalIndeterminateReason::ConflictingValuation {
                position: conflict_position,
                atom,
            },
        };
    }
    if !input.authoritative_origin
        && input.positions[0].stamp > 0
        && has_past_operator(clause)
        && matches!(verdict, TemporalVerdict::Violated { .. })
    {
        TemporalVerdict::Indeterminate {
            at: position,
            reason: TemporalIndeterminateReason::MissingHistory,
        }
    } else {
        verdict
    }
}

fn conflicting_timestamped_valuation(
    input: &TimestampedTraceInput,
    clause: &qsl_semantics::check::CheckedTemporalClause,
    readings: &[(
        qsl_semantics::check::FormulaNodeId,
        qsl_eval::temporal::TemporalPosition,
        AtomReading,
    )],
) -> Option<(u64, String)> {
    let nodes = clause.formula().nodes();
    let FormulaNode::Unary {
        operator: UnaryTemporal::Eventually,
        interval: Some(qsl_semantics::check::CheckedInterval::Timed(interval)),
        operand,
    } = nodes.get(clause.formula().root().index())?
    else {
        return None;
    };
    if !matches!(nodes.get(operand.index()), Some(FormulaNode::Holds { .. })) {
        return None;
    }
    let ordered = ordered_timestamped_positions(input);
    let first_stamp = ordered.first()?.stamp;
    let atom_index = operand.index();
    for left in 0..ordered.len() {
        let delta = i128::from(ordered[left].stamp) - i128::from(first_stamp);
        if delta < 0 {
            continue;
        }
        let delta = Rational::from_integer(Integer::from(u64::try_from(delta).ok()?));
        if !interval.contains(&delta) {
            continue;
        }
        let left_at = u64::try_from(left).ok()?;
        let left_true = readings.iter().any(|(atom, at, value)| {
            atom.index() == atom_index && at.index() == left_at && *value == AtomReading::True
        });
        if !left_true {
            continue;
        }
        for right in left + 1..ordered.len() {
            if ordered[right].stamp != ordered[left].stamp {
                break;
            }
            let right_at = u64::try_from(right).ok()?;
            let right_false = readings.iter().any(|(atom, at, value)| {
                atom.index() == atom_index && at.index() == right_at && *value == AtomReading::False
            });
            if right_false {
                let decisive_true_elsewhere = readings.iter().any(|(atom, at, value)| {
                    if atom.index() != atom_index || *value != AtomReading::True {
                        return false;
                    }
                    let index = usize::try_from(at.index()).ok();
                    index.is_some_and(|index| {
                        ordered.get(index).is_some_and(|position| {
                            position.stamp != ordered[left].stamp
                                && interval.contains(&Rational::from_integer(Integer::from(
                                    position.stamp - first_stamp,
                                )))
                        })
                    })
                });
                if !decisive_true_elsewhere {
                    return Some((left_at, format!("holds-node@{atom_index}")));
                }
            }
        }
    }
    None
}

fn counter_straddles_bound(
    input: &TimestampedTraceInput,
    clause: &qsl_semantics::check::CheckedTemporalClause,
    readings: &[(
        qsl_semantics::check::FormulaNodeId,
        qsl_eval::temporal::TemporalPosition,
        AtomReading,
    )],
) -> Option<u64> {
    let TimestampedCover::Counter { .. } = input.cover else {
        return None;
    };
    let nodes = clause.formula().nodes();
    let FormulaNode::Implies(antecedent, consequent) =
        nodes.get(clause.formula().root().index())?
    else {
        return None;
    };
    let FormulaNode::Holds { .. } = nodes.get(antecedent.index())? else {
        return None;
    };
    let FormulaNode::Unary {
        operator: UnaryTemporal::Eventually,
        interval: Some(qsl_semantics::check::CheckedInterval::Timed(interval)),
        operand,
    } = nodes.get(consequent.index())?
    else {
        return None;
    };
    let FormulaNode::Holds { .. } = nodes.get(operand.index())? else {
        return None;
    };
    let bound = interval.upper().numerator().to_u64()?;
    if interval.upper().denominator().to_u64() != Some(1) {
        return None;
    }
    let antecedent_id = *antecedent;
    let consequent_id = *operand;
    let is_true = |node, position| {
        readings.iter().any(|(candidate, at, value)| {
            *candidate == node && at.index() == position && *value == AtomReading::True
        })
    };
    if !is_true(antecedent_id, 0) {
        return None;
    }
    let ordered = ordered_timestamped_positions(input);
    let request = ordered.first()?.stamp;
    for (index, observation) in ordered.iter().enumerate() {
        if !is_true(consequent_id, u64::try_from(index).ok()?) {
            continue;
        }
        if input
            .cover
            .within_bound(request, observation.stamp, i64::try_from(bound).ok()?)
            == CoverComparison::StraddlesBound
        {
            return u64::try_from(index).ok();
        }
    }
    None
}

fn has_past_operator(clause: &qsl_semantics::check::CheckedTemporalClause) -> bool {
    clause.formula().nodes().iter().any(|node| {
        matches!(
            node,
            FormulaNode::Unary {
                operator: UnaryTemporal::Once | UnaryTemporal::Historically,
                ..
            } | FormulaNode::Binary {
                operator: BinaryTemporal::Since | BinaryTemporal::Triggered,
                ..
            }
        )
    })
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
        | TemporalRefusal::NotFiniteWindowProfile { .. }
        | TemporalRefusal::UnorderedTie { .. }
        | TemporalRefusal::NegativeStamp { .. }
        | TemporalRefusal::DecreasingStamp { .. }
        | TemporalRefusal::NonPositiveLoopDelay
        | TemporalRefusal::LoopReentersBeforeLastStamp => TemporalTraceRefusal::Fault(
            quire_semantic_value::catalog::InternalFault::new("temporal", "step-evaluator-refusal"),
        ),
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330: the `Temporal` selection of [`super::run_clause`]. It resolves one
//! temporal clause by name, admits each observation of the supplied trace
//! (FR-106), binds the clause's `over` parameter at position 0 and evaluates
//! the clause once through the TemporalTrace evaluator (FR-327, FR-328,
//! FR-329).

use std::collections::{BTreeMap, BTreeSet};

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
    timestamped_evaluation_positions(input, clause, None).2
}

fn timestamped_evaluation_positions<'a>(
    input: &'a TimestampedTraceInput,
    clause: &qsl_semantics::check::CheckedTemporalClause,
    activation: Option<usize>,
) -> (
    Vec<&'a TimestampedObservation>,
    BTreeSet<(usize, usize)>,
    Option<u64>,
) {
    let positions = ordered_timestamped_positions(input);
    let initial = activation.map_or_else(
        || match clause.activation() {
            qsl_semantics::check::Activation::Origin => vec![0],
            qsl_semantics::check::Activation::Each { .. } => (0..positions.len()).collect(),
        },
        |position| vec![position],
    );
    let mut visited = BTreeSet::new();
    let mut refusal = None;

    fn visit(
        node_id: qsl_semantics::check::FormulaNodeId,
        at: usize,
        formula: &qsl_semantics::check::CheckedFormula,
        positions: &[&TimestampedObservation],
        visited: &mut BTreeSet<(usize, usize)>,
        refusal: &mut Option<u64>,
    ) {
        if refusal.is_some() || !visited.insert((node_id.index(), at)) {
            return;
        }
        let Some(node) = formula.nodes().get(node_id.index()) else {
            return;
        };
        match node {
            FormulaNode::Not(child) => visit(*child, at, formula, positions, visited, refusal),
            FormulaNode::And(left, right)
            | FormulaNode::Or(left, right)
            | FormulaNode::Implies(left, right) => {
                visit(*left, at, formula, positions, visited, refusal);
                visit(*right, at, formula, positions, visited, refusal);
            }
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                let future = matches!(operator, UnaryTemporal::Eventually | UnaryTemporal::Always);
                for position in eligible_positions(positions, at, interval.as_ref(), future) {
                    visit(*operand, position, formula, positions, visited, refusal);
                }
            }
            FormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => {
                let future = matches!(operator, BinaryTemporal::Until | BinaryTemporal::Release);
                let eligible = eligible_positions(positions, at, interval.as_ref(), future);
                if matches!(
                    operator,
                    BinaryTemporal::Until
                        | BinaryTemporal::Release
                        | BinaryTemporal::Since
                        | BinaryTemporal::Triggered
                ) {
                    *refusal = incomparable_equal_stamp_position(positions, &eligible);
                }
                for position in eligible {
                    visit(*left, position, formula, positions, visited, refusal);
                    visit(*right, position, formula, positions, visited, refusal);
                }
            }
            FormulaNode::Constant(_)
            | FormulaNode::Holds { .. }
            | FormulaNode::Aggregate { .. } => {}
        }
    }

    for position in initial {
        visit(
            clause.formula().root(),
            position,
            clause.formula(),
            &positions,
            &mut visited,
            &mut refusal,
        );
    }
    (positions, visited, refusal)
}

fn eligible_positions(
    positions: &[&TimestampedObservation],
    anchor: usize,
    interval: Option<&qsl_semantics::check::CheckedInterval>,
    future: bool,
) -> Vec<usize> {
    let Some(anchor_position) = positions.get(anchor) else {
        return Vec::new();
    };
    positions
        .iter()
        .enumerate()
        .filter_map(|(index, position)| {
            let delta = if future {
                if position.stamp < anchor_position.stamp {
                    return None;
                }
                position.stamp - anchor_position.stamp
            } else {
                if position.stamp > anchor_position.stamp {
                    return None;
                }
                anchor_position.stamp - position.stamp
            };
            let delta = Rational::from_integer(Integer::from(delta));
            match interval {
                Some(qsl_semantics::check::CheckedInterval::Timed(interval))
                    if !interval.contains(&delta) =>
                {
                    None
                }
                Some(qsl_semantics::check::CheckedInterval::Steps(_)) => None,
                _ => Some(index),
            }
        })
        .collect()
}

fn incomparable_equal_stamp_position(
    positions: &[&TimestampedObservation],
    eligible: &[usize],
) -> Option<u64> {
    let eligible: BTreeSet<_> = eligible.iter().copied().collect();
    let mut start = 0;
    while start < positions.len() {
        let stamp = positions[start].stamp;
        let mut end = start + 1;
        while end < positions.len() && positions[end].stamp == stamp {
            end += 1;
        }
        let group: Vec<_> = (start..end)
            .filter(|position| eligible.contains(position))
            .collect();
        if group.len() > 1 {
            let Some(authority) = group
                .first()
                .and_then(|position| positions[*position].order.as_ref())
                .map(|order| order.authority.as_str())
            else {
                return Some(u64::try_from(*group.last()?).unwrap_or(u64::MAX));
            };
            let mut keys = BTreeSet::new();
            for position in &group {
                let Some(order) = positions[*position].order.as_ref() else {
                    return Some(u64::try_from(*group.last()?).unwrap_or(u64::MAX));
                };
                if order.authority != authority || !keys.insert(order.key) {
                    return Some(u64::try_from(*group.last()?).unwrap_or(u64::MAX));
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
                    let progress = match qsl_eval::temporal::FiniteWindowProgress::new(
                        watermark,
                        complete_through,
                        margin,
                        missing_valuations,
                    ) {
                        Ok(progress) => progress,
                        Err(refusal) => {
                            return late(ClauseDisposition::TemporalRefusal(trace_refusal(
                                refusal,
                            )));
                        }
                    };
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
    let unestablished_activation = observed.unestablished_activation;
    let atom_readings = observed.atom_readings;
    let conflicting_valuation = observed.conflicting_valuation;
    let temporal_visits = observed.temporal_visits;
    let disposition = if let Some(position) = unestablished_activation {
        match node.guard() {
            Some((guard, _)) => {
                ClauseDisposition::Temporal(TemporalVerdict::ActivationIncomplete {
                    at: position.index(),
                    guard: guard.location().clone(),
                })
            }
            None => {
                ClauseDisposition::EvaluateFault(quire_semantic_value::catalog::InternalFault::new(
                    "temporal.observed",
                    "undefined-activation-has-guard",
                ))
            }
        }
    } else {
        match (observed.stopped, observed.result) {
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
                        conflicting_valuation,
                    ),
                    TraceInput::Finite(_) | TraceInput::Lasso { .. } => verdict(outcome),
                };
                ClauseDisposition::Temporal(verdict)
            }
            (None, Err(TemporalRefusal::Fault(fault))) => ClauseDisposition::EvaluateFault(fault),
            (None, Err(refusal)) => ClauseDisposition::TemporalRefusal(trace_refusal(refusal)),
        }
    };
    let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
    usage.temporal_visits = temporal_visits;
    report(disposition, usage)
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
    conflicting_valuation: Option<(
        qsl_semantics::check::FormulaNodeId,
        qsl_eval::temporal::TemporalPosition,
    )>,
) -> TemporalVerdict {
    let violated_activation = match &outcome {
        TemporalOutcome::Violated { at } => Some(at.index()),
        _ => None,
    };
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
            } else if let Some((atom, conflict_position)) = conflicting_valuation {
                TemporalVerdict::Indeterminate {
                    at: position,
                    reason: TemporalIndeterminateReason::ConflictingValuation {
                        position: conflict_position.index(),
                        atom: format!("holds-node@{}", atom.index()),
                    },
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
    if !input.authoritative_origin
        && violated_activation.is_some_and(|activation| {
            history_cutoff_precedes_trace(input, clause, usize::try_from(activation).ok())
        })
        && matches!(verdict, TemporalVerdict::Violated { .. })
    {
        TemporalVerdict::Indeterminate {
            at: violated_activation.unwrap_or_default(),
            reason: TemporalIndeterminateReason::MissingHistory,
        }
    } else {
        verdict
    }
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
    let bound = i64::try_from(bound).ok()?;
    if ordered.iter().enumerate().any(|(index, observation)| {
        is_true(consequent_id, u64::try_from(index).unwrap_or(u64::MAX))
            && input.cover.within_bound(request, observation.stamp, bound) == CoverComparison::True
    }) {
        return None;
    }
    for (index, observation) in ordered.iter().enumerate() {
        if !is_true(consequent_id, u64::try_from(index).ok()?) {
            continue;
        }
        if input.cover.within_bound(request, observation.stamp, bound)
            == CoverComparison::StraddlesBound
        {
            return u64::try_from(index).ok();
        }
    }
    None
}

fn history_cutoff_precedes_trace(
    input: &TimestampedTraceInput,
    clause: &qsl_semantics::check::CheckedTemporalClause,
    activation: Option<usize>,
) -> bool {
    let (positions, evaluated, _) = timestamped_evaluation_positions(input, clause, activation);
    let Some(first_stamp) = positions.first().map(|position| position.stamp) else {
        return false;
    };
    let first_stamp = Rational::from_integer(Integer::from(first_stamp));
    evaluated.into_iter().any(|(node_index, position_index)| {
        let Some(node) = clause.formula().nodes().get(node_index) else {
            return false;
        };
        let interval = match node {
            FormulaNode::Unary {
                operator: UnaryTemporal::Once | UnaryTemporal::Historically,
                interval,
                ..
            }
            | FormulaNode::Binary {
                operator: BinaryTemporal::Since | BinaryTemporal::Triggered,
                interval,
                ..
            } => interval.as_ref(),
            _ => return false,
        };
        let Some(position) = positions.get(position_index) else {
            return false;
        };
        let now = Rational::from_integer(Integer::from(position.stamp));
        match interval {
            Some(qsl_semantics::check::CheckedInterval::Timed(interval)) => {
                now.sub(interval.upper()) < first_stamp
            }
            Some(qsl_semantics::check::CheckedInterval::Steps(_)) => false,
            None => now > Rational::from_integer(Integer::from(0_i64)),
        }
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
        TemporalRefusal::NegativeUncertaintyMargin => TemporalTraceRefusal::InvalidProgress,
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

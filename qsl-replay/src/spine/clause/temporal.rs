// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330: the `Temporal` selection of [`super::run_clause`]. It resolves one
//! temporal clause by name, admits each observation of the supplied trace
//! (FR-106), binds the clause's `over` parameter at position 0 and evaluates
//! the clause once through the TemporalTrace evaluator (FR-327, FR-328,
//! FR-329).

use std::collections::BTreeMap;

use qsl_eval::temporal::{
    ClockCover, HoldsStop, PositionRefusal, TemporalOutcome, TemporalRefusal, TemporalTrace,
    UndefinedCause, UndefinedEvaluation, WindowRefusal,
};
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::FormulaNode;
use qsl_semantics::check::TemporalProfile;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    admit_trace_snapshot, population_universe_for, AdmissionFailure, AdmissionRecord,
    AdmissionUsage, DocumentRef,
};
use quire_exact::{Meter, ValueType};
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
    /// The observations, in any order: the evaluator reads them in canonical
    /// order (FR-501 "Canonical order") and never consults this listing.
    pub positions: Vec<TimestampedObservation>,
    /// The profile, clock binding name and clock parameters the trace asserts
    /// (FR-522), required: a trace with no binding is refused at stage
    /// `admit` (FR-330-AC-11).
    pub binding: Option<TraceBinding>,
    /// Inclusive progress watermark.
    pub watermark: u64,
    /// Completeness assertion through an inclusive tick, or no completeness.
    pub complete_through: Option<u64>,
    /// Whether the decision scope has closed.
    pub scope_closed: bool,
    /// Whether the surrounding execution has closed. It changes only the
    /// reported premise, never the verdict (FR-501 "Verdicts").
    pub execution_closed: bool,
    /// The clock binding's declared cover of a stamp (FR-501 Inputs).
    pub cover: ClockCover,
    /// Atom valuations absent from an admitted observation (QSpec FR-160):
    /// each is a position, by its index in `positions`, and the zero-based
    /// `holds` formula node of the checked clause.
    pub missing_valuations: Vec<TimestampedMissingValuation>,
    /// Atom valuations at which two differing valuations were admitted at the
    /// one position (QSpec FR-160), named as `missing_valuations` are.
    pub conflicting_valuations: Vec<TimestampedMissingValuation>,
    /// Whether the first represented position is an authoritative origin.
    pub authoritative_origin: bool,
}

/// FR-522 `TraceBinding`: what a trace of tick-valued quantities asserts
/// about the clause it is run under.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceBinding {
    /// The profile identity the trace asserts.
    pub profile: String,
    /// The clock binding name the trace asserts.
    pub clock_name: String,
    /// The clock configuration the trace asserts.
    pub parameters: qsl_package::ClockConfiguration,
}

/// One timestamped atom valuation, absent or conflicting, named by position
/// and atom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimestampedMissingValuation {
    /// Zero-based trace position.
    pub position: u64,
    /// Zero-based checked formula node of the `holds` atom.
    pub atom: u64,
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

/// The identity of an observation's document for ordering (FR-106): its four
/// labels joined by NUL, which orders as the labels do field by field.
pub(super) fn observation_identity(document: &DocumentRef) -> String {
    format!(
        "{}\0{}\0{}\0{}",
        document.authority, document.identity, document.revision_namespace, document.revision
    )
}

/// FR-501 "Canonical order": ascending stamp, then ascending order-authority
/// identity (an observation without an order key after every keyed one), then
/// ascending key within the authority, then ascending observation identity.
/// A total order over distinct observations: the listing order decides
/// nothing.
fn ordered_timestamped_positions(input: &TimestampedTraceInput) -> Vec<&TimestampedObservation> {
    let mut positions: Vec<_> = input.positions.iter().collect();
    positions.sort_by(|left, right| {
        fn order(position: &TimestampedObservation) -> Option<(&str, i64)> {
            position
                .order
                .as_ref()
                .map(|order| (order.authority.as_str(), order.key))
        }
        left.stamp
            .cmp(&right.stamp)
            .then_with(|| match (order(left), order(right)) {
                (Some(left), Some(right)) => left.cmp(&right),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
            .then_with(|| left.document.cmp(&right.document))
    });
    positions
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

/// FR-522 "Authentication": the refusal of a `TraceBinding` that is absent or
/// that asserts another profile or clock binding name than the clause's, at
/// stage `admit`, `invalid_runtime_input`/`invalid-value` with
/// `mismatched_dimension` and the asserted and expected values. The dimensions
/// are tested in the order `Profile`, `Clock`.
///
/// The clock parameters (`ClockParameters` and the member dimensions) compare
/// against the `temporal_bindings` row of the declaration, which the replay
/// side of the checked package does not expose; they are not compared here.
fn binding_refusal(
    binding: Option<&TraceBinding>,
    clause: &qsl_semantics::check::CheckedTemporalClause,
) -> Option<ClauseDisposition> {
    let Some(binding) = binding else {
        return Some(refused("invalid-value", BTreeMap::new()));
    };
    let differ = |dimension: &'static str, asserted: &str, expected: &str| {
        Some(refused(
            "invalid-value",
            BTreeMap::from([
                ("mismatched_dimension", dimension.to_owned()),
                ("asserted", asserted.to_owned()),
                ("expected", expected.to_owned()),
            ]),
        ))
    };
    if binding.profile != clause.profile().identity() {
        return differ("Profile", &binding.profile, clause.profile().identity());
    }
    if binding.clock_name != clause.clock() {
        return differ("Clock", &binding.clock_name, clause.clock());
    }
    None
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
        // FR-522: the trace's binding is authenticated before any position is
        // visited, any stamp or completeness is checked or the meter is
        // charged (FR-330-AC-11).
        if let Some(refusal) = binding_refusal(input.binding.as_ref(), clause) {
            return early(refusal);
        }
        if input.positions.is_empty() {
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

    let mut meter = Meter::new(run.accounting);
    if let TraceInput::Timestamped(input) = trace {
        let (disposition, visits, window) =
            run_finite_window(run, node, input, &environments, reference, &mut meter);
        let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
        usage.temporal_visits = visits;
        let mut finished = report(disposition, usage);
        finished.finite_window = window;
        return finished;
    }
    let observed = qsl_eval::temporal::evaluate_observed(
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

/// FR-501: evaluate the finite-window clause `node` over the admitted
/// `environments`, one per observation in canonical order, and report its
/// verdict with its basis, decision support, premises and result identity.
/// Returns the disposition, the `temporal.visit` charges made and the report.
fn run_finite_window(
    run: &CompiledRun<'_>,
    node: &qsl_semantics::check::TemporalClauseNode,
    input: &TimestampedTraceInput,
    environments: &[ObjectEnvironment],
    over: &quire_exact::ObjectReference,
    meter: &mut Meter,
) -> (ClauseDisposition, u64, Option<FiniteWindowReport>) {
    use qsl_eval::temporal::{
        evaluate_observed_finite_window, Closure, Completeness, LowerBoundary,
        TimestampedObservation as WindowObservation, TimestampedTrace, Watermark, WindowClock,
        WindowProgress,
    };
    let fault = |invariant: &'static str| {
        (
            ClauseDisposition::EvaluateFault(quire_semantic_value::catalog::InternalFault::new(
                "temporal", invariant,
            )),
            0,
            None,
        )
    };
    let clause = node.clause();
    let closure = |closed: bool| {
        if closed {
            Closure::Closed
        } else {
            Closure::Open
        }
    };
    let ordered = ordered_timestamped_positions(input);
    let trace = match TimestampedTrace::admit(
        clause.name(),
        WindowClock {
            unit: clause.clock().to_owned(),
            cover: input.cover,
        },
        ordered
            .iter()
            .map(|position| WindowObservation {
                stamp: position.stamp,
                order_key: position
                    .order
                    .as_ref()
                    .map(|order| qsl_eval::temporal::OrderKey {
                        authority: order.authority.clone(),
                        key: order.key,
                    }),
                observation: observation_identity(&position.document),
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
    ) {
        Ok(trace) => trace,
        Err(_) => return fault("finite-window-trace-admits"),
    };
    // The valuation lists name positions by listing index; the evaluator reads
    // canonical positions.
    let canonical = |valuations: &[TimestampedMissingValuation]| -> Vec<(u64, u64)> {
        valuations
            .iter()
            .filter_map(|valuation| {
                let original = input
                    .positions
                    .get(usize::try_from(valuation.position).ok()?)?;
                let position = ordered
                    .iter()
                    .position(|candidate| std::ptr::eq(*candidate, original))?;
                Some((u64::try_from(position).ok()?, valuation.atom))
            })
            .collect()
    };
    // Nodes whose `holds` conditions read the same in the unit are one atom
    // and share one valuation at a position.
    let classes = atom_classes(run, node);
    let same_atom = |valuations: Vec<(u64, u64)>| -> Vec<(u64, u64)> {
        valuations
            .iter()
            .flat_map(|(position, atom)| {
                let class = usize::try_from(*atom)
                    .ok()
                    .and_then(|atom| classes.get(atom).copied());
                classes
                    .iter()
                    .enumerate()
                    .filter(move |(_, candidate)| Some(**candidate) == class)
                    .filter_map(|(index, _)| Some((*position, u64::try_from(index).ok()?)))
                    .collect::<Vec<_>>()
            })
            .collect()
    };
    let absent = same_atom(canonical(&input.missing_valuations));
    let conflicting = same_atom(canonical(&input.conflicting_valuations));
    let observed = evaluate_observed_finite_window(
        run.package,
        node,
        environments,
        &trace,
        over,
        qsl_eval::temporal::WindowValuations {
            absent: &absent,
            conflicting: &conflicting,
            classes: &classes,
        },
        meter,
    );
    match (observed.stopped, observed.result) {
        (Some(HoldsStop::Refused(evaluation)), _) => {
            let disposition = match convert_outcome(evaluation, run.package.graph(), run.sources) {
                Ok(outcome) => ClauseDisposition::Evaluate(outcome),
                Err(refusal) => match *refusal {
                    RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                    other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
                },
            };
            (disposition, 0, None)
        }
        (Some(HoldsStop::Failed(failure)), _) => {
            let disposition = match *convert_call_failure(failure) {
                RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
            };
            (disposition, 0, None)
        }
        (None, Ok(evaluation)) => {
            let names = |atom: qsl_semantics::check::FormulaNodeId| atom_name(run, node, atom);
            let verdict = window_verdict(evaluation.verdict, &names);
            (
                ClauseDisposition::Temporal(verdict),
                evaluation.visits,
                Some(FiniteWindowReport {
                    basis: evaluation.basis,
                    support: evaluation.support,
                    premises: evaluation.premises,
                    identity: evaluation.identity,
                }),
            )
        }
        (None, Err(TemporalRefusal::Fault(internal))) => {
            (ClauseDisposition::EvaluateFault(internal), 0, None)
        }
        (None, Err(refusal)) => (
            ClauseDisposition::TemporalRefusal(trace_refusal(refusal)),
            0,
            None,
        ),
    }
}

/// Each formula node's atom class: the index of the first `holds` node whose
/// condition reads the same in the unit, or the node's own index.
fn atom_classes(
    run: &CompiledRun<'_>,
    node: &qsl_semantics::check::TemporalClauseNode,
) -> Vec<usize> {
    let nodes = node.clause().formula().nodes();
    let text = |index: usize| match nodes.get(index) {
        Some(FormulaNode::Holds { span }) => run
            .sources
            .first()
            .and_then(|source| source.text().get(span.start..span.end)),
        _ => None,
    };
    (0..nodes.len())
        .map(|index| match text(index) {
            Some(own) => (0..index)
                .find(|earlier| text(*earlier) == Some(own))
                .unwrap_or(index),
            None => index,
        })
        .collect()
}

/// The name of the `holds` atom `atom`: the source text of its condition, as
/// the unit wrote it.
fn atom_name(
    run: &CompiledRun<'_>,
    node: &qsl_semantics::check::TemporalClauseNode,
    atom: qsl_semantics::check::FormulaNodeId,
) -> String {
    let FormulaNode::Holds { span } = node
        .clause()
        .formula()
        .nodes()
        .get(atom.index())
        .cloned()
        .unwrap_or(FormulaNode::Constant(false))
    else {
        return format!("node@{}", atom.index());
    };
    run.sources
        .first()
        .and_then(|source| source.text().get(span.start..span.end))
        .map_or_else(|| format!("node@{}", atom.index()), str::to_owned)
}

/// The spine's own form of the finite-window evaluator's verdict
/// (FR-100-AC-8).
fn window_verdict(
    verdict: qsl_eval::temporal::WindowVerdict,
    names: &dyn Fn(qsl_semantics::check::FormulaNodeId) -> String,
) -> TemporalVerdict {
    use qsl_eval::temporal::{IndeterminateReason, WindowVerdict};
    match verdict {
        WindowVerdict::Holds => TemporalVerdict::Holds,
        WindowVerdict::Violated { at } => TemporalVerdict::Violated { at: at.index() },
        WindowVerdict::Pending { at, deadline } => TemporalVerdict::PendingWindow {
            at: at.index(),
            deadline,
        },
        WindowVerdict::Indeterminate { at, reason } => TemporalVerdict::Indeterminate {
            at: at.index(),
            reason: match reason {
                IndeterminateReason::MissingValuation { position, atom } => {
                    TemporalIndeterminateReason::MissingValuation {
                        position: position.index(),
                        atom: names(atom),
                        instance: None,
                        also_held_by: Vec::new(),
                    }
                }
                IndeterminateReason::ConflictingValuation { position, atom } => {
                    TemporalIndeterminateReason::ConflictingValuation {
                        position: position.index(),
                        atom: names(atom),
                    }
                }
                IndeterminateReason::IncompleteInput => {
                    TemporalIndeterminateReason::IncompleteInput
                }
                IndeterminateReason::MissingHistory => TemporalIndeterminateReason::MissingHistory,
                IndeterminateReason::StraddlesBound => TemporalIndeterminateReason::StraddlesBound,
            },
        },
        WindowVerdict::Undefined(undefined) => verdict_of_undefined(undefined),
        WindowVerdict::Incomplete(incomplete) => TemporalVerdict::Incomplete(incomplete),
    }
}

fn verdict_of_undefined(undefined: UndefinedEvaluation) -> TemporalVerdict {
    verdict(TemporalOutcome::Undefined(undefined))
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
        TemporalRefusal::UnorderedTie { position } => {
            TemporalTraceRefusal::SameStampOrderUnavailable {
                position: position.index(),
            }
        }
        TemporalRefusal::Fault(fault) => TemporalTraceRefusal::Fault(fault),
        // The timed evaluator's refusals: the spine reads a clause through
        // the step evaluator only, which never returns them.
        TemporalRefusal::NotTimedProfile { .. }
        | TemporalRefusal::NotFiniteWindowProfile { .. }
        | TemporalRefusal::TraceCarriesNoStamps { .. }
        | TemporalRefusal::NegativeStamp { .. }
        | TemporalRefusal::DecreasingStamp { .. }
        | TemporalRefusal::NonPositiveLoopDelay
        | TemporalRefusal::LoopReentersBeforeLastStamp => TemporalTraceRefusal::Fault(
            quire_semantic_value::catalog::InternalFault::new("temporal", "step-evaluator-refusal"),
        ),
    }
}

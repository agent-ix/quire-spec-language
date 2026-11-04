// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330: the `Temporal` selection of [`super::run_clause`]. It resolves one
//! temporal clause by name, admits each observation of the supplied trace
//! (FR-106), binds the clause's `over` parameter at position 0 and evaluates
//! the clause once through the TemporalTrace evaluator (FR-327, FR-328,
//! FR-329).

use std::collections::BTreeMap;
use std::ops::ControlFlow;

use qsl_eval::temporal::{
    evaluate_observed, FirstUndefined, HoldsStop, PositionRefusal, ProgressedRefusal,
    TemporalOutcome, TemporalPosition, TemporalRefusal, TemporalTrace, TimestampedTrace,
    UndefinedCause, UndefinedEvaluation, WindowRefusal,
};
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{FormulaNode, TemporalProfile};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    admit_trace_snapshot, population_universe_for, AdmissionFailure, AdmissionRecord,
    AdmissionUsage, DocumentRef,
};
use quire_exact::{Meter, ValueType};
use quire_semantic_value::catalog::CatalogCode;

use super::progress::{
    ContradictedDimension, LedgerBinding, ObservedPosition, ProgressAssertion, ProgressClosure,
    ProgressCompleteness, ProgressContradiction, ProgressLedger, ProgressLowerBoundary,
    ProgressValue, Refusal,
};
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
    /// A finite event-position or fixed-sample trace with progress (FR-504).
    Progressed {
        /// The admitted observations.
        trace: Vec<DocumentRef>,
        /// The progress assertion for this request.
        progress: ProgressAssertion,
        /// Whether the first position is an authoritative origin.
        lower_boundary: ProgressLowerBoundary,
        /// The profile, clock binding name and clock parameters the trace
        /// asserts (FR-522), compared with the clause's before any position
        /// is visited.
        binding: TraceBinding,
    },
    /// A timestamped-event trace with progress (FR-501).
    Timestamped {
        /// The observations, in any order: the evaluator reads them in
        /// canonical order (FR-501 "Canonical order") and never consults the
        /// listing.
        observations: Vec<TimestampedObservation>,
        /// The progress assertion for this request.
        progress: ProgressAssertion,
        /// The rest of the finite-window input (FR-330, FR-501 Inputs).
        window: Box<TimestampedWindow>,
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
            Self::Progressed {
                trace, progress, ..
            } => {
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
                ..
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
            Self::Timestamped {
                observations,
                progress,
                window,
            } => Box::new(
                ordered_timestamped_positions(&Timestamped {
                    observations,
                    progress: *progress,
                    window,
                })
                .into_iter()
                .map(|observation| &observation.document),
            ),
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
        "{}:{}{}:{}",
        document.authority.len(),
        document.authority,
        document.identity.len(),
        document.identity,
    )
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

/// One timestamped atom valuation, absent from its observation, named by
/// position and atom.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TimestampedMissingValuation {
    /// Zero-based trace position.
    pub position: u64,
    /// Zero-based checked formula node of the `holds` atom.
    pub atom: u64,
}

/// An admitted order key comparable only within one authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimestampedOrderKey {
    /// Identity of the authority that assigned the key.
    pub authority: String,
    /// Key within that authority's ordered domain.
    pub key: i64,
}

/// FR-330 Inputs: the finite-window input beyond its observations and
/// progress.
#[derive(Clone, Debug)]
pub struct TimestampedWindow {
    /// The profile, clock binding name and clock parameters the trace asserts
    /// (FR-522), required: a trace with no binding is refused at stage
    /// `admit` (FR-330-AC-11).
    pub binding: Option<TraceBinding>,
    /// Whether the first represented position is an authoritative origin
    /// (the lower-boundary kind).
    pub authoritative_origin: bool,
    /// The cover the trace declares for its stamps (FR-501 Inputs); `None` is
    /// `Exact`. The checked clause's clock configuration names the timestamp
    /// unit only, so the cover has no other source.
    pub cover: Option<TimestampedCover>,
    /// Atom valuations absent from an admitted observation (QSpec FR-160):
    /// each is a position, by its index in `observations`, and the zero-based
    /// `holds` formula node of the checked clause.
    pub missing_valuations: Vec<TimestampedMissingValuation>,
}

/// The finite-window input as the evaluator reads it.
struct Timestamped<'a> {
    observations: &'a [TimestampedObservation],
    progress: ProgressAssertion,
    window: &'a TimestampedWindow,
}

/// One timestamped observation in a progress-carrying trace.
#[derive(Clone, Debug)]
pub struct TimestampedObservation {
    /// The admitted snapshot identity.
    pub document: DocumentRef,
    /// The non-negative tick stamp.
    pub stamp: u64,
    /// The order key, when the observation has one: an order-authority
    /// identity and a key within it.
    pub order_key: Option<TimestampedOrderKey>,
}

/// The identity of an observation's document for ordering (FR-106): its two
/// labels joined by NUL, which orders as the labels do field by field.
pub(super) fn observation_identity(document: &DocumentRef) -> String {
    format!("{}\0{}", document.authority, document.identity)
}

/// FR-501 "Canonical order": ascending stamp, then ascending order-authority
/// identity (an observation without an order key after every keyed one), then
/// ascending key within the authority, then ascending observation identity.
/// A total order over distinct observations: the listing order decides
/// nothing.
fn ordered_timestamped_positions<'a>(input: &Timestamped<'a>) -> Vec<&'a TimestampedObservation> {
    let mut positions: Vec<_> = input.observations.iter().collect();
    positions.sort_by(|left, right| {
        fn order(position: &TimestampedObservation) -> Option<(&str, i64)> {
            position
                .order_key
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
    /// The compiled package's `temporal_bindings` rows, keyed by clause
    /// identity (FR-522 "Authentication").
    pub(super) clock_rows: &'a qsl_package::TemporalBindingRows,
    /// The emitted bytes after strict checked-package admission.
    pub(super) admitted_package: &'a quire_contract_model::CheckedPackageV2,
}

/// FR-522 "Authentication": compare a `TraceBinding` against the profile and
/// clock rows recovered from the strictly admitted package. A mismatch is
/// refused at stage `admit`, `invalid_runtime_input`/`invalid-value`, with the
/// first differing dimension and asserted/expected values. The clock row is
/// the one keyed by the declaration's `occurrence`. The checked
/// declaration identity comes from emission's exact `(NodeKey, occurrence)`
/// mapping; the profile and clock are read from that declaration's admitted
/// temporal subject.
fn authenticate_binding(
    binding: Option<&TraceBinding>,
    subject: &qsl_package::checked_views::TemporalSubjectView<'_>,
    occurrence: u64,
) -> Result<crate::spine::AuthenticatedTemporalSelection, Box<ClauseDisposition>> {
    let Some(binding) = binding else {
        return Err(Box::new(refused("invalid-value", BTreeMap::new())));
    };
    let binding = qsl_eval::temporal::evidence::TraceBinding {
        profile: binding.profile.clone().into_boxed_str(),
        clock_name: binding.clock_name.clone().into_boxed_str(),
        parameters: binding.parameters.clone(),
    };
    let authenticated = binding
        .authenticate(&qsl_eval::temporal::evidence::ClauseSelection {
            declaration: subject.declaration(),
            profile: subject.profile(),
            // The clock row is the one keyed by this declaration's occurrence:
            // a node several clauses share has a row per occurrence.
            clock: subject
                .occurrences()
                .iter()
                .find(|found| found.ordinal == occurrence)
                .and_then(|found| subject.clock_of(found)),
        })
        .map_err(|mismatch| {
            let mut fields = BTreeMap::from([
                ("mismatched_dimension", mismatch.dimension.name().to_owned()),
                ("asserted", mismatch.asserted),
            ]);
            if let Some(expected) = mismatch.expected {
                fields.insert("expected", expected);
            }
            Box::new(refused("invalid-value", fields))
        })?;
    Ok(crate::spine::AuthenticatedTemporalSelection {
        declaration: authenticated.declaration,
        source_regions: subject.source_span().regions.clone(),
        profile: authenticated.profile,
        clock_name: authenticated.clock_name,
        clock: authenticated.clock,
    })
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
        clock_rows,
        admitted_package,
    } = *request;
    // A trace is evidence for that trace only: it settles no item and
    // derives no witness (FR-332).
    let report = |disposition: ClauseDisposition,
                  usage: ClauseRunUsage,
                  authenticated_temporal_selection| ClauseRunReport {
        // FR-266's Temporal row.
        basis: match &disposition {
            ClauseDisposition::Temporal(verdict) => verdict.basis(),
            _ => SettlementBasis::Unavailable,
        },
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
        finite_window: None,
        counterexample: None,
        decisive: None,
        authenticated_temporal_selection,
    };
    let early = |disposition| report(disposition, ClauseRunUsage::default(), None);

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
    let Some(declaration) = clock_rows.declaration(&node.identity(), node.claim().ordinal()) else {
        return early(ClauseDisposition::EvaluateFault(
            quire_semantic_value::catalog::InternalFault::new(
                "FR-050",
                "emitted-temporal-declaration-retains-checked-id",
            ),
        ));
    };
    let subject = match qsl_package::checked_views::temporal_subject(admitted_package, declaration)
    {
        Ok(subject) => subject,
        Err(_) => {
            return early(ClauseDisposition::EvaluateFault(
                quire_semantic_value::catalog::InternalFault::new(
                    "FR-050",
                    "admitted-temporal-subject-resolves",
                ),
            ));
        }
    };
    let mut authenticated_temporal_selection = None;

    let trace_matches_profile = match trace {
        TraceInput::Progressed { .. } => matches!(
            clause.profile(),
            TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension
        ),
        TraceInput::Timestamped { .. } => {
            clause.profile() == TemporalProfile::TimestampedEventFiniteWindow
        }
        // The finite-window profile reads a timestamped trace only (FR-330).
        TraceInput::Finite(_) | TraceInput::Lasso { .. } => {
            clause.profile() != TemporalProfile::TimestampedEventFiniteWindow
        }
    };
    if !trace_matches_profile {
        return early(refused("invalid-value", BTreeMap::new()));
    }

    if let TraceInput::Progressed { binding, .. } = trace {
        // FR-522: the same authentication precedes every position of a
        // progressed trace (FR-330-AC-11).
        authenticated_temporal_selection =
            match authenticate_binding(Some(binding), &subject, node.claim().ordinal()) {
                Ok(selection) => Some(selection),
                Err(refusal) => return early(*refusal),
            };
    }

    if let TraceInput::Timestamped {
        observations,
        window,
        ..
    } = trace
    {
        // FR-522: the trace's binding is authenticated before any position is
        // visited, any stamp or completeness is checked or the meter is
        // charged (FR-330-AC-11).
        authenticated_temporal_selection =
            match authenticate_binding(window.binding.as_ref(), &subject, node.claim().ordinal()) {
                Ok(selection) => Some(selection),
                Err(refusal) => return early(*refusal),
            };
        if observations.is_empty() {
            return early(refused("invalid-value", BTreeMap::new()));
        }
        if window.missing_valuations.iter().any(|missing| {
            usize::try_from(missing.position)
                .ok()
                .is_none_or(|position| position >= observations.len())
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

    // FR-504 "Consistency": an assertion the trace contradicts is refused at
    // admit before any snapshot is read, and is never shortened.
    let mut meter = Meter::new(run.accounting);
    let progressed = match trace {
        TraceInput::Progressed {
            trace: documents,
            progress,
            lower_boundary,
            ..
        } => {
            let assertion = qsl_eval::temporal::SampleProgress {
                watermark: progress.watermark,
                completeness: progress.completeness.into(),
                scope: progress.scope.into(),
                execution: progress.execution.into(),
                lower_boundary: match lower_boundary {
                    ProgressLowerBoundary::AuthoritativeOrigin => {
                        qsl_eval::temporal::LowerBoundary::AuthoritativeOrigin
                    }
                    ProgressLowerBoundary::HistoryCutoff => {
                        qsl_eval::temporal::LowerBoundary::HistoryCutoff
                    }
                },
            };
            let identities = documents.iter().map(observation_identity).collect();
            match TimestampedTrace::admit_progressed(
                &qsl_eval::temporal::evidence::ClauseFacts::of_checked(clause),
                identities,
                assertion,
                &mut meter,
            ) {
                Ok(Ok(admitted)) => Some(admitted),
                Ok(Err(inconsistent)) => {
                    let member = match inconsistent {
                        ProgressedRefusal::WatermarkBelowLastPosition => "watermark",
                        ProgressedRefusal::CompletenessBelowLastPosition => "completeness",
                        ProgressedRefusal::NotSampleProfile { .. } => "profile",
                        ProgressedRefusal::Incomplete(incomplete) => {
                            return early(ClauseDisposition::Temporal(
                                TemporalVerdict::Incomplete(incomplete),
                            ));
                        }
                    };
                    return early(refused(
                        "invalid-value",
                        BTreeMap::from([("member", member.to_owned())]),
                    ));
                }
                Err(refusal) => {
                    return early(ClauseDisposition::TemporalRefusal(trace_refusal(refusal)))
                }
            }
        }
        TraceInput::Finite(_) | TraceInput::Timestamped { .. } | TraceInput::Lasso { .. } => None,
    };

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
                    authenticated_temporal_selection.clone(),
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
            authenticated_temporal_selection.clone(),
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
            package: package_identity.identity.clone(),
            clause: node.identity(),
            occurrence: node.claim().ordinal(),
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

    if let Some(window_trace) = progressed {
        let silent_from = u64::try_from(window_trace.observed()).ok();
        let window = run_window(
            run,
            node,
            &window_trace,
            (&[], silent_from),
            &environments,
            reference,
            &mut meter,
        );
        let (disposition, counterexample) =
            with_counterexample(node, trace, window.disposition, window.undefined);
        let accepted = !matches!(
            disposition.category(),
            quire_semantic_value::catalog::Category::Refusal
                | quire_semantic_value::catalog::Category::InternalFailure
        );
        let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
        usage.temporal_visits = window.visits;
        let decisive =
            decisive_information(run, node, trace, &environments, reference, &disposition);
        let mut finished = report(disposition, usage, authenticated_temporal_selection.clone());
        finished.finite_window = window.report;
        finished.counterexample = counterexample;
        finished.decisive = decisive;
        if accepted {
            finished.ledger = accepted_ledger.or_else(|| ledger.cloned());
        }
        return finished;
    }
    if let TraceInput::Timestamped {
        observations,
        progress,
        window,
    } = trace
    {
        let view = Timestamped {
            observations,
            progress: *progress,
            window,
        };
        let window = run_finite_window(run, node, &view, &environments, reference, &mut meter);
        let (disposition, counterexample) =
            with_counterexample(node, trace, window.disposition, window.undefined);
        let accepted = !matches!(
            disposition.category(),
            quire_semantic_value::catalog::Category::Refusal
                | quire_semantic_value::catalog::Category::InternalFailure
        );
        let mut usage = ClauseRunUsage::from_meter(&meter, admitted);
        usage.temporal_visits = window.visits;
        let decisive =
            decisive_information(run, node, trace, &environments, reference, &disposition);
        let mut finished = report(disposition, usage, authenticated_temporal_selection.clone());
        finished.finite_window = window.report;
        finished.counterexample = counterexample;
        finished.decisive = decisive;
        if accepted {
            finished.ledger = accepted_ledger.or_else(|| ledger.cloned());
        }
        return finished;
    }
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
    let (disposition, counterexample) =
        with_counterexample(node, trace, disposition, observed.undefined);
    let accepted = !matches!(
        disposition.category(),
        quire_semantic_value::catalog::Category::Refusal
            | quire_semantic_value::catalog::Category::InternalFailure
    );
    let decisive = decisive_information(run, node, trace, &environments, reference, &disposition);
    let mut report = report(
        disposition,
        ClauseRunUsage::from_meter(&meter, admitted),
        authenticated_temporal_selection,
    );
    report.counterexample = counterexample;
    report.decisive = decisive;
    if accepted {
        report.ledger = accepted_ledger.or_else(|| ledger.cloned());
    }
    report
}

/// FR-501: evaluate the finite-window clause `node` over the admitted
/// `environments`, one per observation in canonical order, and report its
/// verdict with its basis, decision support, premises and result identity.
fn run_finite_window(
    run: &CompiledRun<'_>,
    node: &qsl_semantics::check::TemporalClauseNode,
    input: &Timestamped<'_>,
    environments: &[ObjectEnvironment],
    over: &quire_exact::ObjectReference,
    meter: &mut Meter,
) -> WindowRun {
    use qsl_eval::temporal::{
        LowerBoundary, TimestampedObservation as WindowObservation, Watermark, WindowClock,
        WindowProgress,
    };
    let fault = |invariant: &'static str| {
        WindowRun::stopped(ClauseDisposition::EvaluateFault(
            quire_semantic_value::catalog::InternalFault::new("temporal", invariant),
        ))
    };
    let clause = node.clause();
    let ordered = ordered_timestamped_positions(input);
    let trace = match TimestampedTrace::admit(
        clause.name(),
        WindowClock {
            unit: clause.clock().to_owned(),
            cover: match input.window.cover.unwrap_or(TimestampedCover::Exact) {
                TimestampedCover::Exact => qsl_eval::temporal::ClockCover::Exact,
                TimestampedCover::Counter { uncertainty } => {
                    qsl_eval::temporal::ClockCover::Counter { uncertainty }
                }
            },
        },
        ordered
            .iter()
            .map(|position| WindowObservation {
                stamp: position.stamp,
                order_key: position
                    .order_key
                    .as_ref()
                    .map(|order| qsl_eval::temporal::OrderKey {
                        authority: order.authority.clone(),
                        key: order.key,
                    }),
                observation: observation_identity(&position.document),
            })
            .collect(),
        WindowProgress {
            watermark: Watermark::Inclusive(input.progress.watermark),
            completeness: input.progress.completeness.into(),
            scope: input.progress.scope.into(),
            execution: input.progress.execution.into(),
            lower_boundary: if input.window.authoritative_origin {
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
                    .observations
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
    let absent = same_atom(canonical(&input.window.missing_valuations));
    run_window(
        run,
        node,
        &trace,
        (&absent, None),
        environments,
        over,
        meter,
    )
}

/// FR-501, FR-504: evaluate `clause` over the admitted `trace` and report its
/// verdict with its basis, decision support, premises and result identity.
/// `valuations` are the `(position, atom)` valuations absent from the
/// observations and the first silent sample.
fn run_window(
    run: &CompiledRun<'_>,
    node: &qsl_semantics::check::TemporalClauseNode,
    trace: &qsl_eval::temporal::TimestampedTrace,
    (absent, silent_from): (&[(u64, u64)], Option<u64>),
    environments: &[ObjectEnvironment],
    over: &quire_exact::ObjectReference,
    meter: &mut Meter,
) -> WindowRun {
    let classes = atom_classes(run, node);
    let observed = qsl_eval::temporal::evaluate_observed_finite_window(
        run.package,
        node,
        environments,
        trace,
        over,
        qsl_eval::temporal::WindowValuations {
            absent,
            classes: &classes,
            silent_from,
        },
        meter,
    );
    let undefined = observed.undefined;
    match (observed.stopped, observed.result) {
        (Some(HoldsStop::Refused(evaluation)), _) => {
            let disposition = match convert_outcome(evaluation, run.package.graph(), run.sources) {
                Ok(outcome) => ClauseDisposition::Evaluate(outcome),
                Err(refusal) => match *refusal {
                    RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                    other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
                },
            };
            WindowRun::stopped(disposition)
        }
        (Some(HoldsStop::Failed(failure)), _) => {
            let disposition = match *convert_call_failure(failure) {
                RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
            };
            WindowRun::stopped(disposition)
        }
        (None, Ok(evaluation)) => {
            let names = |atom: qsl_semantics::check::FormulaNodeId| atom_name(run, node, atom);
            let qsl_eval::temporal::WindowEvaluation {
                verdict,
                basis,
                support,
                premises,
                identity,
                visits,
            } = evaluation;
            let Some(verdict) = window_verdict(verdict, &names, guard_location(node)) else {
                return WindowRun::stopped(ClauseDisposition::EvaluateFault(
                    quire_semantic_value::catalog::InternalFault::new(
                        "temporal",
                        "unresolved-guard-has-a-guard",
                    ),
                ));
            };
            WindowRun {
                disposition: ClauseDisposition::Temporal(verdict),
                visits,
                report: Some(finite_window_report(
                    run,
                    node,
                    (basis, support, premises, identity),
                )),
                undefined,
            }
        }
        (None, Err(TemporalRefusal::Fault(internal))) => {
            WindowRun::stopped(ClauseDisposition::EvaluateFault(internal))
        }
        (None, Err(refusal)) => {
            WindowRun::stopped(ClauseDisposition::TemporalRefusal(trace_refusal(refusal)))
        }
    }
}

/// What a finite-window or progressed run returns.
struct WindowRun {
    /// The run's disposition.
    disposition: ClauseDisposition,
    /// The `temporal.visit` charges made.
    visits: u64,
    /// FR-501's result report, for a verdict.
    report: Option<FiniteWindowReport>,
    /// The first undefined reading.
    undefined: Option<FirstUndefined>,
}

impl WindowRun {
    /// A run that stopped with `disposition` before any verdict.
    fn stopped(disposition: ClauseDisposition) -> Self {
        Self {
            disposition,
            visits: 0,
            report: None,
            undefined: None,
        }
    }
}

/// FR-330 Outputs, FR-283 `decisive`: the decisive information of a
/// temporal violation. The deciding atom is the first `holds` atom, in
/// clause-node order, that evaluates false at the failing position over the
/// observation there; FR-107's evaluator, its witness channel kept, gives
/// the quantifier that decided it, its element, index and value path, or,
/// when no quantifier decides it, the atom's value `false` at the empty path
/// at the observation's root. `None` for every other disposition, when no
/// atom evaluates false at a position with an observation (a silent sample
/// has none), and when the evaluation of an atom fails before one is found
/// false or the deciding atom's record cannot be built: the search never
/// passes a failure to report a later atom that did not decide. The atoms
/// share one meter of their own, so the search does one run budget of work
/// and the run's usage is the evaluator's alone.
fn decisive_information(
    run: &CompiledRun<'_>,
    node: &qsl_semantics::check::TemporalClauseNode,
    trace: &TraceInput,
    environments: &[ObjectEnvironment],
    over: &quire_exact::ObjectReference,
    disposition: &ClauseDisposition,
) -> Option<crate::TemporalDecisive> {
    use qsl_eval::value::CheckedPackageEvaluation as _;
    use quire_eval::FamilyOutcome;
    use quire_exact::{Outcome, Value};
    let ClauseDisposition::Temporal(TemporalVerdict::Violated { at }) = disposition else {
        return None;
    };
    let slot = observation_slot(trace, usize::try_from(*at).ok()?)?;
    let document = trace.documents().nth(slot)?;
    let objects = environments.get(slot)?;
    let observation = qsl_eval::value::ObservationIdentity {
        authority: document.authority.clone(),
        identity: document.identity.clone(),
    };
    let formula = node.clause().formula();
    let atoms = formula
        .nodes()
        .iter()
        .enumerate()
        .filter(|(_, formula)| matches!(formula, FormulaNode::Holds { .. }))
        .filter_map(|(index, _)| formula.node_id(index));
    let mut meter = Meter::new(run.accounting);
    first_false_atom(atoms, |atom| {
        let Ok(evaluation) = run.package.evaluate_holds_traced(
            node,
            atom,
            over,
            objects,
            observation.clone(),
            &mut meter,
        ) else {
            return ControlFlow::Break(None);
        };
        if !matches!(
            evaluation.evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
        ) {
            return ControlFlow::Continue(());
        }
        let decided = node
            .condition(atom)
            .and_then(|(condition, _)| crate::witness::decisive_stop(condition, &evaluation).ok());
        ControlFlow::Break(match decided {
            Some(Some(report)) => Some(crate::TemporalDecisive::from_stop(report)),
            Some(None) => evaluation
                .receiver_path()
                .cloned()
                .map(crate::TemporalDecisive::undecided),
            None => None,
        })
    })
}

/// The trace slot holding the observation at position `at`: a lasso's
/// position past its prefix wraps into its loop.
pub(super) fn observation_slot(trace: &TraceInput, at: usize) -> Option<usize> {
    match trace {
        TraceInput::Lasso { prefix, repeat } if at >= prefix.len() => {
            let offset = (at - prefix.len()).checked_rem(repeat.len())?;
            Some(prefix.len() + offset)
        }
        TraceInput::Lasso { .. }
        | TraceInput::Finite(_)
        | TraceInput::Progressed { .. }
        | TraceInput::Timestamped { .. } => Some(at),
    }
}

/// The record of the first atom `read` stops at, in `atoms` order. `read`
/// continues past an atom that is not false and stops at the first that is
/// false or whose evaluation fails, with that atom's record or `None`.
pub(super) fn first_false_atom<A, T>(
    atoms: impl IntoIterator<Item = A>,
    read: impl FnMut(A) -> ControlFlow<Option<T>>,
) -> Option<T> {
    atoms.into_iter().try_for_each(read).break_value().flatten()
}

/// `disposition` with its FR-330 counterexample. A violation or an
/// `Undefined` that is a violation whose counterexample the run cannot build
/// is an internal failure: FR-267 writes no such verdict without one.
fn with_counterexample(
    node: &qsl_semantics::check::TemporalClauseNode,
    trace: &TraceInput,
    disposition: ClauseDisposition,
    undefined: Option<FirstUndefined>,
) -> (
    ClauseDisposition,
    Option<crate::ObservedTemporalCounterexample>,
) {
    let needed = matches!(
        &disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Violated { .. })
    ) || matches!(
        &disposition,
        ClauseDisposition::Temporal(TemporalVerdict::Undefined { cause, .. })
            if !matches!(cause, TemporalUndefinedCause::InsufficientData { .. })
    );
    match counterexample(node, trace, &disposition, undefined) {
        Some(counterexample) => (disposition, Some(counterexample)),
        None if needed => (
            ClauseDisposition::EvaluateFault(quire_semantic_value::catalog::InternalFault::new(
                "temporal",
                "violation-has-its-counterexample",
            )),
            None,
        ),
        None => (disposition, None),
    }
}

/// The first `Steps` interval of the clause's operators, in operator order.
/// A time-bounded operator's interval names ticks, not trace positions, and
/// FR-364's observed arm carries none.
fn step_interval(
    clause: &qsl_semantics::check::CheckedTemporalClause,
) -> Option<qsl_foundation::bound::TemporalInterval> {
    clause
        .operators()
        .iter()
        .find_map(|operator| match operator.interval.as_ref() {
            Some(qsl_semantics::check::CheckedInterval::Steps(interval)) => Some(*interval),
            _ => None,
        })
}

/// FR-330 Outputs: the observed-arm counterexample (QSpec FR-364) of a
/// `Completed(false)` or of an `Undefined` that is a violation, built from what
/// the entry ran: one `{document}` step per represented position (for an
/// `Undefined` result, the positions 0 to the failing position and no loop),
/// the clause's fairness set, the clause's first `Steps` interval (none for
/// an `Undefined` result) and the trace position. `None` for every other
/// disposition, and for an `Undefined` result whose first undefined reading
/// the evaluator did not record at its position.
fn counterexample(
    node: &qsl_semantics::check::TemporalClauseNode,
    trace: &TraceInput,
    disposition: &ClauseDisposition,
    undefined: Option<FirstUndefined>,
) -> Option<crate::ObservedTemporalCounterexample> {
    use crate::{ObservedTemporalCounterexample, ObservedTemporalKind, ObservedTemporalStep};
    let clause = node.clause();
    let documents: Vec<&DocumentRef> = trace.documents().collect();
    let step = |document: &DocumentRef| ObservedTemporalStep {
        document: document.clone(),
    };
    match disposition {
        ClauseDisposition::Temporal(TemporalVerdict::Violated { at }) => {
            Some(ObservedTemporalCounterexample {
                prefix: documents.iter().map(|document| step(document)).collect(),
                loop_start: match trace {
                    TraceInput::Lasso { prefix, .. } => Some(u64::try_from(prefix.len()).ok()?),
                    _ => None,
                },
                fairness: clause.fairness().to_vec(),
                interval: step_interval(clause),
                kind: ObservedTemporalKind::Formula,
                trace_position: TemporalPosition::new(*at),
            })
        }
        ClauseDisposition::Temporal(TemporalVerdict::Undefined { at, cause })
            if !matches!(cause, TemporalUndefinedCause::InsufficientData { .. }) =>
        {
            let first = undefined.filter(|first| first.position.index() == *at)?;
            // A lasso position past its represented positions re-enters the
            // loop.
            let (prefix_len, loop_len) = match trace {
                TraceInput::Lasso { prefix, repeat } => (prefix.len(), repeat.len()),
                _ => (documents.len(), 0),
            };
            let slot = |index: usize| {
                if index < prefix_len || loop_len == 0 {
                    index
                } else {
                    prefix_len + (index - prefix_len) % loop_len
                }
            };
            let last = usize::try_from(*at).ok()?;
            let prefix = (0..=last)
                .map(|index| documents.get(slot(index)).map(|document| step(document)))
                .collect::<Option<Vec<_>>>()?;
            Some(ObservedTemporalCounterexample {
                prefix,
                loop_start: None,
                fairness: clause.fairness().to_vec(),
                interval: None,
                kind: ObservedTemporalKind::UndefinedEvaluation {
                    locus: first.locus,
                    cause: first.cause,
                },
                trace_position: TemporalPosition::new(*at),
            })
        }
        _ => None,
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

/// The checked location of the clause's activation guard.
fn guard_location(
    node: &qsl_semantics::check::TemporalClauseNode,
) -> Option<quire_semantic_value::location::Location> {
    node.guard().map(|(guard, _)| guard.location().clone())
}

/// The spine's own form of the evaluator's result report (FR-100-AC-8).
fn finite_window_report(
    run: &CompiledRun<'_>,
    node: &qsl_semantics::check::TemporalClauseNode,
    (basis, support, premises, identity): (
        qsl_eval::temporal::Basis,
        Vec<qsl_eval::temporal::SupportPair>,
        qsl_eval::temporal::Premises,
        qsl_eval::temporal::ResultIdentity,
    ),
) -> FiniteWindowReport {
    use qsl_eval::temporal::{Assessment, Basis, ClockCover, LowerBoundary};
    FiniteWindowReport {
        basis: match basis {
            Basis::ClosedScope => SettlementBasis::ClosedScope,
            Basis::DecisiveWitness => SettlementBasis::DecisiveWitness,
            Basis::DecisiveCounterexample => SettlementBasis::DecisiveCounterexample,
            Basis::Unsettled => SettlementBasis::Unsettled,
            Basis::Unavailable => SettlementBasis::Unavailable,
        },
        support: support
            .iter()
            .map(|pair| FiniteWindowSupport {
                position: pair.position.index(),
                atom: atom_name(run, node, pair.atom),
            })
            .collect(),
        premises: FiniteWindowPremises {
            scope: premises.scope.into(),
            execution: premises.execution.into(),
            assessment: match premises.assessment {
                Assessment::Completed => FiniteWindowAssessment::Completed,
                Assessment::Failed => FiniteWindowAssessment::Failed,
            },
            completeness: premises.completeness.into(),
            watermark: premises.watermark,
            authoritative_origin: premises.lower_boundary == LowerBoundary::AuthoritativeOrigin,
            cover: match premises.cover {
                ClockCover::Exact => TimestampedCover::Exact,
                ClockCover::Counter { uncertainty } => TimestampedCover::Counter { uncertainty },
            },
            unit: premises.unit,
        },
        identity: identity.0,
    }
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
    guard: Option<quire_semantic_value::location::Location>,
) -> Option<TemporalVerdict> {
    use qsl_eval::temporal::{IndeterminateReason, WindowVerdict};
    Some(match verdict {
        WindowVerdict::Holds => TemporalVerdict::Holds,
        WindowVerdict::Violated { at } => TemporalVerdict::Violated { at: at.index() },
        WindowVerdict::Pending { at, deadline } => TemporalVerdict::PendingWindow {
            at: at.index(),
            deadline,
        },
        WindowVerdict::PendingNextEvent { at } => {
            TemporalVerdict::PendingNextEvent { at: at.index() }
        }
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
        // An unresolved guard comes from the clause's own guard: a clause
        // without one broke an evaluator invariant.
        WindowVerdict::ActivationIncomplete { at } => TemporalVerdict::ActivationIncomplete {
            at: at.index(),
            guard: guard?,
        },
    })
}

fn verdict_of_undefined(undefined: UndefinedEvaluation) -> TemporalVerdict {
    verdict(TemporalOutcome::Undefined(undefined))
}

/// The declared cover of a trace's stamps (FR-501 Inputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimestampedCover {
    /// A stamp is the instant itself.
    Exact,
    /// A counter reading `k` covers `[k, k+1)` widened by `uncertainty` ticks
    /// on each side.
    Counter {
        /// The widening `u`, in ticks.
        uncertainty: u64,
    },
}

/// Whether the evaluator produced an outcome (FR-501 "Result report").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FiniteWindowAssessment {
    /// The evaluator produced an outcome.
    Completed,
    /// The evaluator returned `Incomplete` or a refusal.
    Failed,
}

/// One `(position, atom)` pair of a decision support.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct FiniteWindowSupport {
    /// The position, in canonical order.
    pub position: u64,
    /// The atom: the source text of its `holds` condition.
    pub atom: String,
}

/// The four independent dimensions and the inputs that qualify a finite-window
/// verdict (FR-501 "Result report").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteWindowPremises {
    /// The decision scope's closure.
    pub scope: ProgressClosure,
    /// The surrounding execution's closure.
    pub execution: ProgressClosure,
    /// The assessment-execution disposition.
    pub assessment: FiniteWindowAssessment,
    /// The input completeness.
    pub completeness: ProgressCompleteness,
    /// The watermark as an inclusive tick, `-1` when progress covers none.
    pub watermark: i128,
    /// Whether the first position is an authoritative origin (the
    /// lower-boundary kind).
    pub authoritative_origin: bool,
    /// The clock cover.
    pub cover: TimestampedCover,
    /// The clock unit.
    pub unit: String,
}

/// FR-330, FR-501 "Result report": the basis, decision support, premises and
/// result identity of a finite-window run, as separate fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteWindowReport {
    /// What the verdict rests on.
    pub basis: SettlementBasis,
    /// The decision support, as `(position, atom)` pairs.
    pub support: Vec<FiniteWindowSupport>,
    /// The premises.
    pub premises: FiniteWindowPremises,
    /// The result identity: a content identity over the inputs.
    pub identity: String,
}

/// The spine's own form of the evaluator's outcome (FR-100-AC-8).
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

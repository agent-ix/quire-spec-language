// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-6's request adapter for release-backed schedulability claims (FR-249).

use fsl_foundation::source::provenance::OccurrenceKey;
use fsl_foundation::RequestIndex;
use fsl_route::Mode;
use fsl_semantics::check::{Capability, DeclarationFamily, ScheduleClaim};

use super::{AnalyzeItem, EngineReport, Refusal};
/// What the symbolic pass and the checker made of a closed task-product search.
#[derive(Debug)]
pub struct TaskProof {
    /// How the proof attempt ended.
    pub verdict: ProofVerdict,
    /// The certificate wire the checker read, when one was built.
    pub certificate: Option<serde_json::Value>,
}

/// How a task-product proof attempt ended.
#[derive(Debug)]
pub enum ProofVerdict {
    /// FR-418's checker returned this verdict for the certificate.
    Checked(fsl_replay::ZoneCertificateVerdict),
    /// The set is outside `closed-finite-product/v1`, or a miss or an
    /// overflow is reachable: no certificate states a proof.
    Uncertified,
    /// A limit or a cancel stopped the pass.
    Stopped(fsl_replay::IncompleteCause),
    /// The engine or the checker could not run: a tool fault, never a verdict.
    Fault,
}

/// The bounded task-product result retained for FR-621 settlement.
#[derive(Debug)]
pub struct TaskProductReport {
    /// The symbolic proof attempt, made once the concrete search closed.
    pub proof: Option<TaskProof>,
    /// EN-6's task-product search result.
    pub outcome: crate::zone_check::TaskProductSearchOutcome,
    /// FR-364 counterexample wire, populated only after the task trace replays.
    pub counterexample: Option<serde_json::Value>,
    /// FSpec FR-331 `task_search` frontier evidence of a bounded stop, without
    /// its `identity`; present only for a stop that retained a frontier.
    pub task_search: Option<serde_json::Value>,
    pub(crate) temporal_counterexample: Option<
        Box<fsl_replay::TemporalCounterexample<fsl_eval::simulation::model::TaskProductTransition>>,
    >,
}

/// Build EN-6's task-product item when `claim` names a release-backed
/// schedulability claim. `None` means the checked task set has no releases.
pub(super) fn item<'a>(
    analysis: &'a super::RunInput<'a>,
    binding: &'a super::SubjectBinding<'a>,
    limits: super::AnalyzeLimits,
    max_work: Option<std::num::NonZeroU64>,
    claim: &'a OccurrenceKey,
    index: RequestIndex,
) -> Option<AnalyzeItem<'a>> {
    let task_sets = analysis.package.graph().task_sets();
    let checked_claim = task_sets.claims().iter().find(|checked| {
        matches!(checked.claim, ScheduleClaim::Schedulable(_))
            && checked.occurrence().as_ref() == Some(claim)
    })?;
    let task_set = task_sets.task_set_of(checked_claim)?;
    let releases: Vec<_> = task_sets
        .releases()
        .iter()
        .filter(|release| release.task_set() == task_set.name)
        .collect();
    let first_release = releases.first()?;
    let model = first_release.operation().model();
    if releases
        .iter()
        .any(|release| release.operation().model() != model)
    {
        return None;
    }
    let model = model.to_owned();
    let claim_name = checked_claim.name.clone();
    let checked_claim: &'a fsl_semantics::check::CheckedScheduleClaim = checked_claim;
    let claim = claim.clone();
    Some(AnalyzeItem::new(
        index,
        Capability::Schedulability,
        Some(DeclarationFamily::RealTime),
        Mode::Bounded,
        crate::zone_check::manifest::BACKEND_ID,
        move |cancel| {
            let simulation = super::request::simulation(&analysis.model, binding, &limits);
            let certificate_simulation = simulation.clone();
            let system = match fsl_eval::simulation::model::ModelSystem::new(simulation) {
                Ok(system) => system,
                Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
            };
            let timed = match fsl_eval::simulation::model::TimedSystem::new(&system, &model) {
                Ok(timed) => timed,
                Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
            };
            let product =
                match fsl_eval::simulation::model::TaskProductSystem::new(timed, &claim_name) {
                    Ok(product) => product,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                };
            // The symbolic pass first: the concrete event search cannot close a
            // sporadic or large set, but a proof the checker accepts needs
            // no search. Anything else falls through to the concrete search,
            // which alone can refute.
            let proof = prove(
                &product,
                certificate_simulation,
                &model,
                checked_claim,
                &claim,
                cancel,
            );
            let proved = matches!(
                proof.verdict,
                ProofVerdict::Checked(fsl_replay::ZoneCertificateVerdict::Accepted)
            );
            let outcome = if proved {
                crate::zone_check::TaskProductSearchOutcome::Stopped {
                    cause: crate::zone_check::TaskProductSearchStop::RegionClosureRequired,
                    frontier: Vec::new(),
                    expanded: 0,
                    transitions: 0,
                }
            } else {
                match crate::zone_check::check_task_product_with_cancel(
                    &product,
                    crate::zone_check::TaskProductSearchLimits {
                        max_work: max_work.map_or(DEFAULT_MAX_WORK, std::num::NonZeroU64::get),
                    },
                    cancel,
                ) {
                    Ok(outcome) => outcome,
                    Err(_) => return EngineReport::Refused(Refusal::Failed(BROKEN)),
                }
            };
            if let crate::zone_check::TaskProductSearchOutcome::Stopped {
                cause: crate::zone_check::TaskProductSearchStop::Cancelled(source),
                ..
            } = &outcome
            {
                return EngineReport::Refused(Refusal::Incomplete(
                    fsl_replay::IncompleteCause::Cancelled { source: *source },
                ));
            }
            let reproduced = match &outcome {
                crate::zone_check::TaskProductSearchOutcome::Refuted(refutation) => {
                    task_counterexample(&product, &refutation.trace, &claim)
                }
                crate::zone_check::TaskProductSearchOutcome::Stopped { .. } => None,
            };
            let task_search = match &outcome {
                crate::zone_check::TaskProductSearchOutcome::Stopped {
                    cause: crate::zone_check::TaskProductSearchStop::WorkLimit(work),
                    frontier,
                    ..
                } if !frontier.is_empty() => task_search_evidence(&product, frontier, Some(*work)),
                // Unresolved dynamics carries no claimed limit (FSpec FR-331).
                crate::zone_check::TaskProductSearchOutcome::Stopped {
                    cause:
                        crate::zone_check::TaskProductSearchStop::UnresolvedChoice
                        | crate::zone_check::TaskProductSearchStop::QueueOverflow,
                    frontier,
                    ..
                } if !frontier.is_empty() => task_search_evidence(&product, frontier, None),
                _ => None,
            };
            let counterexample = reproduced
                .as_ref()
                .map(|(_, counterexample)| counterexample.clone());
            let temporal_counterexample = reproduced.map(|(temporal, _)| Box::new(temporal));
            EngineReport::TaskProduct(Box::new(TaskProductReport {
                outcome,
                counterexample,
                task_search,
                temporal_counterexample,
                proof: Some(proof),
            }))
        },
    ))
}

fn task_counterexample(
    product: &fsl_eval::simulation::model::TaskProductSystem<'_, '_>,
    trace: &fsl_eval::simulation::model::TaskProductTrace,
    claim: &OccurrenceKey,
) -> Option<(
    fsl_replay::TemporalCounterexample<fsl_eval::simulation::model::TaskProductTransition>,
    serde_json::Value,
)> {
    use fsl_eval::simulation::model::TaskProductTransition;
    use fsl_replay::{CounterexampleKind, Lasso, ModelStep, TemporalCounterexample};

    // A finite missed-deadline prefix is useful search evidence, but FR-249
    // settles a violation only with FR-364's replay-checked divergent
    // continuation (lasso or terminal stutter). Keep a bare prefix incomplete.
    trace.lasso_entry?;
    let states = product.replay_trace_states(trace).ok()?;
    let terminal_stutter_delay = trace.steps.last().and_then(|step| {
        matches!(
            &step.transition,
            fsl_eval::simulation::model::TaskProductTransition::TerminalStutter
        )
        .then(|| step.delay.clone())
    });
    let prefix_steps = if terminal_stutter_delay.is_some() {
        trace.steps.get(..trace.steps.len().checked_sub(1)?)?
    } else {
        &trace.steps
    };
    let prefix = prefix_steps
        .iter()
        .enumerate()
        .map(|(index, step)| {
            let state = states.get(index + 1)?;
            let post_state = fsl_eval::simulation::StateKey::from_canonical_bytes(
                product.state_key(state).ok()?,
            );
            Some(ModelStep {
                transition: step.transition.clone(),
                post_state,
                delay: Some(step.delay.clone()),
            })
        })
        .collect::<Option<Vec<ModelStep<TaskProductTransition>>>>()?;
    let counterexample = TemporalCounterexample {
        initial_state: trace.initial_state,
        prefix,
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: trace.lasso_entry.map(|entry| Lasso {
            entry,
            terminal_stutter: terminal_stutter_delay.is_some(),
            terminal_stutter_delay,
        }),
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
    };
    // FR-237 re-admission: rebuild the internal task trace from the typed
    // FR-364 counterexample, replay it against the checked product, and
    // compare every supplied post-state key with the recomputed state.
    // This keeps the emitted task transition identities and lasso position
    // tied to the independently replayed subject.
    let wire = fsl_replay::task_temporal_wire(&counterexample, None, Some(claim)).ok()?;
    // The item is a release-backed schedulability item over an FR-249 task
    // product, so its alphabet is the task alphabet (FR-237 "Document form").
    let reproduced =
        fsl_replay::replay_model_trace_document(&wire, fsl_replay::TraceAlphabet::Task, |read| {
            match read {
                fsl_replay::ReadCounterexample::Task(payload) => {
                    readmit_task_counterexample(product, payload)
                }
                fsl_replay::ReadCounterexample::Model(_) => Ok(false),
            }
        })
        .ok()?;
    reproduced.then_some((counterexample, wire))
}

fn readmit_task_counterexample(
    product: &fsl_eval::simulation::model::TaskProductSystem<'_, '_>,
    counterexample: &fsl_replay::TemporalCounterexample<
        fsl_eval::simulation::model::TaskProductTransition,
    >,
) -> Result<bool, fsl_replay::TemporalCounterexampleReadRefusal> {
    let Some(lasso_entry) = counterexample.lasso.as_ref().map(|lasso| lasso.entry) else {
        return Ok(false);
    };
    let miss_positions: Vec<_> = counterexample
        .prefix
        .iter()
        .enumerate()
        .filter_map(|(index, step)| match &step.transition {
            fsl_eval::simulation::model::TaskProductTransition::Task(identity)
                if identity.action
                    == fsl_eval::simulation::model::TaskTransitionAction::DeadlineMiss =>
            {
                Some(index)
            }
            _ => None,
        })
        .collect();
    if miss_positions.len() != 1 || miss_positions[0] >= lasso_entry {
        return Ok(false);
    }
    match fsl_replay::replay_task_product_trace(product, counterexample) {
        Ok(()) => Ok(true),
        Err(fsl_replay::TaskProductTraceReplayRefusal::InvalidRuntimeInput {
            pointer,
            supplied,
            recomputed,
        }) => Err(
            fsl_replay::TemporalCounterexampleReadRefusal::InvalidRuntimeInput {
                pointer,
                supplied,
                recomputed,
            },
        ),
        Err(fsl_replay::TaskProductTraceReplayRefusal::TraceAlphabet { pointer }) => {
            Err(fsl_replay::TemporalCounterexampleReadRefusal::TraceAlphabet { pointer })
        }
        Err(_) => Ok(false),
    }
}

const BROKEN: filament_semantic_value::catalog::CatalogCode =
    filament_semantic_value::catalog::CatalogCode::new(
        "runtime_invariant",
        "established-invariant-broken",
    );

const DEFAULT_MAX_WORK: u64 = 100_000;

/// The FSpec FR-331 `task_search` frontier evidence of a stopped search,
/// without its `identity`, which the result writer adds from the item's
/// certificate identity.
///
/// Nodes are the product states along every frontier path and the frontier
/// states themselves. Each carries its exact clock valuation as a point zone,
/// in the order of the source's clock places followed by each task slot's
/// service and nominal-deadline clocks. Every bound is scaled by one factor,
/// the least common multiple of the valuations' denominators. A path's edges
/// carry the exact transition identities; delays between them are not edges.
/// Returns `None` when a frontier path no longer replays or a state cannot be
/// keyed, so a record is never given evidence that does not replay.
fn task_search_evidence(
    product: &fsl_eval::simulation::model::TaskProductSystem<'_, '_>,
    frontier: &[crate::zone_check::TaskProductSearchFrontier],
    limit: Option<u64>,
) -> Option<serde_json::Value> {
    use crate::zone_check::TaskProductSearchStop as Stop;
    use serde_json::json;

    struct Node {
        key: fsl_eval::simulation::StateKey,
        clocks: Vec<filament_exact::Rational>,
        depth: u64,
        expanded: bool,
    }
    let mut nodes: Vec<Node> = Vec::new();
    #[allow(
        clippy::mutable_key_type,
        reason = "exact rationals cache integer representations; their numeric ordering and equality never mutate"
    )]
    let mut lookup = std::collections::BTreeMap::new();
    let mut add = |state: &fsl_eval::simulation::model::TaskProductState,
                   depth: u64,
                   expanded: bool|
     -> Option<usize> {
        let key =
            fsl_eval::simulation::StateKey::from_canonical_bytes(product.state_key(state).ok()?);
        let clocks = clock_values(product, state);
        // A node is a state at one path length, either expanded or an open
        // leaf, so an edge always runs from depth d to depth d + 1 and an
        // open leaf is never also an expanded node.
        let at = *lookup
            .entry((key.as_bytes().to_vec(), clocks.clone(), depth, expanded))
            .or_insert(nodes.len());
        if at == nodes.len() {
            nodes.push(Node {
                key,
                clocks,
                depth,
                expanded,
            });
        }
        Some(at)
    };
    let mut edges = Vec::new();
    let mut open_nodes = std::collections::BTreeMap::new();
    for open in frontier {
        let states = product.replay_trace_states(&open.trace).ok()?;
        let total = open.trace.steps.len();
        let mut previous = add(states.first()?, 0, total != 0)?;
        for (index, step) in open.trace.steps.iter().enumerate() {
            let depth = u64::try_from(index + 1).ok()?;
            // The path ends at the open node, which is not expanded.
            let next = add(states.get(index + 1)?, depth, index + 1 < total)?;
            let transition = fsl_replay::task_product_transition_wire(&step.transition).ok()?;
            let edge = json!({"node": previous, "transition": transition, "target": next});
            // Frontier paths share prefixes; each edge is written once.
            if !edges.contains(&edge) {
                edges.push(edge);
            }
            previous = next;
        }
        let node = previous;
        let (reason, mut dimensions) = match open.stop {
            Stop::WorkLimit(_) => ("work-limit", vec!["successors"]),
            Stop::UnresolvedChoice => ("unresolved-choice", vec!["demand"]),
            Stop::QueueOverflow => ("unresolved-choice", vec!["queue"]),
            Stop::RegionClosureRequired | Stop::Cancelled(_) => return None,
        };
        // A search that stopped after a delay leaves that delay open.
        if !open.final_delay.is_zero() {
            dimensions.push("delay");
        }
        open_nodes.entry(node).or_insert((reason, dimensions));
    }
    let constants: Vec<_> = nodes
        .iter()
        .flat_map(|node| node.clocks.iter().cloned())
        .collect();
    let scaled = crate::zone_check::scale_constants(&constants);
    let mut cursor = 0;
    let mut written = Vec::with_capacity(nodes.len());
    for node in &nodes {
        let width = node.clocks.len();
        let values = scaled.constants().get(cursor..cursor + width)?;
        cursor += width;
        let bound = |row: usize, column: usize| {
            let at = |index: usize| {
                index
                    .checked_sub(1)
                    .and_then(|clock| values.get(clock))
                    .cloned()
                    .unwrap_or_default()
            };
            json!({"bound": (at(row) - at(column)).to_string(), "strict": false})
        };
        let zone: Vec<Vec<serde_json::Value>> = (0..=width)
            .map(|row| (0..=width).map(|column| bound(row, column)).collect())
            .collect();
        written.push(json!({
            "state": fsl_replay::state_key_wire(&node.key).ok()?,
            "depth": node.depth,
            "zone": zone,
            "expanded": node.expanded,
        }));
    }
    let rows: Vec<_> = open_nodes
        .into_iter()
        .map(|(node, (reason, open))| json!({"node": node, "reason": reason, "open": open}))
        .collect();
    let mut evidence = json!({
        "type": "task-search-frontier",
        "nodes": written,
        "edges": edges,
        "frontier": rows,
    });
    if let Some(value) = limit {
        let limit = fsl_replay::ModelCheckLimit::TaskProductWork;
        evidence["limit"] = json!({
            "type": "limit-reached",
            "limit": limit.as_str(),
            "value": value,
            "setting": limit.setting(),
        });
    }
    Some(evidence)
}

/// The exact clock valuation of a task-product state: the source clock
/// places, then each task slot's service and time since nominal activation.
fn clock_values(
    product: &fsl_eval::simulation::model::TaskProductSystem<'_, '_>,
    state: &fsl_eval::simulation::model::TaskProductState,
) -> Vec<filament_exact::Rational> {
    let zero = filament_exact::Rational::from_integer(filament_exact::Integer::zero());
    let mut clocks = state.source().clocks().values().to_vec();
    for (index, task) in product.plan().tasks.iter().enumerate() {
        for slot in 0..task.max_jobs {
            let live = state
                .jobs()
                .iter()
                .find(|job| job.task_index == index && job.slot == slot);
            clocks.push(live.map_or_else(|| zero.clone(), |job| job.service.clone()));
            clocks
                .push(live.map_or_else(|| zero.clone(), |job| state.now().sub(&job.activated_at)));
        }
    }
    clocks
}

/// The symbolic pass, then FR-418's checker, for a search that closed.
///
/// The engine explores its own refined-region quotient and writes the
/// certificate; the certificate is then spelled as FR-418 JSON, read back,
/// and the checker runs on what it read, so `proved` rests on the bytes
/// emitted and never on the producer's in-memory value.
fn prove<'a>(
    product: &fsl_eval::simulation::model::TaskProductSystem<'_, '_>,
    simulation: fsl_eval::simulation::model::ModelSimulationRequest<'a>,
    model: &str,
    checked_claim: &fsl_semantics::check::CheckedScheduleClaim,
    claim: &OccurrenceKey,
    cancel: &filament_exact::Cancel,
) -> TaskProof {
    use crate::zone_check::{SymbolicStop, TaskCertificateRefusal};
    let fault = || TaskProof {
        verdict: ProofVerdict::Fault,
        certificate: None,
    };
    let limits = fsl_replay::CertificateCheckLimits::default();
    let automaton = fsl_eval::simulation::model::TaskAutomaton::new(product);
    let graph = match crate::zone_check::explore_task_symbolic(
        &automaton,
        limits.max_check_work,
        cancel,
    ) {
        Ok(graph) => graph,
        Err(SymbolicStop::Uncertified) => {
            return TaskProof {
                verdict: ProofVerdict::Uncertified,
                certificate: None,
            }
        }
        Err(SymbolicStop::WorkLimit(value)) => {
            return TaskProof {
                verdict: ProofVerdict::Stopped(fsl_replay::IncompleteCause::LimitReached {
                    limit: fsl_replay::ModelCheckLimit::ZoneMaxCheckWork,
                    value,
                }),
                certificate: None,
            }
        }
        Err(SymbolicStop::Cancelled(source)) => {
            return TaskProof {
                verdict: ProofVerdict::Stopped(fsl_replay::IncompleteCause::Cancelled { source }),
                certificate: None,
            }
        }
        Err(SymbolicStop::Refused(_) | SymbolicStop::Fault(_)) => return fault(),
    };
    let built = match crate::zone_check::task_product_certificate(&automaton, &graph, claim.clone())
    {
        Ok(built) => built,
        Err(
            TaskCertificateRefusal::Missed
            | TaskCertificateRefusal::NoDivergence
            | TaskCertificateRefusal::NoMember,
        ) => {
            return TaskProof {
                verdict: ProofVerdict::Uncertified,
                certificate: None,
            }
        }
        Err(TaskCertificateRefusal::Shape) => return fault(),
    };
    let Ok(identity) = fsl_replay::certificate_identity_wire(claim, model) else {
        return fault();
    };
    let Ok(wire) = built.to_wire(identity.clone()) else {
        return fault();
    };
    let Ok((read, read_identity)) =
        fsl_replay::ZoneCertificate::from_wire(&wire, claim.clone())
    else {
        return fault();
    };
    // Equal by wire spelling: a read transition equals its original only by
    // spelling (`same_source_transition`), never structurally.
    if read_identity != identity || read.to_wire(read_identity.clone()).ok().as_ref() != Some(&wire) {
        return fault();
    }
    let verdict = fsl_replay::check_zone_certificate(
        fsl_replay::ZoneCertificateRequest {
            simulation,
            model,
            max_clocks: fsl_eval::simulation::model::DEFAULT_MAX_CLOCKS,
            item: fsl_replay::CertifiedItem::Schedulability {
                claim: checked_claim,
            },
        },
        claim,
        |obligation: &OccurrenceKey| fsl_replay::CertificateItem::Schedulability(obligation),
        &read,
        limits,
        cancel,
    );
    TaskProof {
        verdict: match verdict {
            Ok(verdict) => ProofVerdict::Checked(verdict),
            Err(_) => ProofVerdict::Fault,
        },
        certificate: Some(wire),
    }
}

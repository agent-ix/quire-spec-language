// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-169 (ADR-022 GV-1 to GV-7, GC-1 to GC-4): the one settlement map from
//! a state-graph item instance's FR-168 outcome to its terminal record.
//!
//! Evidence settles only through its replay (FR-170): the caller's `replay`
//! supplies the result, and the map checks its `package_id`. A `Witnessed`
//! or `Holds` outcome settles `proved` only after the core's own
//! [`check_state_graph`], which the map runs itself against the request's
//! own package, accepts its certificate: a rejected certificate settles
//! `Inconclusive(CertificateRejected{rule, at})` and a stopped check
//! settles `Incomplete`, whatever the replay would give, and a `Witnessed`
//! outcome is replayed only after its certificate is accepted. Every row is
//! built by FR-127's own row builders, so the two maps settle each shared
//! row alike.

use qsl_eval::model_subject::ModelSubject;
use qsl_foundation::digest::DigestRecord;
use qsl_semantics::check::CheckedStateGraphClaim;
use qsl_semantics::library::PackageId;
use quire_exact::Cancel;

use crate::certificate::state_graph::{
    check_state_graph, StateGraphCertificate, StateGraphCertificateRequest,
};
use crate::execute::ReplayRefusal;
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits, ReachedLimit};
use crate::model_check::settle::{Execution, ResultLabel, Row, Truth, EXPLICIT_STATE};
use crate::model_check::state_graph::{
    GraphUndefined, GraphWitness, OpenCause, RunEnd, StateGraphInstanceOutcome, StateGraphOutcome,
    WitnessSource,
};
use crate::proof_result::{
    IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
};
use crate::result::DisagreementCause;

/// How the replay of a state-graph outcome's evidence settled (FR-170).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GraphReplaySettlement {
    /// `reproduced-with-evaluated-witness`: the evidence re-executed and
    /// every predicate it claims evaluated as claimed.
    Reproduced,
    /// `inconclusive`: the replay disagrees with the engine.
    Inconclusive(DisagreementCause),
    /// The replay of a trap's closure stopped at a limit.
    Stopped(ReachedLimit),
}

/// The result of replaying a state-graph outcome's evidence: the settlement
/// and the recompiled `package_id` it was settled against.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphReplayResult {
    /// The settlement.
    pub settlement: GraphReplaySettlement,
    /// The recompiled `package_id`.
    pub package_id: PackageId,
}

/// The method parameters of the run that decided the outcome: what every
/// FR-169 record states beside its value.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StateGraphMethod {
    /// The run's limits, applied or defaulted.
    pub limits: ModelCheckLimits,
    /// The search horizon; `None` when the run set none.
    pub max_depth: Option<u64>,
    /// Phase 0's seed, applied or defaulted.
    pub seed: u64,
}

/// One state-graph item instance's terminal record (FR-169 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphRecord {
    /// The terminal value.
    pub value: TerminalValue,
    /// Its QSpec FR-360 label.
    pub label: ResultLabel,
    /// Its QSpec FR-243 basis.
    pub basis: SettlementBasis,
    /// The execution status.
    pub execution: Execution,
    /// The truth.
    pub truth: Truth,
    /// The settlement method: [`EXPLICIT_STATE`].
    pub method: &'static str,
    /// The search horizon, a method parameter and never a limit (QSpec
    /// FR-392).
    pub max_depth: Option<u64>,
    /// Each limit of the run with the value it used.
    pub limits: [(ModelCheckLimit, u64); 3],
    /// The limit the run, its replay or the certificate check reached.
    pub limit_reached: Option<ReachedLimit>,
    /// Phase 0's seed.
    pub seed: u64,
    /// Phase 0's walks per initial state.
    pub witness_samples: u64,
    /// The phase 0 walks `max_walk_steps` stopped (FR-166).
    pub walks_stopped: u64,
    /// How the run ended, which a `WellDefinednessUnchecked` record states.
    pub end: Option<RunEnd>,
    /// Where and why a claim predicate has no value, which a refuted
    /// `Undefined` record states (ADR-022 GV-7).
    pub undefined: Option<GraphUndefined>,
}

/// FR-169: settle `instance`, decided by a run under `method` over the
/// package `package_id`, as one terminal record. A `Witnessed` or `Holds`
/// outcome's certificate is checked by [`check_state_graph`] over `subject`
/// for `claim`'s instance, after the subject's own package is emitted and
/// its `package_id` equals `package_id` (FR-282; another package settles
/// `Inconclusive(ReplayRefused(stale_dependency))`). Evidence is handed to
/// `replay` (FR-170) and settles by its result: reproduced against
/// `package_id` settles the outcome's row; a result settled against any
/// other package is `ReplayRefused(stale_dependency)`; a disagreement is
/// `ReplayParity`; a stopped trap replay is `Incomplete(LimitReached)`; a
/// refusal is `ReplayRefused`, except an internal fault, which is `Failed`.
pub fn settle_state_graph(
    instance: &StateGraphInstanceOutcome,
    method: &StateGraphMethod,
    package_id: &DigestRecord,
    subject: ModelSubject<'_>,
    claim: &CheckedStateGraphClaim,
    cancel: &Cancel,
    replay: impl FnOnce(&StateGraphInstanceOutcome) -> Result<GraphReplayResult, ReplayRefusal>,
) -> StateGraphRecord {
    let initial = subject.simulation.initial.len();
    let check = Certified {
        request: StateGraphCertificateRequest {
            subject,
            claim,
            over: instance.over.clone(),
        },
        limits: &method.limits,
        package_id,
        cancel,
    };
    let mut end = None;
    let mut undefined = None;
    let row = match &instance.outcome {
        // FR-168 returns one witness per initial state, in order; any other
        // shape is the engine's broken invariant, never a proof.
        StateGraphOutcome::Witnessed { paths, .. }
        | StateGraphOutcome::WitnessedUnchecked { paths, .. }
            if !one_per_initial(paths, initial) =>
        {
            Row::failed()
        }
        StateGraphOutcome::Witnessed { paths, certificate } => check.row(certificate, || {
            replayed(replay(instance), package_id, || {
                Row::proved(
                    ProofBasis::Witness {
                        sources: sources(paths),
                    },
                    SettlementBasis::DecisiveWitness,
                )
            })
        }),
        StateGraphOutcome::Holds { certificate } => check.row(certificate, || {
            Row::proved(ProofBasis::Exhaustive, SettlementBasis::ClosedScope)
        }),
        StateGraphOutcome::WitnessedUnchecked {
            paths,
            end: run_end,
            ..
        } => replayed(replay(instance), package_id, || {
            // Only a `WellDefinednessUnchecked` record states the run's end
            // and the limit that stopped it.
            end = Some(run_end.clone());
            Row {
                limit_reached: match run_end {
                    RunEnd::Stopped { limit, .. } => *limit,
                    RunEnd::Completed => None,
                },
                ..Row::inconclusive(InconclusiveCause::WellDefinednessUnchecked {
                    sources: sources(paths),
                })
            }
        }),
        StateGraphOutcome::Trapped { .. } => replayed(replay(instance), package_id, || {
            Row::refuted(SettlementBasis::ClosedScope)
        }),
        StateGraphOutcome::PathPair { .. } => replayed(replay(instance), package_id, || {
            Row::refuted(SettlementBasis::DecisiveCounterexample)
        }),
        StateGraphOutcome::Undefined {
            undefined: found, ..
        } => replayed(replay(instance), package_id, || {
            undefined = Some(found.clone());
            Row::refuted(SettlementBasis::DecisiveCounterexample)
        }),
        StateGraphOutcome::NoDecision {
            end: RunEnd::Stopped { cause, limit },
            ..
        } => Row::stopped_run(*cause, *limit),
        StateGraphOutcome::NoDecision {
            end: RunEnd::Completed,
            open,
        } => {
            let horizon_only =
                !open.is_empty() && open.iter().all(|cause| *cause == OpenCause::MaxDepth);
            match method.max_depth {
                _ if open.contains(&OpenCause::UndecidedSuccessor) => {
                    Row::inconclusive(InconclusiveCause::UndecidedSuccessor)
                }
                // V-5: the horizon alone left nodes open.
                Some(depth) if horizon_only => {
                    Row::inconclusive(InconclusiveCause::BoundReached { depth })
                }
                // A completed run with no open node decides its claim, a
                // horizon node needs a horizon, and a completed run leaves no
                // node unexpanded: the engine broke an invariant.
                Some(_) | None => Row::failed(),
            }
        }
        StateGraphOutcome::NoInitialState => Row::inconclusive(InconclusiveCause::NoInitialState),
    };
    StateGraphRecord {
        value: row.value,
        label: row.label,
        basis: row.basis,
        execution: row.execution,
        truth: row.truth,
        method: EXPLICIT_STATE,
        max_depth: method.max_depth,
        limits: method.limits.used(),
        limit_reached: row.limit_reached,
        seed: method.seed,
        witness_samples: method.limits.witness_samples,
        walks_stopped: instance.walks_stopped,
        end,
        undefined,
    }
}

/// The core check of a state-graph certificate for one instance.
struct Certified<'r, 'a> {
    request: StateGraphCertificateRequest<'a>,
    limits: &'r ModelCheckLimits,
    package_id: &'r DigestRecord,
    cancel: &'r Cancel,
}

impl Certified<'_, '_> {
    /// `accepted`'s row when the request's own package is the run's and the
    /// core accepts `certificate`; the check's row otherwise.
    fn row(&self, certificate: &StateGraphCertificate, accepted: impl FnOnce() -> Row) -> Row {
        Row::bound(
            self.package_id,
            self.request.subject.simulation.package,
            self.cancel,
            || {
                Row::of_check(
                    check_state_graph(&self.request, certificate, self.limits, self.cancel),
                    accepted,
                )
            },
        )
    }
}

/// The row of a replay `result`: `reproduced`'s when it reproduced the
/// evidence against `package_id`.
fn replayed(
    result: Result<GraphReplayResult, ReplayRefusal>,
    package_id: &DigestRecord,
    reproduced: impl FnOnce() -> Row,
) -> Row {
    match result {
        Ok(GraphReplayResult {
            package_id: replayed,
            ..
        }) if !replayed.matches(package_id) => Row::stale(),
        Ok(GraphReplayResult { settlement, .. }) => match settlement {
            GraphReplaySettlement::Reproduced => reproduced(),
            GraphReplaySettlement::Inconclusive(cause) => {
                Row::inconclusive(InconclusiveCause::ReplayParity(cause))
            }
            GraphReplaySettlement::Stopped(reached) => Row::incomplete(
                IncompleteCause::LimitReached {
                    limit: reached.limit,
                    value: reached.value,
                },
                Some(reached),
            ),
        },
        Err(refusal) => Row::refused(&refusal),
    }
}

/// Each witness's source, in initial-state order.
fn sources(paths: &[GraphWitness]) -> Vec<WitnessSource> {
    paths.iter().map(|witness| witness.source.clone()).collect()
}

/// Whether `paths` holds exactly one witness per initial state of a subject
/// with `initial` states, in initial-state order.
fn one_per_initial(paths: &[GraphWitness], initial: usize) -> bool {
    paths.len() == initial
        && paths.iter().enumerate().all(|(index, witness)| {
            usize::try_from(witness.path.initial).is_ok_and(|at| at == index)
        })
}

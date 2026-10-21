// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-127 (ADR-018 V-1, V-4 to V-7, LA-3): the one settlement map from an
//! EN-1 outcome to a QSpec FR-331 terminal record.
//!
//! A `Violated` outcome settles `refuted` only on a replay result (ADR-018
//! SM-7) that reproduced it against the package the run checked: the
//! caller's `replay` (in production [`replay_model_trace`]) supplies the
//! result, and the map checks its `package_id`. A `Holds` outcome settles
//! only through the core's own closure checker, [`check_closure`], which the
//! map runs itself on the outcome's certificate (ADR-018 PC-3, PC-5):
//! accepted is `Proved{Exhaustive, Certified}`, rejected is
//! `Inconclusive(CertificateRejected{rule, at})`, and a stopped check is
//! `Incomplete` with its exact limit or cancellation source.

use qsl_foundation::diagnostic::{Category, Code};
use qsl_foundation::digest::DigestRecord;
use qsl_package::CheckedPackage;

use quire_exact::Cancel;

use crate::certificate::closure::{
    check_closure, check_protocol_closure, CertificateRejection, CertificateRequest,
    ClosureCertificate, ClosureRefusal, ProtocolCertificateRequest, TemporalCertificateCheck,
};
use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::TemporalCounterexample;
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits, ReachedLimit};
use crate::model_check::outcome::{MemoryCause, ModelCheckOutcome, ReductionCause, UndecidedCause};
use crate::model_check::replay::{
    no_value_cause, ModelTraceReplayResult, ModelTraceSettlement, StrongFairnessHint,
};
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, MemoryBound, MemoryBoundMember, ProofBasis,
    SettlementBasis, StatedMemoryBound, TerminalValue, UnavailabilityCause,
};
use qsl_eval::simulation::protocol::{ProtocolTransition, SubjectMemory, WeakAccessShape};
use quire_semantic_value::model_transition::ModelTransition;

/// The settlement method every EN-1 record states (FR-127).
pub const EXPLICIT_STATE: &str = "explicit-state";

/// QSpec FR-360's result label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultLabel {
    /// `proved`.
    Proved,
    /// `refuted`.
    Refuted,
    /// `inconclusive`.
    Inconclusive,
    /// `unsupported`.
    Unsupported,
    /// `failed`.
    Failed,
}

impl ResultLabel {
    /// The ADR-013 O-16 category a backend's result settles in (FR-332,
    /// ADR-014 A-5): `proved` is success, `refuted` violation, `inconclusive`
    /// inconclusive, `unsupported` unsupported, `failed` with execution
    /// `resource-incomplete` incomplete, and every other `failed` an
    /// internal failure.
    pub fn category(self, execution: Execution) -> Category {
        match (self, execution) {
            (Self::Proved, _) => Category::Success,
            (Self::Refuted, _) => Category::Violation,
            (Self::Inconclusive, _) => Category::Inconclusive,
            (Self::Unsupported, _) => Category::Unsupported,
            (Self::Failed, Execution::ResourceIncomplete) => Category::Incomplete,
            (Self::Failed, Execution::Completed | Execution::Failed) => Category::InternalFailure,
        }
    }
}

/// The record's execution status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Execution {
    /// The method completed.
    Completed,
    /// A limit or a cancellation stopped it: `resource-incomplete`.
    ResourceIncomplete,
    /// The tool failed.
    Failed,
}

/// The record's truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Truth {
    /// The item holds.
    Satisfied,
    /// The item does not hold.
    Violated,
    /// Execution completed and the truth is still pending.
    Pending,
    /// No truth was settled.
    Unavailable,
}

/// One EN-1 item's terminal record (FR-127 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCheckRecord<T = ModelTransition> {
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
    /// The search horizon a V-5 record states.
    pub depth: Option<u64>,
    /// Each limit of the run with the value it used.
    pub limits: [(ModelCheckLimit, u64); 3],
    /// The limit the run reached, when one stopped it.
    pub limit_reached: Option<ReachedLimit>,
    /// The counterexample a `refuted` record carries.
    pub counterexample: Option<TemporalCounterexample<T>>,
    /// The strong constraints that would exclude a `refuted` liveness
    /// lasso (FR-133), each a warning diagnostic beside the counterexample.
    /// They are outside it, its replay identity and the obligation
    /// identity, and change no verdict.
    pub hints: Vec<StrongFairnessHint>,
    /// The memory bound the run used and whether it was reached (ADR-025
    /// MB-2, FR-225); `None` for a subject whose memory model has no bound,
    /// such as one under `sc`.
    pub memory_bound: Option<StatedMemoryBound>,
}

/// FR-127: settle `outcome`, a run under `limits` over the package
/// `package_id`, as one terminal record. A `Holds` outcome settles only
/// after the request's own package is emitted and its `package_id` equals
/// `package_id` (another package settles
/// `Inconclusive(ReplayRefused(stale_dependency))`, one that does not emit
/// settles `failed`, or `Incomplete(Cancelled)` when `cancel` stopped it) and [`check_closure`] checks its certificate for
/// `certificate` under `limits` and `cancel`; a refused check settles
/// `inconclusive` by `NoInstance` or `NoInitialState` and `failed`
/// otherwise, `clause-root-unresolved` among the causes. A `Violated` outcome's
/// counterexample is handed to `replay` (FR-128) and settles by its result:
/// reproduced against `package_id` is `Refuted`; a result settled against
/// any other package is `Inconclusive(ReplayRefused(stale_dependency))`; a
/// disagreement is `Inconclusive(ReplayParity)`; a refusal is
/// `Inconclusive(ReplayRefused)`, except an internal fault, which is
/// `Failed`.
///
/// [`replay_model_trace`]: crate::model_check::replay::replay_model_trace
pub fn settle_model_check(
    outcome: &ModelCheckOutcome,
    limits: &ModelCheckLimits,
    package_id: &DigestRecord,
    certificate: &CertificateRequest<'_>,
    cancel: &Cancel,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<ModelTraceReplayResult, ReplayRefusal>,
) -> ModelCheckRecord {
    settle(
        outcome,
        limits,
        package_id,
        certificate.subject.simulation.package,
        cancel,
        |closure| check_closure(certificate, closure, limits, cancel),
        replay,
    )
}

/// FR-215 and FR-217 (ADR-027 TS-6, PX-1): [`settle_model_check`]'s one
/// settlement map over an outcome of a protocol subject. A `Holds` outcome's
/// certificate is checked by [`check_protocol_closure`] over `certificate`;
/// a `Violated` outcome's counterexample is handed to `replay` (in
/// production [`replay_protocol_trace`]).
///
/// [`replay_protocol_trace`]: crate::model_check::replay::replay_protocol_trace
pub fn settle_protocol_check(
    outcome: &ModelCheckOutcome<ProtocolTransition>,
    limits: &ModelCheckLimits,
    package_id: &DigestRecord,
    certificate: &ProtocolCertificateRequest<'_>,
    cancel: &Cancel,
    replay: impl FnOnce(
        &TemporalCounterexample<ProtocolTransition>,
    ) -> Result<ModelTraceReplayResult, ReplayRefusal>,
) -> ModelCheckRecord<ProtocolTransition> {
    let mut record = settle(
        outcome,
        limits,
        package_id,
        certificate.subject.model.simulation.package,
        cancel,
        |closure| check_protocol_closure(certificate, closure, limits, cancel),
        replay,
    );
    record.memory_bound =
        match SubjectMemory::of(&certificate.subject.memory, certificate.max_store_buffer) {
            SubjectMemory::Sc => None,
            SubjectMemory::Tso(tso) => Some(StatedMemoryBound {
                bound: MemoryBound {
                    member: MemoryBoundMember::MaxStoreBuffer,
                    value: tso.max_store_buffer(),
                },
                // Reached whenever a store was cut: by the engine's run,
                // whatever it settled (a `refuted` run too), or by the
                // closure checker's re-expansion.
                reached: outcome.stats().bound_limited > 0
                    || matches!(
                        record.value,
                        TerminalValue::Inconclusive(InconclusiveCause::MemoryBoundReached { .. })
                    ),
            }),
        };
    record
}

/// FR-439's `weak-access-shape` cause of `shape`: its site, its
/// transition's and its shape's canonical texts; `None` when either has no
/// canonical text.
fn weak_access_shape(shape: &WeakAccessShape) -> Option<UnavailabilityCause> {
    let limits = quire_canonical::Limits::new(u64::MAX);
    let transition = match &shape.transition {
        Some(transition) => Some(
            quire_canonical::to_vec(transition, limits)
                .ok()
                .and_then(|bytes| String::from_utf8(bytes).ok())?,
        ),
        None => None,
    };
    Some(UnavailabilityCause::WeakAccessShape {
        site: shape.site.clone(),
        transition,
        shape: shape.shape_text().ok()?,
    })
}

fn settle<T: Clone>(
    outcome: &ModelCheckOutcome<T>,
    limits: &ModelCheckLimits,
    package_id: &DigestRecord,
    package: &CheckedPackage,
    cancel: &Cancel,
    closure: impl FnOnce(&ClosureCertificate) -> Result<TemporalCertificateCheck, ClosureRefusal>,
    replay: impl FnOnce(&TemporalCounterexample<T>) -> Result<ModelTraceReplayResult, ReplayRefusal>,
) -> ModelCheckRecord<T> {
    let mut counterexample = None;
    let mut hints = Vec::new();
    let mut depth = None;
    let row = match outcome {
        ModelCheckOutcome::Holds { certificate, .. } => {
            Row::bound(package_id, package, cancel, || {
                Row::of_check(closure(certificate), || {
                    Row::proved(ProofBasis::Exhaustive, SettlementBasis::ClosedScope)
                })
            })
        }
        // ADR-021 RV-1: a reduced proof is its own technique, and a
        // constraint-only proof explored the subject without a certificate.
        ModelCheckOutcome::HoldsReduced { reductions, .. } => Row::proof(
            if reductions.is_empty() {
                ProofBasis::Exhaustive
            } else {
                ProofBasis::Reduced {
                    reductions: reductions.clone(),
                }
            },
            Certification::Uncertified,
            SettlementBasis::ClosedScope,
        ),
        ModelCheckOutcome::Reduction { cause, .. } => match cause {
            ReductionCause::Unsupported { reduction } => {
                Row::unsupported(UnavailabilityCause::UnsupportedReduction {
                    reduction: *reduction,
                })
            }
            ReductionCause::NotPreserving { reduction, form } => {
                Row::inconclusive(InconclusiveCause::ReductionNotPreserving {
                    reduction: *reduction,
                    form: *form,
                })
            }
            ReductionCause::SymmetryBroken { population, cause } => {
                Row::inconclusive(InconclusiveCause::SymmetryBroken {
                    population: population.clone(),
                    cause: cause.clone(),
                })
            }
            ReductionCause::ConstraintReached { boundary_states } => {
                Row::inconclusive(InconclusiveCause::ConstraintReached {
                    boundary_states: *boundary_states,
                })
            }
            ReductionCause::Horizon { max_depth } => {
                Row::inconclusive(InconclusiveCause::ReductionHorizon {
                    max_depth: *max_depth,
                })
            }
        },
        ModelCheckOutcome::Memory { cause, .. } => match cause {
            // A transition or shape with no canonical text is a broken
            // invariant: the item fails rather than settle a degraded cause.
            MemoryCause::WeakAccessShape(shape) => match weak_access_shape(shape) {
                Some(cause) => Row {
                    value: TerminalValue::Unsupported(cause),
                    label: ResultLabel::Unsupported,
                    basis: SettlementBasis::Unavailable,
                    execution: Execution::Completed,
                    truth: Truth::Unavailable,
                    limit_reached: None,
                },
                None => Row::failed(),
            },
            MemoryCause::BoundReached { bound, states } => {
                Row::inconclusive(InconclusiveCause::MemoryBoundReached {
                    bound: *bound,
                    states: *states,
                })
            }
        },
        ModelCheckOutcome::BoundReached { depth: reached, .. } => {
            depth = Some(*reached);
            Row::inconclusive(InconclusiveCause::BoundReached { depth: *reached })
        }
        ModelCheckOutcome::Undecided { cause, .. } => Row::inconclusive(match cause {
            UndecidedCause::UndecidedSuccessor => InconclusiveCause::UndecidedSuccessor,
            UndecidedCause::NoInitialState => InconclusiveCause::NoInitialState,
            UndecidedCause::NoInstance => InconclusiveCause::NoInstance,
            UndecidedCause::MatchUndetermined => InconclusiveCause::MatchUndetermined,
            UndecidedCause::VacuousMatch => InconclusiveCause::VacuousMatch,
            UndecidedCause::UncountedLoop => InconclusiveCause::UncountedLoop,
        }),
        ModelCheckOutcome::Stopped { cause, limit, .. } => Row::stopped_run(*cause, *limit),
        ModelCheckOutcome::Violated {
            counterexample: payload,
            ..
        } => match replay(payload) {
            Ok(ModelTraceReplayResult {
                package_id: replayed,
                ..
            }) if !replayed.matches(package_id) => Row::stale(),
            Ok(ModelTraceReplayResult {
                settlement: ModelTraceSettlement::Reproduced { .. },
                hints: replayed_hints,
                ..
            }) => {
                hints = replayed_hints;
                counterexample = Some(payload.as_ref().clone());
                Row::refuted(SettlementBasis::DecisiveCounterexample)
            }
            Ok(ModelTraceReplayResult {
                settlement: ModelTraceSettlement::Inconclusive(cause),
                ..
            }) => Row::inconclusive(InconclusiveCause::ReplayParity(cause)),
            // EN-1 builds only fair lassos (ADR-019 FS-7), so replay's refusal
            // of one as unfair is a disagreement between the two (SV-4).
            Err(ReplayRefusal::UnfairLasso { .. }) => {
                Row::inconclusive(InconclusiveCause::ReplayParity(no_value_cause()))
            }
            Err(refusal) => Row::refused(&refusal),
        },
    };
    ModelCheckRecord {
        value: row.value,
        label: row.label,
        basis: row.basis,
        execution: row.execution,
        truth: row.truth,
        method: EXPLICIT_STATE,
        depth,
        limits: limits.used(),
        limit_reached: row.limit_reached,
        counterexample,
        hints,
        memory_bound: None,
    }
}

/// One row of an EN-1 settlement map: the terminal value with its label,
/// basis, execution and truth, and the limit a stopped row names. FR-127's
/// map and FR-169's build their records from it, so the two settle each
/// shared row alike.
pub(super) struct Row {
    pub(super) value: TerminalValue,
    pub(super) label: ResultLabel,
    pub(super) basis: SettlementBasis,
    pub(super) execution: Execution,
    pub(super) truth: Truth,
    pub(super) limit_reached: Option<ReachedLimit>,
}

impl Row {
    /// `proved` on `basis`, `Certified`: a core check stands behind it.
    pub(super) fn proved(basis: ProofBasis, settlement: SettlementBasis) -> Self {
        Self::proof(basis, Certification::Certified, settlement)
    }

    /// `proved` on `basis` with `certification`.
    pub(super) fn proof(
        basis: ProofBasis,
        certification: Certification,
        settlement: SettlementBasis,
    ) -> Self {
        Self {
            value: TerminalValue::Proved {
                basis,
                certification,
            },
            label: ResultLabel::Proved,
            basis: settlement,
            execution: Execution::Completed,
            truth: Truth::Satisfied,
            limit_reached: None,
        }
    }

    /// `refuted` on `basis`.
    pub(super) fn refuted(basis: SettlementBasis) -> Self {
        Self {
            value: TerminalValue::Refuted,
            label: ResultLabel::Refuted,
            basis,
            execution: Execution::Completed,
            truth: Truth::Violated,
            limit_reached: None,
        }
    }

    /// `unsupported`, `unavailable`, for `cause`.
    pub(super) fn unsupported(cause: UnavailabilityCause) -> Self {
        Self {
            value: TerminalValue::Unsupported(cause),
            label: ResultLabel::Unsupported,
            basis: SettlementBasis::Unavailable,
            execution: Execution::Completed,
            truth: Truth::Unavailable,
            limit_reached: None,
        }
    }

    /// `inconclusive`, `unsettled`, for `cause`.
    pub(super) fn inconclusive(cause: InconclusiveCause) -> Self {
        Self {
            value: TerminalValue::Inconclusive(cause),
            label: ResultLabel::Inconclusive,
            basis: SettlementBasis::Unsettled,
            execution: Execution::Completed,
            truth: Truth::Pending,
            limit_reached: None,
        }
    }

    /// `failed`, `resource-incomplete`, for `cause`, naming the limit
    /// reached.
    pub(super) fn incomplete(cause: IncompleteCause, limit_reached: Option<ReachedLimit>) -> Self {
        Self {
            value: TerminalValue::Incomplete(cause),
            label: ResultLabel::Failed,
            basis: SettlementBasis::Unavailable,
            execution: Execution::ResourceIncomplete,
            truth: Truth::Unavailable,
            limit_reached,
        }
    }

    /// `failed`: the tool itself failed.
    pub(super) fn failed() -> Self {
        Self {
            value: TerminalValue::Failed,
            label: ResultLabel::Failed,
            basis: SettlementBasis::Unavailable,
            execution: Execution::Failed,
            truth: Truth::Unavailable,
            limit_reached: None,
        }
    }

    /// Evidence settled against another package than the run's:
    /// `ReplayRefused(stale_dependency)`.
    pub(super) fn stale() -> Self {
        Self::inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency))
    }

    /// A refused replay: `ReplayRefused` with its code, or `failed` for a
    /// fault.
    pub(super) fn refused(refusal: &ReplayRefusal) -> Self {
        match refusal {
            ReplayRefusal::Fault(_) => Self::failed(),
            refusal => Self::inconclusive(InconclusiveCause::ReplayRefused(refusal.code())),
        }
    }

    /// A run that `cause` stopped at `limit`: a limit stop names it as
    /// `LimitReached`, any other stop keeps its cause.
    pub(super) fn stopped_run(cause: IncompleteCause, limit: Option<ReachedLimit>) -> Self {
        let cause = match (cause, limit) {
            (IncompleteCause::ResourceExhausted, Some(reached)) => IncompleteCause::LimitReached {
                limit: reached.limit,
                value: reached.value,
            },
            (cause, _) => cause,
        };
        Self::incomplete(cause, limit)
    }

    /// The row of a certificate check: `accepted`'s when the core accepts
    /// the certificate, `CertificateRejected{rule, at}` when it rejects it,
    /// an `Incomplete` row naming the limit when a limit or cancellation
    /// stops it; a refused check settles `NoInstance` or `NoInitialState`,
    /// and `failed` otherwise.
    pub(super) fn of_check(
        check: Result<TemporalCertificateCheck, ClosureRefusal>,
        accepted: impl FnOnce() -> Self,
    ) -> Self {
        match check {
            Ok(TemporalCertificateCheck::Accepted) => accepted(),
            Ok(TemporalCertificateCheck::Rejected(CertificateRejection { rule, at })) => {
                Self::inconclusive(InconclusiveCause::CertificateRejected { rule, at })
            }
            Ok(TemporalCertificateCheck::Stopped(cause)) => {
                let limit_reached = match cause {
                    IncompleteCause::LimitReached { limit, value } => {
                        Some(ReachedLimit { limit, value })
                    }
                    IncompleteCause::TimedOut
                    | IncompleteCause::Cancelled { .. }
                    | IncompleteCause::ResourceExhausted => None,
                };
                Self::incomplete(cause, limit_reached)
            }
            Ok(TemporalCertificateCheck::MemoryBoundReached { bound, states }) => {
                Self::inconclusive(InconclusiveCause::MemoryBoundReached { bound, states })
            }
            Err(ClosureRefusal::NoInstance) => Self::inconclusive(InconclusiveCause::NoInstance),
            Err(ClosureRefusal::NoInitialState) => {
                Self::inconclusive(InconclusiveCause::NoInitialState)
            }
            Err(_) => Self::failed(),
        }
    }

    /// FR-282's package binding of a certificate: `check`'s row only when
    /// `package`, the checker's own package, emits and its `package_id`
    /// equals `package_id`, the package the run checked. Another package
    /// settles `ReplayRefused(stale_dependency)`; a package that does not
    /// emit settles `failed`, or `Incomplete(Cancelled)` when `cancel`
    /// stopped it.
    pub(super) fn bound(
        package_id: &DigestRecord,
        package: &CheckedPackage,
        cancel: &Cancel,
        check: impl FnOnce() -> Self,
    ) -> Self {
        match emitted_package_id(package, cancel) {
            Some(checked) if checked != *package_id => Self::stale(),
            Some(_) => check(),
            None => match cancel.cause() {
                Some(source) => Self::incomplete(IncompleteCause::Cancelled { source }, None),
                None => Self::failed(),
            },
        }
    }
}

/// The `package_id` of `package`, by emitting it (FR-322) under `cancel`;
/// `None` when it does not emit or the handle stopped it.
fn emitted_package_id(package: &CheckedPackage, cancel: &Cancel) -> Option<DigestRecord> {
    qsl_package::emit_checked_with_cancel(package, cancel)
        .ok()
        .map(|emission| emission.package().package_id().record())
}

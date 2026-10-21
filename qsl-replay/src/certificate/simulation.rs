// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-149 (ADR-020 CT-1 to CT-3, ADR-018 PC-2, PC-5, ADR-029 CB-2): the
//! simulation certificate EN-1 returns with a holding refinement safety half,
//! and [`check_simulation_certificate`], the core's checker of it.
//!
//! The checker reads nothing from the engine but the certificate. It
//! re-opens the refinement's two `ModelSystem`s with [`open_systems`], runs
//! [`check_initial`] on every concrete initial state, recomputes the concrete
//! successors of every position and runs [`check_step`] on every one of them.
//! It applies its rules in this order and rejects at the first failing
//! position in index order:
//!
//! 1. identity: a certificate that names another refinement is refused
//!    `stale_dependency`/`content-mismatch` ([`SimulationRefusal::Stale`]);
//! 2. `Malformed`, before any recomputation: the lists do not match in
//!    length, or a position has no state because no initial state and no
//!    earlier edge reaches it;
//! 3. `InitialMismatch`: an initial state does not pass with the listed
//!    position;
//! 4. per position: `BadState` (a contract of the concrete model is
//!    undecided, so its successors are not known), `SuccessorsDiffer`, then
//!    per edge `StepFails` and `PostDiffers`.
//!
//! The positions count against `max_states` and the edges against
//! `max_transitions`, and a `check_step` that a budget stops ends the check
//! `Stopped`, never `Rejected`.
//!
//! Identity of the packages and the obligation is FR-144's, which refuses a
//! certificate before this checker runs; the checker reads the refinement
//! node only, so the certificate carries the refinement's declared name and
//! no package or obligation identity yet. A position carries the key of its
//! `HistoryValues` (FR-138), so product states that differ only in history
//! are different positions. The `UndefinedEvaluation`
//! verdict writes its `where` and `cause` as kinds only; FR-127's full
//! encoding of them is remaining work on QSL-549.

use qsl_eval::simulation::model::{
    check_model_bounds, ClauseOutcome, ModelFinding, ModelSimState, ModelSimulationRequest,
    ModelSystem,
};
use qsl_eval::simulation::{TransitionSystem, BOUND_POSITIONS};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::digest::DigestRecord;
use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{Cancel, LimitKind};
use quire_semantic_value::model_transition::ModelTransition;
use serde::{Serialize, Serializer};

use crate::certificate::check::{
    expansion_halt, finish, poll, stopped, widen, Budgets, Checker, Halt, Step,
};
use crate::certificate::closure::DigestWire;
use crate::certificate::closure::{CertificateRejection, ClosureRefusal, TemporalCertificateCheck};
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::model_check::counterexample::{UndefinedEvaluation, UndefinedLocus};
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits};
use crate::refinement::{
    check_initial, check_step, open_systems, ConcreteStep, EvalFailure, RefinementFailure,
    RefinementPosition, RefinementSystems, StepRejection, StepVerdict, SubjectRefusal, Taken,
    Undetermined,
};

/// One position of a simulation certificate: a product state EN-1 stored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifiedPosition {
    /// The concrete state's `quire.simulation.state-key/v1` digest.
    pub concrete: DigestRecord,
    /// The refinement's history values at it, as their typed canonical key
    /// (QSpec FR-378's encoding): two positions with one concrete state and
    /// different history are different positions.
    pub history: Vec<u8>,
    /// The abstract states it stands for, as digests, in the order
    /// [`check_step`] gives them.
    pub abstract_states: Vec<DigestRecord>,
}

/// One edge a simulation certificate retains.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifiedEdge {
    /// The concrete transition; `None` is the terminal stutter step of a
    /// state with no successor (ADR-018 SM-4).
    pub transition: Option<ModelTransition>,
    /// The index of the post position in `positions`.
    pub post: u64,
    /// What [`check_step`] returned for the edge.
    pub taken: Taken,
}

/// FR-149's simulation certificate of one refinement item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimulationCertificate {
    /// The refinement's declared name.
    pub refinement: String,
    /// Every stored product position, in the order EN-1 stored them.
    pub positions: Vec<CertifiedPosition>,
    /// The position of each concrete initial state, in the order of the
    /// subject's initial states.
    pub initial: Vec<u64>,
    /// The edges out of each position, in FR-120's canonical order.
    pub successors: Vec<Vec<CertifiedEdge>>,
}

/// The limit an incomplete evaluation ran out of.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IncompleteLimit {
    /// The evaluation meter's work units.
    EvaluationMeter,
    /// Another counter of the evaluation's scalar limits.
    Counter(LimitKind),
}

impl IncompleteLimit {
    fn of(limit: LimitKind) -> Self {
        match limit {
            LimitKind::WorkUnits => Self::EvaluationMeter,
            other => Self::Counter(other),
        }
    }

    /// The limit's FR-126 spelling; another counter keeps its member name.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::EvaluationMeter => ModelCheckLimit::EvaluationMeter.as_str(),
            Self::Counter(kind) => kind.as_str(),
        }
    }
}

/// The evaluation outcome that left a read undetermined (FR-149 Outputs).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluationOutcome {
    /// A row or contract was refused, or could not be decided.
    Refused,
    /// An evaluation ran out of a limit.
    Incomplete(IncompleteLimit),
}

impl EvaluationOutcome {
    /// The outcome of `cause`: a mapping read's evaluation, or a successor
    /// set that could not be decided.
    fn of_undetermined(cause: &Undetermined) -> Self {
        match cause {
            Undetermined::Mapping(mapping) => Self::of_mapping(&mapping.cause),
            Undetermined::UndecidedSuccessor => Self::Refused,
        }
    }

    /// The outcome of a mapping read's `cause`.
    fn of_mapping(cause: &EvalFailure) -> Self {
        match cause {
            EvalFailure::Evaluation(outcome) => Self::of_clause(outcome),
            EvalFailure::UnmappedType(_) | EvalFailure::OutOfType => Self::Refused,
        }
    }

    /// The outcome of a contract clause's evaluation.
    fn of_clause(outcome: &ClauseOutcome) -> Self {
        match outcome {
            ClauseOutcome::Incomplete(incomplete) => {
                Self::Incomplete(IncompleteLimit::of(incomplete.limit_kind))
            }
            _ => Self::Refused,
        }
    }

    /// Whether the evaluation only ran out of the evaluation meter.
    pub fn is_meter_only(self) -> bool {
        self == Self::Incomplete(IncompleteLimit::EvaluationMeter)
    }
}

/// What [`check_step`] gave an edge it did not pass: FR-149's locus
/// `verdict` for `StepFails`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailedStep {
    /// The refinement does not hold at the edge.
    Fails(RefinementFailure),
    /// A mapping row or step-row argument evaluated undefined.
    Undefined(Box<UndefinedEvaluation>),
    /// The check could not decide.
    Undetermined(UndeterminedStep),
}

/// An `Undetermined` verdict with the evaluation outcome that caused it. The
/// outcome is derived from the cause, so the pair is always one the checker
/// can produce.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndeterminedStep {
    cause: Undetermined,
    evaluation: EvaluationOutcome,
}

impl UndeterminedStep {
    /// The step of `cause`.
    pub fn new(cause: Undetermined) -> Self {
        let evaluation = EvaluationOutcome::of_undetermined(&cause);
        Self { cause, evaluation }
    }

    /// FR-141's cause.
    pub fn cause(&self) -> &Undetermined {
        &self.cause
    }

    /// The evaluation outcome that caused it.
    pub fn evaluation(&self) -> EvaluationOutcome {
        self.evaluation
    }
}

impl FailedStep {
    /// The verdict as the locus writes it (FR-149 "Wire form").
    fn wire(&self) -> Result<serde_json::Value, String> {
        use serde_json::json;
        let decimal = |value: u64| value.to_string();
        Ok(match self {
            Self::Fails(RefinementFailure::InitialNotAbstract { initial }) => {
                json!({"kind": "InitialNotAbstract", "initial": initial.to_string()})
            }
            Self::Fails(RefinementFailure::StutterChanged { position }) => {
                json!({"kind": "StutterChanged", "position": decimal(*position)})
            }
            Self::Fails(RefinementFailure::AbstractStepRejected {
                position,
                transition,
                cause,
            }) => json!({
                "kind": "AbstractStepRejected",
                "position": decimal(*position),
                "transition": transition_wire(transition)?,
                "cause": match cause {
                    StepRejection::Precondition { clause } => {
                        json!({"kind": "Precondition", "clause": clause})
                    }
                    StepRejection::Frame { code } => {
                        json!({"kind": "Frame", "code": code.to_string()})
                    }
                    StepRejection::Postcondition => json!({"kind": "Postcondition"}),
                },
            }),
            Self::Fails(RefinementFailure::NoAbstractMatch { position }) => {
                json!({"kind": "NoAbstractMatch", "position": decimal(*position)})
            }
            // `check_step` never returns this form; the engine writes it.
            Self::Fails(RefinementFailure::Undefined { position, .. }) => {
                json!({"kind": "Undefined", "position": decimal(*position)})
            }
            // The liveness half has no certificate, so no step check writes
            // these.
            Self::Fails(
                RefinementFailure::Divergence { .. } | RefinementFailure::AbstractUnfair { .. },
            ) => return Err("a step check fails with a safety kind".to_owned()),
            Self::Undefined(evaluation) => json!({
                "kind": "UndefinedEvaluation",
                "where": {
                    "position": decimal(evaluation.site.position),
                    "state_key": DigestWire::of(&evaluation.site.state_key),
                    "locus": locus_kind(&evaluation.site.locus),
                },
                "cause": undefined_cause_kind(&evaluation.cause),
            }),
            Self::Undetermined(UndeterminedStep { cause, evaluation }) => json!({
                "kind": "Undetermined",
                "cause": match cause {
                    Undetermined::Mapping(_) => "MappingUndetermined",
                    Undetermined::UndecidedSuccessor => "UndecidedSuccessor",
                },
                "evaluation": match evaluation {
                    EvaluationOutcome::Refused => json!({"outcome": "refused"}),
                    EvaluationOutcome::Incomplete(limit) => {
                        json!({"outcome": "incomplete", "limit": limit.as_str()})
                    }
                },
            }),
        })
    }
}

impl Serialize for FailedStep {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.wire()
            .map_err(serde::ser::Error::custom)?
            .serialize(serializer)
    }
}

/// QSpec FR-364's encoding of a transition identity: the RFC 8785 identity
/// document QSpec FR-181 defines.
fn transition_wire(transition: &ModelTransition) -> Result<serde_json::Value, String> {
    let bytes = transition
        .canonical_bytes(quire_canonical::Limits::new(u64::MAX))
        .map_err(|refusal| format!("a transition identity encodes: {refusal:?}"))?;
    serde_json::from_slice(&bytes).map_err(|error| error.to_string())
}

/// The kind of expression an undefined evaluation names.
fn locus_kind(locus: &UndefinedLocus) -> &'static str {
    match locus {
        UndefinedLocus::Atom(_) => "atom",
        UndefinedLocus::Contract(_) => "contract",
        UndefinedLocus::Terminal(_) => "terminal",
        UndefinedLocus::Constraint(_) => "constraint",
        UndefinedLocus::Protocol(_) => "protocol",
        UndefinedLocus::Mapping(_) => "mapping",
        UndefinedLocus::ClaimPredicate(_) => "claim-predicate",
    }
}

/// The kind of the undefined value's cause.
fn undefined_cause_kind(cause: &UndefinedCause) -> &'static str {
    match cause {
        UndefinedCause::Kernel(_) => "kernel",
        UndefinedCause::Family(_) => "family",
        UndefinedCause::InsufficientData(_) => "insufficient-data",
    }
}

/// What [`check_simulation_certificate`] checks: the refinement and its
/// subjects, re-opened by the checker itself.
pub struct SimulationCertificateRequest<'a> {
    /// The checked refinement (FR-135).
    pub refinement: &'a CheckedRefinement,
    /// The concrete subject's request, under the meter the check evaluates
    /// each clause with.
    pub concrete: ModelSimulationRequest<'a>,
    /// The package that holds the abstract model.
    pub abstract_package: &'a CheckedPackage,
    /// The abstract subject's FR-106 initial snapshots.
    pub abstract_initial: Vec<DocumentRef>,
}

/// Why [`check_simulation_certificate`] checked nothing: the request, not
/// the certificate's rules, is at fault.
#[derive(Debug, thiserror::Error)]
pub enum SimulationRefusal {
    /// The certificate names another refinement than the request's.
    #[error("stale_dependency/content-mismatch: the certificate names refinement {certificate} but the request is for {request}")]
    Stale {
        /// The refinement the certificate names.
        certificate: String,
        /// The request's refinement.
        request: String,
    },
    /// A subject did not open, the subject has no initial state, a state
    /// has no canonical encoding, or an invariant broke.
    #[error(transparent)]
    Subject(#[from] ClosureRefusal),
}

/// FR-149: check `certificate` for `request`'s refinement under `limits` and
/// the concrete subject's own evaluation meter and candidate bound, polling
/// `cancel` at every position and edge.
///
/// # Errors
///
/// [`SimulationRefusal`] when the certificate names another refinement, a
/// subject does not open, a root is unbounded, or the concrete subject has no
/// initial state. A refusal is never an acceptance.
pub fn check_simulation_certificate(
    request: &SimulationCertificateRequest<'_>,
    certificate: &SimulationCertificate,
    limits: &ModelCheckLimits,
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, SimulationRefusal> {
    if certificate.refinement != request.refinement.name() {
        return Err(SimulationRefusal::Stale {
            certificate: certificate.refinement.clone(),
            request: request.refinement.name().to_owned(),
        });
    }
    let budgets = Budgets::of(&request.concrete);
    let RefinementSystems {
        concrete,
        abstract_system,
    } = open_systems(
        request.refinement,
        &request.concrete,
        request.abstract_package,
        request.abstract_initial.clone(),
    )
    .map_err(|refusal| match refusal {
        SubjectRefusal::Admission(failure) => ClosureRefusal::Admission(failure),
        SubjectRefusal::NoModels
        | SubjectRefusal::NoUniverse
        | SubjectRefusal::ProtocolAbstractSide => {
            Checker::Simulation.fault("the-subjects-of-a-refinement-open")
        }
    })?;
    for system in [&concrete, &abstract_system] {
        check_model_bounds(system, BOUND_POSITIONS)
            .map_err(|refusal| ClosureRefusal::NotSimulated(Box::new(refusal)))?;
    }
    if concrete.initial_states().is_empty() {
        return Err(ClosureRefusal::NoInitialState.into());
    }
    let mut run = Run {
        refinement: request.refinement,
        concrete: &concrete,
        abstract_system: &abstract_system,
        certificate,
        limits,
        cancel,
        budgets,
        known: vec![None; certificate.positions.len()],
        edges: 0,
    };
    Ok(finish(run.run())?)
}

/// A position the check has reached: the concrete state that stands for it,
/// and the position the first passing check gave it.
#[derive(Clone)]
struct Known {
    state: ModelSimState,
    position: RefinementPosition,
}

struct Run<'r, 'a> {
    refinement: &'r CheckedRefinement,
    concrete: &'r ModelSystem<'a>,
    abstract_system: &'r ModelSystem<'a>,
    certificate: &'r SimulationCertificate,
    limits: &'r ModelCheckLimits,
    cancel: &'r Cancel,
    budgets: Budgets,
    /// Each position's state, once an initial state or an edge reaches it.
    known: Vec<Option<Known>>,
    /// The edges checked so far.
    edges: usize,
}

impl Run<'_, '_> {
    fn reject(
        rule: CertificateRule,
        position: u64,
        edge: Option<u64>,
        verdict: Option<Box<FailedStep>>,
    ) -> Halt {
        Halt::Check(TemporalCertificateCheck::Rejected(CertificateRejection {
            rule,
            at: CertificateLocus::SimulationStep {
                position,
                edge,
                verdict,
            },
        }))
    }

    fn fault(invariant: &'static str) -> Halt {
        Halt::Refusal(Checker::Simulation.fault(invariant))
    }

    fn poll(&self) -> Step<()> {
        poll(self.cancel)
    }

    /// The stop of a verdict, or a `BadState` finding, that only ran out of
    /// the evaluation meter (PC-2: a certificate is rejected only when the
    /// checker found it wrong).
    fn meter_stop(&self) -> Halt {
        stopped(
            ModelCheckLimit::EvaluationMeter,
            self.concrete.meter().work_units,
        )
    }

    /// Whether `undetermined` only ran out of the evaluation meter.
    fn meter_only(undetermined: &Undetermined) -> bool {
        EvaluationOutcome::of_undetermined(undetermined).is_meter_only()
    }

    fn encoding<T>(result: Result<T, qsl_eval::simulation::EncodingRefusal>) -> Step<T> {
        result.map_err(Halt::from)
    }

    /// Whether the certificate's position `listed` is `position`.
    fn matches(&self, listed: &CertifiedPosition, position: &RefinementPosition) -> Step<bool> {
        let (_, concrete) = Self::encoding(self.concrete.state_key_of(&position.concrete))?;
        if listed.concrete != concrete {
            return Ok(false);
        }
        if listed.history != Self::encoding(position.history.key(self.concrete))? {
            return Ok(false);
        }
        let mut digests = Vec::with_capacity(position.abstract_states.len());
        for state in &position.abstract_states {
            digests.push(Self::encoding(self.abstract_system.state_key_of(state))?.1);
        }
        Ok(listed.abstract_states == digests)
    }

    fn run(&mut self) -> Step<()> {
        self.poll()?;
        let certificate = self.certificate;
        let positions = widen(certificate.positions.len());
        if certificate.successors.len() != certificate.positions.len() {
            return Err(Self::reject(
                CertificateRule::Malformed,
                widen(
                    certificate
                        .successors
                        .len()
                        .min(certificate.positions.len()),
                ),
                None,
                None,
            ));
        }
        if certificate.initial.len() != self.concrete.initial_states().len() {
            return Err(Self::reject(
                CertificateRule::Malformed,
                widen(certificate.initial.len()),
                None,
                None,
            ));
        }
        self.reached()?;
        if certificate.positions.len() > self.limits.limits.max_states {
            return Err(stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            ));
        }
        self.initial()?;
        for at in 0..positions {
            self.poll()?;
            self.position(at)?;
        }
        Ok(())
    }

    /// `Malformed`, at the first position that no initial state reaches and
    /// no edge listed at a lower-indexed position reaches.
    fn reached(&self) -> Step<()> {
        let mut reached = vec![false; self.certificate.positions.len()];
        for &position in &self.certificate.initial {
            if let Some(slot) = usize::try_from(position)
                .ok()
                .and_then(|at| reached.get_mut(at))
            {
                *slot = true;
            }
        }
        for (at, edges) in self.certificate.successors.iter().enumerate() {
            if !reached[at] {
                return Err(Self::reject(
                    CertificateRule::Malformed,
                    widen(at),
                    None,
                    None,
                ));
            }
            for edge in edges {
                let later = usize::try_from(edge.post)
                    .ok()
                    .filter(|post| *post > at)
                    .and_then(|post| reached.get_mut(post));
                if let Some(slot) = later {
                    *slot = true;
                }
            }
        }
        Ok(())
    }

    /// `InitialMismatch`: each concrete initial state passes `check_initial`
    /// with the position the certificate lists for it.
    fn initial(&mut self) -> Step<()> {
        let initial = self.concrete.initial_states().to_vec();
        for (index, state) in initial.iter().enumerate() {
            self.poll()?;
            let Some(&listed) = self.certificate.initial.get(index) else {
                return Err(Self::fault("every-initial-state-lists-a-position"));
            };
            let mismatch = || Self::reject(CertificateRule::InitialMismatch, listed, None, None);
            let verdict = check_initial(
                self.refinement,
                self.concrete,
                self.abstract_system,
                index,
                state.state(),
            );
            let StepVerdict::Passes { post, .. } = verdict else {
                return Err(self.unsound(&verdict, mismatch()));
            };
            let slot = usize::try_from(listed)
                .ok()
                .and_then(|at| self.certificate.positions.get(at).map(|entry| (at, entry)));
            let Some((at, entry)) = slot else {
                return Err(mismatch());
            };
            if !self.matches(entry, &post)? {
                return Err(mismatch());
            }
            self.known[at].get_or_insert(Known {
                state: state.clone(),
                position: post,
            });
        }
        Ok(())
    }

    /// The end of a check that did not pass: a stop, a fault or a meter that
    /// ran out ends the check as itself, any other verdict is `otherwise`.
    fn unsound(&self, verdict: &StepVerdict, otherwise: Halt) -> Halt {
        match verdict {
            StepVerdict::Stopped(reached) => stopped(reached.limit, reached.value),
            StepVerdict::Fault(_) => Self::fault("refinement-step"),
            StepVerdict::Undetermined(undetermined) if Self::meter_only(undetermined) => {
                self.meter_stop()
            }
            _ => otherwise,
        }
    }

    /// The checks of position `at`: its successors, then each edge.
    fn position(&mut self, at: u64) -> Step<()> {
        let index = usize::try_from(at).map_err(|_| Self::fault("a-position-indexes"))?;
        let Some(known) = self.known.get(index).cloned().flatten() else {
            return Err(Self::fault("a-reached-position-has-a-state"));
        };
        let expansion = self
            .concrete
            .successors(&known.state)
            .map_err(|stop| expansion_halt(&stop, self.budgets, Checker::Simulation))?;
        // FR-120 turns an incomplete clause set into an expansion stop, so a
        // contract finding never carries an incomplete evaluation: any one
        // leaves the successors unknown.
        if expansion
            .findings
            .iter()
            .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
        {
            return Err(Self::reject(CertificateRule::BadState, at, None, None));
        }
        let mut computed = Vec::with_capacity(expansion.successors.len());
        for (transition, state) in expansion.successors {
            let order = Self::encoding(self.concrete.transition_key(&transition))?;
            let (key, _) = Self::encoding(self.concrete.state_key(&state))?;
            computed.push((order, key, Some(transition), state));
        }
        computed.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        if computed.is_empty() {
            // ADR-018 SM-4: a terminal state repeats forever, one stutter
            // step that is no model step.
            computed.push((Vec::new(), Vec::new(), None, known.state.clone()));
        }
        let listed = self
            .certificate
            .successors
            .get(index)
            .ok_or_else(|| Self::fault("every-position-lists-its-edges"))?;
        let same = listed.len() == computed.len()
            && listed
                .iter()
                .zip(&computed)
                .all(|(edge, (_, _, transition, _))| edge.transition == *transition);
        if !same {
            return Err(Self::reject(
                CertificateRule::SuccessorsDiffer,
                at,
                None,
                None,
            ));
        }
        for (number, (edge, (_, _, transition, state))) in listed.iter().zip(&computed).enumerate()
        {
            self.poll()?;
            if self.edges >= self.limits.limits.max_transitions {
                return Err(stopped(
                    ModelCheckLimit::MaxTransitions,
                    widen(self.limits.limits.max_transitions),
                ));
            }
            self.edges = self.edges.saturating_add(1);
            let number = widen(number);
            let verdict = check_step(
                self.refinement,
                self.concrete,
                self.abstract_system,
                &known.position,
                &ConcreteStep {
                    position: edge.post,
                    transition: transition.as_ref(),
                    post: state,
                },
            )
            .verdict;
            let failed = |verdict| {
                Self::reject(
                    CertificateRule::StepFails,
                    at,
                    Some(number),
                    Some(Box::new(verdict)),
                )
            };
            let (post, taken) = match verdict {
                StepVerdict::Passes { post, taken } => (post, taken),
                StepVerdict::Fails(failure) => return Err(failed(FailedStep::Fails(failure))),
                StepVerdict::Undefined(evaluation) => {
                    return Err(failed(FailedStep::Undefined(Box::new(evaluation))))
                }
                StepVerdict::Undetermined(undetermined) => {
                    if Self::meter_only(&undetermined) {
                        return Err(self.meter_stop());
                    }
                    return Err(failed(FailedStep::Undetermined(UndeterminedStep::new(
                        undetermined,
                    ))));
                }
                StepVerdict::Stopped(reached) => return Err(stopped(reached.limit, reached.value)),
                StepVerdict::Fault(_) => return Err(Self::fault("refinement-step")),
            };
            let differs = || Self::reject(CertificateRule::PostDiffers, at, Some(number), None);
            let slot = usize::try_from(edge.post).ok().and_then(|post_at| {
                self.certificate
                    .positions
                    .get(post_at)
                    .map(|entry| (post_at, entry))
            });
            let Some((post_at, entry)) = slot else {
                return Err(differs());
            };
            if edge.taken != taken || !self.matches(entry, &post)? {
                return Err(differs());
            }
            self.known[post_at].get_or_insert(Known {
                state: state.clone(),
                position: post,
            });
        }
        Ok(())
    }
}

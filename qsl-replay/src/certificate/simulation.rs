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
//! What this does not do yet. FR-149's `concrete_package`, `abstract_package`
//! and `obligation` identity fields are not carried, because S4 emits no
//! refinement and so no `package_id` or obligation identity exists for one
//! (FR-145 "the facade"); the refinement is identified by its declared name.
//! `HistoryValues` are not carried, because history fields are refused
//! (FR-138). The terminal stutter edge of ADR-018 SM-4 is a listed edge whose
//! `transition` is `None`, which FR-149's `CertifiedEdge` does not name.

use qsl_eval::simulation::model::{
    check_model_bounds, ClauseOutcome, ModelFinding, ModelSimState, ModelSimulationRequest,
    ModelSystem,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::digest::DigestRecord;
use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::Cancel;
use quire_semantic_value::model_transition::ModelTransition;

use crate::certificate::check::{
    expansion_halt, finish, poll, stopped, widen, Budgets, Checker, Halt, Step, BOUND_POSITIONS,
};
use crate::certificate::closure::{CertificateRejection, ClosureRefusal, TemporalCertificateCheck};
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::model_check::counterexample::UndefinedEvaluation;
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits};
use crate::refinement::{
    check_initial, check_step, open_systems, ConcreteStep, EvalFailure, RefinementFailure,
    RefinementPosition, RefinementSystems, StepVerdict, SubjectRefusal, Taken, Undetermined,
};

/// One position of a simulation certificate: a product state EN-1 stored.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertifiedPosition {
    /// The concrete state's `quire.simulation.state-key/v1` digest.
    pub concrete: DigestRecord,
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

/// What [`check_step`] gave an edge it did not pass: FR-149's locus
/// `verdict` for `StepFails`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FailedStep {
    /// The refinement does not hold at the edge.
    Fails(RefinementFailure),
    /// A mapping row or step-row argument evaluated undefined.
    Undefined(Box<UndefinedEvaluation>),
    /// The check could not decide.
    Undetermined(Undetermined),
}

impl FailedStep {
    /// The verdict's kind as the locus writes it.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Fails(RefinementFailure::InitialNotAbstract { .. }) => "initial-not-abstract",
            Self::Fails(RefinementFailure::StutterChanged { .. }) => "stutter-changed",
            Self::Fails(RefinementFailure::AbstractStepRejected { .. }) => "abstract-step-rejected",
            Self::Fails(RefinementFailure::NoAbstractMatch { .. }) => "no-abstract-match",
            Self::Fails(RefinementFailure::Undefined { .. }) | Self::Undefined(_) => {
                "undefined-evaluation"
            }
            Self::Undetermined(_) => "undetermined",
        }
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
        SubjectRefusal::NoModels | SubjectRefusal::NoUniverse => {
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

    /// Whether `undetermined` is a mapping row that only ran out of meter.
    fn meter_only(undetermined: &Undetermined) -> bool {
        matches!(
            undetermined,
            Undetermined::Mapping(mapping)
                if matches!(
                    &mapping.cause,
                    EvalFailure::Evaluation(outcome)
                        if matches!(outcome.as_ref(), ClauseOutcome::Incomplete(_))
                )
        )
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
            return Err(Self::reject(CertificateRule::Malformed, at, None, None));
        };
        let expansion = self
            .concrete
            .successors(&known.state)
            .map_err(|stop| expansion_halt(&stop, self.budgets, Checker::Simulation))?;
        let mut undetermined = expansion
            .findings
            .iter()
            .filter_map(|finding| match finding {
                ModelFinding::ContractUndetermined { evaluation, .. } => Some(evaluation),
                _ => None,
            });
        if let Some(evaluation) = undetermined.next() {
            // A contract that only ran out of meter is a stop, not a verdict
            // on the certificate (PC-2).
            if matches!(evaluation, ClauseOutcome::Incomplete(_)) {
                return Err(self.meter_stop());
            }
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
                    return Err(failed(FailedStep::Undetermined(undetermined)));
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

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-069 (ADR-013 O-24): the typed proof-result envelope. Reads one FR-331
//! `quire.backend-provider/v1` terminal record into exactly one of the
//! seven ADR-013 O-16 outcome categories the proof column produces.
//!
//! The category is the one `qsl_foundation` [`Category`] (FR-285). The
//! proof column never produces `undefined`: an undefined claim evaluation
//! settles `refuted`, category violation.

use crate::bounds::{BoundExceeded, ReplayLimits};
use crate::call_site::CallSiteRefusal;
use crate::certificate::closed_form::outcome::ClosedFormAnalysis;
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::execute::ReplayRefusal;
use crate::hybrid::{SolverHorizon, SolverMethod};
use crate::model_check::limits::ModelCheckLimit;
use crate::model_check::state_graph::WitnessSource;
use crate::result::{DisagreementCause, ReplayResult, WitnessSettlement};
use crate::scalar::ScalarAgreement;
use qsl_eval::simulation::{Permutation, SymmetryClass};
use qsl_eval::temporal::TemporalOutcome;
use qsl_foundation::diagnostic::{Category, Code};
use qsl_foundation::BackendId;
use qsl_foundation::RequestIndex;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_contract_model::Std001Code;
use quire_exact::CancelCause;
use quire_semantic_value::fairness::{FairnessConstraint, PropertyForm};
use serde::Serialize;

/// FR-127: how a proof established its claim, independently of certification.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ProofBasis {
    /// Kani's completed SUCCESS checks; zero denotes a vacuous proof.
    #[serde(rename = "bounded-proof")]
    Checks {
        /// Number of completed SUCCESS checks.
        #[serde(rename = "checks")]
        success_checks: u32,
    },
    /// Every reachable product state was explored.
    Exhaustive,
    /// Unrolling completed at or beyond the formula's horizon.
    BoundedComplete {
        /// Completed unrolling depth.
        depth: u64,
    },
    /// The property was established by induction.
    Inductive {
        /// Induction depth.
        depth: u64,
    },
    /// A closed-form schedulability analysis whose evidence the core
    /// recomputed (ADR-026 RT-7).
    ClosedForm {
        /// The analysis.
        analysis: ClosedFormAnalysis,
    },
    /// FR-250: a hybrid solver's sound result, holding for the claim within
    /// its time horizon and jump bound. Always paired with
    /// [`Certification::Uncertified`]: a solver QSL runs has no core
    /// certificate checker, and `SolverResult::terminal_value` is the only
    /// place that builds one.
    BoundedSolver {
        /// The kind of method that produced the result.
        method: SolverMethod,
        /// The time horizon the proof covers.
        horizon: SolverHorizon,
        /// The jump bound the proof covers.
        jumps: u64,
    },
    /// FR-169 (ADR-022 GV-1): a replayed witness for every initial state of
    /// a state-graph `possible` item, from a run that completed with no open
    /// node, and how each was found; QSpec FR-391's `witnessed{sources}`.
    #[serde(rename = "witnessed")]
    Witness {
        /// One source per initial state, in initial-state order.
        sources: Vec<WitnessSource>,
    },
    /// ADR-021 RV-1 (FR-160): a model check that completed with a reduction
    /// applied and found no counterexample, a technique of its own and never
    /// `Exhaustive`. Always paired with [`Certification::Uncertified`]: no
    /// core checker covers a reduced graph.
    Reduced {
        /// Each applied reduction, in the order symmetry, partial-order.
        reductions: Vec<AppliedReduction>,
    },
}

/// One reduction a [`ProofBasis::Reduced`] proof applied (FR-160).
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum AppliedReduction {
    /// Symmetry: the run's group, the keys the checked instance stands for,
    /// and the stabiliser it ran under.
    Symmetry {
        /// The run's group, class by class.
        groups: Vec<SymmetryClass>,
        /// Every key of the checked instance's orbit; empty for an item with
        /// no `over` binding.
        orbit: Vec<String>,
        /// The group the instance ran under.
        stabiliser: Vec<SymmetryClass>,
    },
    /// Partial-order reduction with the proviso it was run under.
    PartialOrder {
        /// The cycle proviso.
        proviso: Proviso,
    },
}

/// The cycle proviso of a partial-order reduction (ADR-021 POR-8).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Proviso {
    /// A state with an ample-set successor already discovered is fully
    /// expanded.
    BreadthFirstRevisit,
}

/// A reduction a request selects (FR-160 `ReductionKind`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReductionKind {
    /// Symmetry over population universes.
    Symmetry,
    /// Partial-order reduction.
    PartialOrder,
    /// A state constraint.
    StateConstraint,
}

impl ReductionKind {
    /// The reduction's advertised spelling (FR-159 "Capability").
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Symmetry => "symmetry",
            Self::PartialOrder => "partial-order",
            Self::StateConstraint => "state-constraint",
        }
    }
}

/// Why a symmetry declaration does not admit its subject (FR-160).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SymmetryBrokenCause {
    /// ADR-021 SYM-4: the generator maps the initial state with index
    /// `initial` in the subject's list to no initial state.
    InitialStatesNotClosed {
        /// The initial state's index in the subject's list.
        initial: u64,
        /// The generator.
        generator: Permutation,
    },
}

/// ADR-018 PC-1: who stands behind a proved result.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Certification {
    /// The qualified core produced or checked the proof.
    Certified,
    /// A first-party engine or solver proved it without a core check.
    Uncertified,
    /// A third-party plugin proved it without a core check.
    Trusted,
}

impl Certification {
    /// The required QSpec FR-331 certification member's spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Certified => "certified",
            Self::Uncertified => "uncertified",
            Self::Trusted => "trusted",
        }
    }
}

/// QSpec FR-243's settlement basis: the closed vocabulary every truth
/// result carries exactly one of (ADR-031 SW-3, SW-10). Compared by its
/// exact label.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum SettlementBasis {
    /// `closed-scope`: the decision scope is complete and closed.
    ClosedScope,
    /// `decisive-witness`: a retained witness preserves a `satisfied`
    /// truth.
    DecisiveWitness,
    /// `decisive-counterexample`: a retained counterexample preserves a
    /// `violated` truth.
    DecisiveCounterexample,
    /// `unsettled`: execution completed but the truth is still pending.
    Unsettled,
    /// `unavailable`: no truth was settled (a refusal, an undefined or
    /// incomplete execution, an internal failure).
    Unavailable,
}

impl SettlementBasis {
    /// The exact QSpec FR-243 label.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClosedScope => "closed-scope",
            Self::DecisiveWitness => "decisive-witness",
            Self::DecisiveCounterexample => "decisive-counterexample",
            Self::Unsettled => "unsettled",
            Self::Unavailable => "unavailable",
        }
    }

    /// Whether this basis is decisive, so a separating witness record
    /// accompanies it (QSpec FR-351).
    pub fn is_decisive(self) -> bool {
        match self {
            Self::DecisiveWitness | Self::DecisiveCounterexample => true,
            Self::ClosedScope | Self::Unsettled | Self::Unavailable => false,
        }
    }

    /// The decisive basis for a clause whose truth is `truth` (ADR-031
    /// SW-3): `decisive-witness` for `true`, `decisive-counterexample` for
    /// `false`.
    pub fn decisive(truth: bool) -> Self {
        if truth {
            Self::DecisiveWitness
        } else {
            Self::DecisiveCounterexample
        }
    }
}

/// FR-331's `incomplete` cause: which charge point the run failed to complete.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum IncompleteCause {
    /// The run exceeded its configured time budget.
    TimedOut,
    /// The run was cancelled before it produced a result.
    Cancelled {
        /// The caller-owned cancellation handle's FR-276 cause.
        source: CancelCause,
    },
    /// The run exhausted a configured resource bound before completing.
    ResourceExhausted,
    /// FR-127: an EN-1 run reached one of its named limits before it
    /// completed its method: the limit, the value it was set to, and so the
    /// request member that raises it ([`ModelCheckLimit::setting`]).
    LimitReached {
        /// The limit reached.
        limit: ModelCheckLimit,
        /// The value the run used for it.
        value: u64,
    },
}

impl IncompleteCause {
    /// The cause's FR-331 wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TimedOut => "timed-out",
            Self::Cancelled { .. } => "cancelled",
            Self::ResourceExhausted | Self::LimitReached { .. } => "limit-reached",
        }
    }
}

/// FR-331's `declined` (refusal) cause: `Refused`, `InvalidInput` or
/// `IncompleteInput` all collapse to one FR-331 result with this typed
/// cause (ADR-013 O-16, QC-9).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum ProofRefusalCause {
    /// The backend declined the request outright.
    Refused,
    /// The request's input was invalid.
    InvalidInput,
    /// The request's input was incomplete.
    IncompleteInput,
}

impl ProofRefusalCause {
    /// The cause's wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Refused => "refused",
            Self::InvalidInput => "invalid-input",
            Self::IncompleteInput => "incomplete-input",
        }
    }
}

/// FR-331's `unsupported` cause: the solver or backend is absent after
/// negotiation (ADR-013 O-16), or the premise a clause needs is not there
/// (FR-132).
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum UnavailabilityCause {
    /// No solver satisfying the request's capability negotiation is present.
    SolverAbsent,
    /// No backend satisfying the request's capability negotiation is
    /// present.
    BackendAbsent,
    /// A clause with a fairness constraint was evaluated over a supplied
    /// trace, which carries no enabledness (FR-132, ADR-019 SV-2): the
    /// first constraint of its checked fairness set. Wire cause
    /// `unsupported_projection`/`missing-fairness-premise` (QSpec FR-362).
    MissingFairnessPremise {
        /// The first constraint of the checked fairness set.
        constraint: FairnessConstraint,
    },
    /// ADR-018 V-8, ADR-021 RV-6: the item selects a reduction its system
    /// does not offer. Wire cause
    /// `unsupported_projection`/`unsupported-requested-capability`.
    UnsupportedReduction {
        /// The first selected reduction, in the order symmetry,
        /// partial-order, constraint, that is not offered.
        reduction: ReductionKind,
    },
    /// ADR-020 AX-5: a refinement with a hidden abstract field and a
    /// non-empty `F_A` has no liveness half to decide. Wire cause
    /// `unsupported_projection`/`unsupported-requested-capability`.
    HiddenFieldLiveness,
    /// A refinement with a strong constraint in `F_A`, whose liveness half
    /// the second phase does not decide: it reads a class as weak. Wire cause
    /// `unsupported_projection`/`unsupported-requested-capability`.
    StrongAbstractFairness,
}

impl UnavailabilityCause {
    /// The cause's wire spelling.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SolverAbsent => "solver-absent",
            Self::BackendAbsent => "backend-absent",
            Self::MissingFairnessPremise { .. } => "missing-fairness-premise",
            Self::UnsupportedReduction { .. }
            | Self::HiddenFieldLiveness
            | Self::StrongAbstractFairness => "unsupported-requested-capability",
        }
    }

    /// The catalog code the wire cause travels under, where the cause has
    /// one (QSpec FR-362).
    pub const fn code(&self) -> Option<&'static str> {
        match self {
            Self::SolverAbsent | Self::BackendAbsent => None,
            Self::MissingFairnessPremise { .. }
            | Self::UnsupportedReduction { .. }
            | Self::HiddenFieldLiveness
            | Self::StrongAbstractFairness => Some("unsupported_projection"),
        }
    }

    /// The QSpec FR-243 basis every unavailability settles with: no truth
    /// was settled.
    pub const fn basis(&self) -> SettlementBasis {
        SettlementBasis::Unavailable
    }

    /// The cause an evaluator outcome settles with, for the outcome that
    /// is a missing premise (FR-132); `None` for every other outcome.
    pub fn from_temporal_outcome(outcome: &TemporalOutcome) -> Option<Self> {
        match outcome {
            TemporalOutcome::MissingFairnessPremise { constraint } => {
                Some(Self::MissingFairnessPremise {
                    constraint: constraint.clone(),
                })
            }
            TemporalOutcome::Holds
            | TemporalOutcome::Violated { .. }
            | TemporalOutcome::Undefined(_)
            | TemporalOutcome::Pending
            | TemporalOutcome::Incomplete(_)
            | TemporalOutcome::WindowLimitReached(_) => None,
        }
    }
}

/// The cause of a `TerminalValue::Inconclusive`: a closed set that later
/// causes extend (ADR-018 §1, ADR-020 RE-4, ADR-022, ADR-023 HV-4, ADR-025
/// MV-1), with no vacuous-proof member, because a vacuous proof has basis
/// `Checks { success_checks: 0 }`
/// (ADR-013 C-09) and a second spelling of it must be unrepresentable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InconclusiveCause {
    /// The completed search horizon found no counterexample.
    BoundReached {
        /// Completed search depth.
        depth: u64,
    },
    /// The induction step did not close the proof.
    InductionNotClosed {
        /// Attempted induction depth.
        depth: u64,
    },
    /// A successor's contract conjunction could not be decided.
    UndecidedSuccessor,
    /// The subject supplies no initial state.
    NoInitialState,
    /// The item has zero instances (FR-125).
    NoInstance,
    /// `match-undetermined` (ADR-023 HV-4): a hyper item's `match` block was
    /// refused or incomplete at a reachable joint step.
    MatchUndetermined,
    /// `vacuous-match` (ADR-023 HM-7): a hyper item's non-empty universal
    /// `match` is satisfied by no fair tuple of universal behaviours.
    VacuousMatch,
    /// `mapping-undetermined` (ADR-020 RE-4): a refinement's mapping row or
    /// step-row argument was refused or incomplete at a reachable state.
    MappingUndetermined,
    /// The core rejected a certificate, retaining its failing rule and locus.
    CertificateRejected {
        /// The check rule that failed.
        rule: CertificateRule,
        /// Where the rule failed.
        at: CertificateLocus,
    },
    /// `replay_parity`: the counterexample's E9 replay settled
    /// `inconclusive`, so the reason travels with the cause.
    ReplayParity(DisagreementCause),
    /// `replay_refused`: the counterexample's E9 replay refused (an
    /// identity mismatch, a decode refusal, a stale dependency, a limit
    /// reached), carrying the refusal's catalog code
    /// ([`ReplayRefusal::code`]). A fault is never this cause: it settles
    /// [`TerminalValue::Failed`].
    ReplayRefused(Code),
    /// `scalar_agrees`: a scalar-parity counterexample (FR-357), operator-
    /// or function-level, did not reproduce, because the exact outcome
    /// equals the generated one. The harness reported a divergence there is
    /// none of: a harness defect. The agreement names the claim and the
    /// outcome, and no predicate verdict.
    ScalarAgrees(ScalarAgreement),
    /// AMC-rtb's high-mode criterion failed; the test is sufficient and
    /// not necessary, so it decides nothing (ADR-026 RT-6).
    SufficientTestFailed,
    /// FR-250: a hybrid solver result that proves nothing (`δ-sat`, a
    /// flowpipe meeting the unsafe set, `unknown`, or a method not sound for
    /// its outcome).
    SolverInconclusive,
    /// FR-169 (ADR-022 GV-1, RU-5): every initial state of a state-graph
    /// `possible` item has a replayed witness, but the run a limit stopped,
    /// or that ended with open nodes, did not rule out a reachable undefined
    /// evaluation of the target.
    WellDefinednessUnchecked {
        /// Each witness's source, in initial-state order.
        sources: Vec<WitnessSource>,
    },
    /// ADR-021 RV-2: a selected reduction's preservation-table row reads
    /// "no" for the claim's form.
    ReductionNotPreserving {
        /// The first selected reduction the table refuses.
        reduction: ReductionKind,
        /// The claim's property form.
        form: PropertyForm,
    },
    /// ADR-021 RV-3: a symmetry declaration's initial states are not closed
    /// under its group.
    SymmetryBroken {
        /// The declaration's population.
        population: DeclarationKey,
        /// Why.
        cause: SymmetryBrokenCause,
    },
    /// ADR-021 RV-4: the run completed with no counterexample and reached
    /// boundary states of its state constraint.
    ConstraintReached {
        /// The boundary states the run stored.
        boundary_states: u64,
    },
    /// ADR-021 RV-5: a partial-order run completed to its search horizon
    /// with no counterexample. A reduced graph does not keep short
    /// counterexamples short, so the horizon proves no bound.
    ReductionHorizon {
        /// The search horizon.
        max_depth: u64,
    },
}

impl InconclusiveCause {
    /// The cause's FR-331 wire spelling.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::BoundReached { .. } => "bound-reached",
            Self::InductionNotClosed { .. } => "induction-not-closed",
            Self::UndecidedSuccessor => "undecided-successor",
            Self::NoInitialState => "no-initial-state",
            Self::NoInstance => "no-instance",
            Self::MatchUndetermined => "match-undetermined",
            Self::VacuousMatch => "vacuous-match",
            Self::MappingUndetermined => "mapping-undetermined",
            Self::CertificateRejected { .. } => "certificate-rejected",
            Self::ReplayParity(_) => "replay-parity",
            Self::ReplayRefused(_) => "replay-refused",
            Self::ScalarAgrees(_) => "scalar-agrees",
            Self::SolverInconclusive => "solver-inconclusive",
            Self::SufficientTestFailed => "sufficient-test-failed",
            Self::WellDefinednessUnchecked { .. } => "well-definedness-unchecked",
            Self::ReductionNotPreserving { .. } => "reduction-not-preserving",
            Self::SymmetryBroken { .. } => "symmetry-broken",
            Self::ConstraintReached { .. } => "constraint-reached",
            Self::ReductionHorizon { .. } => "reduction-horizon",
        }
    }

    /// Bytes this cause adds to an encoded record beyond its fixed size:
    /// the nested witness records and failure of a parity disagreement.
    fn measured_bytes(&self) -> usize {
        match self {
            Self::BoundReached { .. }
            | Self::InductionNotClosed { .. }
            | Self::UndecidedSuccessor
            | Self::NoInitialState
            | Self::NoInstance
            | Self::MatchUndetermined
            | Self::VacuousMatch
            | Self::MappingUndetermined
            | Self::SufficientTestFailed
            | Self::CertificateRejected { .. }
            | Self::SolverInconclusive
            | Self::ReductionNotPreserving { .. }
            | Self::ConstraintReached { .. }
            | Self::ReductionHorizon { .. } => 0,
            Self::SymmetryBroken { population, cause } => {
                let SymmetryBrokenCause::InitialStatesNotClosed { generator, .. } = cause;
                let moved: usize = generator
                    .maps()
                    .values()
                    .map(|map| {
                        map.iter()
                            .map(|(from, to)| from.len().saturating_add(to.len()))
                            .sum::<usize>()
                    })
                    .sum();
                population
                    .package
                    .len()
                    .saturating_add(population.node.len())
                    .saturating_add(moved)
            }
            Self::WellDefinednessUnchecked { .. } => 0,
            Self::ReplayParity(cause) => cause.measured_bytes(),
            Self::ReplayRefused(_) => 0,
            Self::ScalarAgrees(agreement) => agreement.measured_bytes(),
        }
    }
}

/// FR-331's `inconclusive` cause as an envelope reports it (ADR-013 O-16
/// inconclusive row): a closed set covering both the vacuous proof and
/// the replay causes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReportedInconclusiveCause {
    /// `kani_vacuous_proof`: a `Proved` run with zero SUCCESS checks in the
    /// obligation, derived from the `Checks { success_checks: 0 }` basis by
    /// [`TerminalValue::vacuous_proof_cause`].
    KaniVacuousProof,
    /// The cause of a `TerminalValue::Inconclusive`.
    Cause(InconclusiveCause),
}

/// The code a `declined` result carries, in the registry it belongs to.
/// Codes are never remapped across registries: a STD-001 registry code
/// (IR's `kani_*` codes among them) is never spelled as a QSL catalog code,
/// and the reverse. The `Std001` arm names the registry, not an issuer, and
/// refuses no unregistered code: `Std001Code`'s own form check is all.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeclineCode {
    /// A QSL catalog code.
    Qsl(Code),
    /// A STD-001 registry code.
    Std001(Std001Code),
}

/// One FR-331 terminal record's result value (ADR-013 O-16 proof column).
/// FR-331's `results` vocabulary admits eight wire values and `measured`
/// is a ninth; this type names seven of the proof column's, with `Proved`
/// distinguishing a vacuous run (zero SUCCESS checks, FR-331-AC-8's
/// `inconclusive`) from an ordinary one by its own field rather than by a
/// second variant (ADR-013 C-09), so the category-mapping reader can tell
/// them apart without inspecting anything but this value.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalValue {
    /// A proof carrying its strength and exactly one certification.
    /// A zero-check `Checks` basis is vacuous (category `inconclusive`);
    /// every other basis maps to `success`, with any certification.
    Proved {
        /// How the claim was established.
        basis: ProofBasis,
        /// Who produced or checked the proof (ADR-018 PC-1).
        certification: Certification,
    },
    /// A backend result of `tested`: `success` category, but never promoted
    /// to `proved` and never counted as proof evidence.
    Tested,
    /// A backend result of `refuted`: the property does not hold.
    Refuted,
    /// A backend result of `declined`, `invalid-input` or
    /// `incomplete-input`, collapsed to one typed cause, with the code of
    /// the refusal it settles (FR-121).
    Declined {
        /// The typed refusal cause.
        cause: ProofRefusalCause,
        /// The refusal's catalog code.
        code: DeclineCode,
    },
    /// A backend result of `unsupported`.
    Unsupported(UnavailabilityCause),
    /// A backend result of `incomplete`.
    Incomplete(IncompleteCause),
    /// A backend result of `inconclusive`, with its typed cause.
    Inconclusive(InconclusiveCause),
    /// A backend result of `failed`: the tool itself failed.
    Failed,
}

impl ReportedInconclusiveCause {
    /// The cause's wire spelling: a vacuous proof's, or the cause's own.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::KaniVacuousProof => "kani-vacuous-proof",
            Self::Cause(cause) => cause.as_str(),
        }
    }
}

impl TerminalValue {
    /// Map this value to its ADR-013 O-16 category. Exhaustive with no `_`
    /// arm; a further `TerminalValue` variant fails this match to compile
    /// rather than silently falling into an existing row (FR-069's
    /// Behavior: "one exhaustive function with no `_` fallback arm").
    pub fn category(&self) -> Category {
        match self {
            Self::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                ..
            }
            | Self::Inconclusive(_) => Category::Inconclusive,
            Self::Proved {
                basis:
                    ProofBasis::Checks { .. }
                    | ProofBasis::Exhaustive
                    | ProofBasis::BoundedComplete { .. }
                    | ProofBasis::Inductive { .. }
                    | ProofBasis::ClosedForm { .. }
                    | ProofBasis::BoundedSolver { .. }
                    | ProofBasis::Witness { .. }
                    | ProofBasis::Reduced { .. },
                ..
            }
            | Self::Tested => Category::Success,
            Self::Refuted => Category::Violation,
            Self::Declined { .. } => Category::Refusal,
            Self::Unsupported(_) => Category::Unsupported,
            Self::Incomplete(_) => Category::Incomplete,
            Self::Failed => Category::InternalFailure,
        }
    }

    /// The typed cause of a vacuous proof, if this value is one.
    pub fn vacuous_proof_cause(&self) -> Option<ReportedInconclusiveCause> {
        match self {
            Self::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                ..
            } => Some(ReportedInconclusiveCause::KaniVacuousProof),
            _ => None,
        }
    }

    /// The typed cause of this value's `inconclusive` category: a vacuous
    /// proof's `KaniVacuousProof`, or an `Inconclusive` value's own cause;
    /// `None` for every other value.
    pub fn inconclusive_cause(&self) -> Option<ReportedInconclusiveCause> {
        match self {
            Self::Inconclusive(cause) => Some(ReportedInconclusiveCause::Cause(cause.clone())),
            Self::Proved { .. } => self.vacuous_proof_cause(),
            Self::Tested
            | Self::Refuted
            | Self::Declined { .. }
            | Self::Unsupported(_)
            | Self::Incomplete(_)
            | Self::Failed => None,
        }
    }

    /// ADR-013 C-09's replay-refusal row, for a refusal of a counterexample's
    /// E9 replay (a refusal after a backend run): `Inconclusive` with
    /// `ReplayRefused` carrying the refusal's code, or [`Self::Failed`] when
    /// the refusal is a fault (`ReplayRefusal::Fault`, or
    /// `ReplayRefusal::Admission` with `AdmissionFailure::Fault`), so a
    /// defect stays a tool failure. A refusal before any backend run is no
    /// replay refusal: it settles `declined` (FR-121).
    pub fn from_replay_refusal(refusal: &ReplayRefusal) -> Self {
        match refusal {
            ReplayRefusal::Fault(_) | ReplayRefusal::Admission(AdmissionFailure::Fault(_)) => {
                Self::Failed
            }
            _ => Self::Inconclusive(InconclusiveCause::ReplayRefused(refusal.code())),
        }
    }

    /// FR-121's call-site-refusal row, for a refusal before any backend run:
    /// the obligation's own input is refused, so a non-fault refusal settles
    /// `Declined` with cause `InvalidInput` and the refusal's own code
    /// ([`CallSiteRefusal::code`]); `CallSiteRefusal::Fault` settles
    /// [`Self::Failed`], so a defect stays a tool failure.
    pub fn from_call_site_refusal(refusal: &CallSiteRefusal) -> Self {
        match refusal {
            CallSiteRefusal::Fault(_) => Self::Failed,
            CallSiteRefusal::Compile { .. }
            | CallSiteRefusal::ModelIntake { .. }
            | CallSiteRefusal::DependencyInput(_)
            | CallSiteRefusal::Import { .. }
            | CallSiteRefusal::Dependency { .. }
            | CallSiteRefusal::UnknownFunction { .. }
            | CallSiteRefusal::UnknownOperation { .. }
            | CallSiteRefusal::UnknownClause { .. }
            | CallSiteRefusal::UnknownField { .. }
            | CallSiteRefusal::UnknownPopulation { .. } => Self::Declined {
                cause: ProofRefusalCause::InvalidInput,
                code: DeclineCode::Qsl(refusal.code()),
            },
        }
    }

    /// Bytes this value adds to an encoded record beyond its fixed size.
    fn measured_bytes(&self) -> usize {
        match self {
            Self::Inconclusive(cause) => cause.measured_bytes(),
            Self::Proved { .. }
            | Self::Tested
            | Self::Refuted
            | Self::Declined { .. }
            | Self::Unsupported(_)
            | Self::Incomplete(_)
            | Self::Failed => 0,
        }
    }
}

/// One requested item's FR-331 terminal record: its `request_index` and its
/// result value. A backend reports exactly one per `request_index`
/// (ADR-013 O-24), and a consumer joins on the index, never on text.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TerminalRecord {
    request_index: RequestIndex,
    /// The producing backend, present when this record was settled from a
    /// replay result through the public constructor.  Legacy provider-side
    /// values assembled inside QSL use the crate-private `from_value` helper
    /// and are completed with the envelope's manifest identity on read.
    backend: Option<BackendId>,
    value: TerminalValue,
    replay: Option<ReplayResult>,
}

impl TerminalRecord {
    /// Build a refuted terminal record from a replay result that reproduced
    /// a violation.  The backend identity is required evidence for the
    /// producing provider; a replay that did not settle `violation` is
    /// refused rather than being relabelled as `refuted`.
    ///
    /// ```compile_fail
    /// use qsl_replay::{RequestIndex, TerminalRecord, TerminalValue};
    ///
    /// // A terminal value alone is not enough evidence to construct a record.
    /// let _ = TerminalRecord::new(RequestIndex::new(0), TerminalValue::Refuted);
    /// ```
    ///
    /// The evidence-bearing constructor remains usable with a replay result:
    ///
    /// ```no_run
    /// use qsl_foundation::BackendId;
    /// use qsl_replay::{ReplayResult, RequestIndex, TerminalRecord};
    ///
    /// # let replay: &ReplayResult = unimplemented!();
    /// let _ = TerminalRecord::new(
    ///     RequestIndex::new(0),
    ///     BackendId::new("backend"),
    ///     replay,
    /// );
    /// ```
    ///
    /// A successful proof likewise requires a producing backend.  There is no
    /// constructor that accepts a bare `Proved` value.
    ///
    /// ```compile_fail
    /// use qsl_replay::{RequestIndex, TerminalRecord};
    ///
    /// let _ = TerminalRecord::proved(RequestIndex::new(0), 1);
    /// ```
    ///
    /// A proved record with its required backend is a compiling control:
    ///
    /// ```
    /// use qsl_foundation::BackendId;
    /// use qsl_replay::{RequestIndex, TerminalRecord};
    ///
    /// let _ = TerminalRecord::proved(RequestIndex::new(0), BackendId::new("backend"), 1);
    /// ```
    pub fn new(
        request_index: RequestIndex,
        backend: BackendId,
        replay: &ReplayResult,
    ) -> Result<Self, TerminalRecordRefusal> {
        match replay {
            ReplayResult::Witness(arm) => match arm.settlement() {
                WitnessSettlement::ReproducedWithEvaluatedWitness
                    if arm.category() == Category::Violation => {}
                WitnessSettlement::ReproducedWithEvaluatedWitness => {
                    return Err(TerminalRecordRefusal::ReplayDidNotRefute(arm.category()));
                }
                WitnessSettlement::Inconclusive => {
                    return Err(TerminalRecordRefusal::ReplayDidNotRefute(
                        Category::Inconclusive,
                    ));
                }
            },
            ReplayResult::Input(_) => return Err(TerminalRecordRefusal::ReplayLacksWitness),
        }
        Ok(Self {
            request_index,
            backend: Some(backend),
            value: TerminalValue::Refuted,
            replay: Some(replay.clone()),
        })
    }

    /// Build a proved record with the producing backend identity and the
    /// number of successful checks that established the proof.
    pub fn proved(request_index: RequestIndex, backend: BackendId, success_checks: u32) -> Self {
        Self::with_backend(
            request_index,
            backend,
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks },
                certification: Certification::Certified,
            },
        )
    }

    /// Build a tested record with the producing backend identity.
    pub fn tested(request_index: RequestIndex, backend: BackendId) -> Self {
        Self::with_backend(request_index, backend, TerminalValue::Tested)
    }

    /// Build an inconclusive record with the producing backend identity and
    /// its typed cause.
    pub fn inconclusive(
        request_index: RequestIndex,
        backend: BackendId,
        cause: InconclusiveCause,
    ) -> Self {
        Self::with_backend(request_index, backend, TerminalValue::Inconclusive(cause))
    }

    /// Build a declined record with the producing backend identity and its
    /// typed refusal cause and code.
    pub fn declined(
        request_index: RequestIndex,
        backend: BackendId,
        cause: ProofRefusalCause,
        code: DeclineCode,
    ) -> Self {
        Self::with_backend(
            request_index,
            backend,
            TerminalValue::Declined { cause, code },
        )
    }

    /// Build an unsupported record with the producing backend identity.
    pub fn unsupported(
        request_index: RequestIndex,
        backend: BackendId,
        cause: UnavailabilityCause,
    ) -> Self {
        Self::with_backend(request_index, backend, TerminalValue::Unsupported(cause))
    }

    /// Build an incomplete record with the producing backend identity.
    pub fn incomplete(
        request_index: RequestIndex,
        backend: BackendId,
        cause: IncompleteCause,
    ) -> Self {
        Self::with_backend(request_index, backend, TerminalValue::Incomplete(cause))
    }

    /// Build a failed record with the producing backend identity.
    pub fn failed(request_index: RequestIndex, backend: BackendId) -> Self {
        Self::with_backend(request_index, backend, TerminalValue::Failed)
    }

    fn with_backend(request_index: RequestIndex, backend: BackendId, value: TerminalValue) -> Self {
        Self {
            request_index,
            backend: Some(backend),
            value,
            replay: None,
        }
    }

    /// Build an internally settled value.  Provider-facing callers must use
    /// [`Self::new`] so a `refuted` value is tied to replay evidence and a
    /// producing backend.
    #[allow(
        dead_code,
        reason = "used by qsl-replay's crate-local outcome fixtures"
    )]
    pub(crate) fn from_value(request_index: RequestIndex, value: TerminalValue) -> Self {
        Self {
            request_index,
            backend: None,
            value,
            replay: None,
        }
    }

    /// The `request_index` of the item this record was read for.
    pub fn request_index(&self) -> RequestIndex {
        self.request_index
    }

    /// The FR-331 result value this record carries.
    pub fn value(&self) -> &TerminalValue {
        &self.value
    }

    /// The producing backend evidence, when this record came from a public
    /// replay settlement.
    pub fn backend(&self) -> Option<&BackendId> {
        self.backend.as_ref()
    }

    /// The replay evidence retained by a refuted record, when it was built
    /// through [`Self::new`].
    pub fn replay_result(&self) -> Option<&ReplayResult> {
        self.replay.as_ref()
    }
}

/// Why a public terminal-record construction was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TerminalRecordRefusal {
    /// The replay result did not settle as a reproduced violation.
    #[error("replay result settled as {0:?}, not violation")]
    ReplayDidNotRefute(Category),
    /// Only a reproduced witness arm can provide refutation evidence.
    #[error("input-arm replay does not carry backend witness evidence")]
    ReplayLacksWitness,
}

/// FR-069/ADR-013 O-24: the typed proof-result envelope for one requested
/// item. No public constructor other than [`read_backend_provider_envelope`]
/// (FR-069-CON-1): a caller cannot name an arbitrary category directly.
///
/// quire:canonical
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProofResultEnvelope {
    category: Category,
    record: TerminalRecord,
    backend: BackendId,
    inconclusive_cause: Option<ReportedInconclusiveCause>,
}

impl ProofResultEnvelope {
    /// The item's ADR-013 O-16 category.
    pub fn category(&self) -> Category {
        self.category
    }

    /// The FR-331 terminal record this category was read from.
    pub fn record(&self) -> &TerminalRecord {
        &self.record
    }

    /// The `backend` member (ADR-013 O-19).
    pub fn backend(&self) -> &BackendId {
        &self.backend
    }

    /// FR-069 Behavior's typed cause: `Some` exactly when this envelope's
    /// category is `Inconclusive` -- `KaniVacuousProof` for a vacuous
    /// `Proved` (zero SUCCESS checks), or the `Inconclusive` value's own
    /// typed inconclusive cause; `None` for every other category.
    /// A consumer that reads `category() == Inconclusive` never re-derives
    /// the cause from [`Self::record`] itself.
    pub fn inconclusive_cause(&self) -> Option<&ReportedInconclusiveCause> {
        self.inconclusive_cause.as_ref()
    }
}

/// [`read_backend_provider_envelope`]'s structured refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProofResultRefusal {
    /// The encoded envelope exceeds the configured reader bound.
    #[error(transparent)]
    BoundExceeded(#[from] BoundExceeded),
    /// A record's producing backend differs from the provider identity in
    /// the source envelope.
    #[error("record backend {record} differs from provider backend {provider}")]
    BackendIdentityMismatch {
        /// The identity embedded in the record.
        record: BackendId,
        /// The identity declared by the provider source.
        provider: BackendId,
    },
}

/// A minimal, already-parsed representation of one FR-331
/// `quire.backend-provider/v1` envelope: exactly the members FR-069's
/// Inputs section names (`results`, `manifest`), enough to build
/// and round-trip a [`ProofResultEnvelope`] per item. This is not a general
/// FR-331 wire reader; it is the input shape FR-069's reader consumes. It
/// is never read from bytes, so it carries no contract version or
/// vocabulary member: the version check belongs to a byte reader, and none
/// exists yet.
#[derive(Clone, Debug)]
pub struct BackendProviderSource {
    /// The provider identity, as the FR-331 manifest states it.
    pub backend_identity: String,
    /// The per-item terminal records this envelope reports.
    pub items: Vec<TerminalRecord>,
}

/// The reader's own measurement of `source`'s encoded size (FR-069-AC-4):
/// the provider and record backend identities' byte lengths, a fixed
/// per-item allowance for the index and terminal value, and the retained
/// replay and nested cause bytes. No caller-declared number can understate
/// an oversized item list past the bound (B3).
fn measured_encoded_bytes(source: &BackendProviderSource) -> usize {
    source.backend_identity.len()
        + source
            .items
            .iter()
            .map(|record| {
                std::mem::size_of::<RequestIndex>()
                    + std::mem::size_of::<TerminalValue>()
                    + record.value().measured_bytes()
                    + record.backend().map_or(0, |backend| backend.as_str().len())
                    + record
                        .replay_result()
                        .map_or(0, crate::result::measured_encoded_bytes)
            })
            .sum::<usize>()
}

/// FR-069's reader: read every item of `source` into its
/// [`ProofResultEnvelope`], refusing before any `results`/`dispositions`/
/// `counterexamples`/`accounting` member is read if the encoded size is over
/// the configured reader bound (FR-069-AC-4).
#[qsl_attrs::string_edge]
pub fn read_backend_provider_envelope(
    source: &BackendProviderSource,
    limits: ReplayLimits,
) -> Result<Vec<ProofResultEnvelope>, ProofResultRefusal> {
    // The bound check happens strictly first: nothing below this point
    // touches `source.items` until it has passed.
    BoundExceeded::check(measured_encoded_bytes(source), limits)?;

    let backend = BackendId::new(source.backend_identity.clone());
    for record in &source.items {
        if let Some(record_backend) = record.backend() {
            if record_backend != &backend {
                return Err(ProofResultRefusal::BackendIdentityMismatch {
                    record: record_backend.clone(),
                    provider: backend.clone(),
                });
            }
        }
    }
    Ok(source
        .items
        .iter()
        .map(|record| ProofResultEnvelope {
            category: record.value().category(),
            inconclusive_cause: record.value().inconclusive_cause(),
            record: record.clone(),
            backend: backend.clone(),
        })
        .collect())
}

impl ProofResultEnvelope {
    /// Re-serialize `envelopes` (every one read from a single source) back
    /// into the [`BackendProviderSource`] shape [`read_backend_provider_envelope`]
    /// consumes, for round-tripping a positive read through the reader again
    /// (FR-069-AC-3's construct -> serialize -> read round trip). Every
    /// envelope this reader ever produces shares one `backend`
    /// (they all come from the one source it read), so the first envelope's
    /// is representative. An empty slice has no `backend` to serialize and
    /// is refused with [`EmptyEnvelopeSet`].
    pub fn to_source(envelopes: &[Self]) -> Result<BackendProviderSource, EmptyEnvelopeSet> {
        let first = envelopes.first().ok_or(EmptyEnvelopeSet)?;
        Ok(BackendProviderSource {
            backend_identity: first.backend.as_str().to_owned(),
            items: envelopes.iter().map(|e| e.record.clone()).collect(),
        })
    }
}

/// [`ProofResultEnvelope::to_source`]'s refusal: there is no envelope to
/// take the source's `backend` from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("no proof-result envelope to serialize")]
pub struct EmptyEnvelopeSet;

#[cfg(test)]
mod tests {

    fn fairness(
        kind: quire_semantic_value::fairness::FairnessKind,
        name: &str,
        granularity: quire_semantic_value::fairness::FairnessGranularity,
    ) -> FairnessConstraint {
        use quire_semantic_value::fairness::FairnessOperation;
        FairnessConstraint::new(
            kind,
            FairnessOperation::new(
                quire_exact::EffectiveId::from_digest([7; 32]),
                name.to_owned(),
            ),
            granularity,
        )
    }

    /// FR-132-AC-1: the evaluator's missing premise settles `unsupported`
    /// with the cause naming the first constraint, basis `unavailable`,
    /// wire cause `unsupported_projection`/`missing-fairness-premise`,
    /// category unsupported.
    #[trace("TC-533", "FR-132-AC-1")]
    #[test]
    fn a_missing_fairness_premise_settles_unsupported_naming_the_first_constraint() {
        use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};
        let first = fairness(FairnessKind::Strong, "acquire", FairnessGranularity::Each);
        let outcome = TemporalOutcome::MissingFairnessPremise {
            constraint: first.clone(),
        };
        let cause = UnavailabilityCause::from_temporal_outcome(&outcome)
            .expect("a missing premise has a cause");
        assert_eq!(
            cause,
            UnavailabilityCause::MissingFairnessPremise { constraint: first }
        );
        assert_ne!(
            cause,
            UnavailabilityCause::MissingFairnessPremise {
                constraint: fairness(FairnessKind::Weak, "release", FairnessGranularity::Whole)
            }
        );
        assert_eq!(cause.as_str(), "missing-fairness-premise");
        assert_eq!(cause.code(), Some("unsupported_projection"));
        assert_eq!(cause.basis().as_str(), "unavailable");

        let record =
            TerminalRecord::unsupported(RequestIndex::new(0), BackendId::new("backend"), cause);
        assert_eq!(record.value().category(), Category::Unsupported);
        let item = crate::outcome::OutcomeItem::from_terminal(&record);
        let wire = serde_json::to_value(&item).expect("the item serialises");
        assert_eq!(wire["result"], "unsupported");
        assert_eq!(wire["cause"]["kind"], "missing-fairness-premise");
        assert_eq!(wire["cause"]["code"], "unsupported_projection");
        assert_eq!(wire["category"], "unsupported");
    }

    /// Every other evaluator outcome is no missing premise.
    #[test]
    fn no_other_outcome_is_a_missing_fairness_premise() {
        assert_eq!(
            UnavailabilityCause::from_temporal_outcome(&TemporalOutcome::Holds),
            None
        );
        assert_eq!(
            UnavailabilityCause::from_temporal_outcome(&TemporalOutcome::Pending),
            None
        );
        // A window over `max_window_values` is an incomplete run, a limit
        // reached, not a premise the trace lacks.
        assert_eq!(
            UnavailabilityCause::from_temporal_outcome(&TemporalOutcome::WindowLimitReached(
                qsl_eval::temporal::WindowLimitReached {
                    limit: "max_window_values",
                    value: 10,
                    setting: "monitor.max_window_values",
                }
            )),
            None
        );
    }
    use super::*;
    use crate::bounds::DEFAULT_INPUT_BYTES;
    use crate::result::Verdict;
    use crate::scalar::{ScalarClaim, ScalarOutcome};
    use crate::witness::{CanonicalAssignment, WitnessValue};
    use ix_trace_rs::trace;
    use qsl_foundation::digest::{DigestDomain, DigestRecord, WireNodeId};
    use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
    use quire_contract_model::std001_code;

    fn source(items: Vec<TerminalRecord>) -> BackendProviderSource {
        BackendProviderSource {
            backend_identity: "kani-backend-1".to_owned(),
            items,
        }
    }

    fn function_claim(assignments: Vec<CanonicalAssignment>) -> ScalarClaim {
        ScalarClaim::Function(Box::new(crate::ValueIdentity {
            obligation: crate::ObligationIdentity::from_digest([1; 32]),
            package_id: (None, String::new()),
            function: crate::QualifiedName::new(vec![quire_exact::Identifier::new("f").unwrap()])
                .unwrap(),
            source: crate::ReplaySource::Input(assignments),
            limits: quire_exact::ScalarLimits {
                integer_bits: 1,
                decimal_digits: 1,
                scale_expansion: 1,
                text_input_bytes: 1,
                text_scalars: 1,
                normalized_scalars: 1,
                unit_edges: 1,
                value_occurrences: 1,
                work_units: 1,
                result_units: 1,
            },
            generated: ScalarOutcome::OutOfRange,
        }))
    }

    fn agreement() -> ScalarAgreement {
        ScalarAgreement::new(function_claim(Vec::new()), ScalarOutcome::OutOfRange)
    }

    fn parity_cause() -> DisagreementCause {
        DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Success),
        }
    }

    #[trace("TC-522", "FR-127-AC-2", "FR-127-AC-9")]
    #[test]
    fn proof_reader_preserves_every_basis_and_required_certification() {
        let bases = [
            (
                ProofBasis::Checks { success_checks: 0 },
                Category::Inconclusive,
            ),
            (ProofBasis::Checks { success_checks: 1 }, Category::Success),
            (ProofBasis::Checks { success_checks: 3 }, Category::Success),
            (ProofBasis::Exhaustive, Category::Success),
            (ProofBasis::BoundedComplete { depth: 5 }, Category::Success),
            (ProofBasis::Inductive { depth: 2 }, Category::Success),
            (ProofBasis::BoundedComplete { depth: 0 }, Category::Success),
            (ProofBasis::Inductive { depth: u64::MAX }, Category::Success),
        ];
        let certifications = [
            (Certification::Certified, "certified"),
            (Certification::Uncertified, "uncertified"),
            (Certification::Trusted, "trusted"),
        ];
        for (basis, category) in bases {
            for (certification, spelling) in certifications {
                let value = TerminalValue::Proved {
                    basis: basis.clone(),
                    certification,
                };
                let source = BackendProviderSource {
                    backend_identity: "proof-provider".to_owned(),
                    items: vec![TerminalRecord::from_value(
                        RequestIndex::new(0),
                        value.clone(),
                    )],
                };
                let envelopes = read_backend_provider_envelope(&source, ReplayLimits::default())
                    .expect("the complete proof record admits");
                assert_eq!(envelopes.len(), 1);
                let envelope = &envelopes[0];
                assert_eq!(
                    envelope.category(),
                    category,
                    "{basis:?}, {certification:?}"
                );
                assert_eq!(envelope.record().value(), &value);
                let TerminalValue::Proved {
                    basis: retained_basis,
                    certification: retained_certification,
                } = envelope.record().value()
                else {
                    panic!("the reader must retain Proved");
                };
                assert_eq!(*retained_basis, basis);
                assert_eq!(retained_certification.as_str(), spelling);
                let expected_cause = if category == Category::Inconclusive {
                    Some(&ReportedInconclusiveCause::KaniVacuousProof)
                } else {
                    None
                };
                assert_eq!(envelope.inconclusive_cause(), expected_cause);
            }
        }
    }

    #[trace("TC-522", "FR-127-AC-2")]
    #[test]
    fn temporal_inconclusive_causes_keep_depth_and_exact_category() {
        for (cause, spelling) in [
            (
                InconclusiveCause::BoundReached { depth: 1 },
                "bound-reached",
            ),
            (
                InconclusiveCause::InductionNotClosed { depth: 2 },
                "induction-not-closed",
            ),
            (InconclusiveCause::UndecidedSuccessor, "undecided-successor"),
            (InconclusiveCause::NoInitialState, "no-initial-state"),
            (InconclusiveCause::NoInstance, "no-instance"),
        ] {
            let source = BackendProviderSource {
                backend_identity: "model-provider".to_owned(),
                items: vec![TerminalRecord::from_value(
                    RequestIndex::new(7),
                    TerminalValue::Inconclusive(cause.clone()),
                )],
            };
            let envelopes = read_backend_provider_envelope(&source, ReplayLimits::default())
                .expect("the complete inconclusive record admits");
            assert_eq!(envelopes.len(), 1);
            assert_eq!(envelopes[0].category(), Category::Inconclusive);
            assert_eq!(envelopes[0].record(), &source.items[0]);
            assert_eq!(
                envelopes[0].inconclusive_cause(),
                Some(&ReportedInconclusiveCause::Cause(cause))
            );
            assert_eq!(
                envelopes[0].inconclusive_cause().unwrap().as_str(),
                spelling
            );
        }
    }

    #[trace("TC-522", "FR-127-AC-2", "FR-127-AC-9")]
    #[test]
    fn certificate_rejections_keep_every_rule_and_complete_typed_locus() {
        use crate::certificate::QueryPart;

        for rule in [
            CertificateRule::InitialMissing,
            CertificateRule::SuccessorMissing,
            CertificateRule::BadState,
            CertificateRule::NotPartition,
            CertificateRule::BackwardEdge,
            CertificateRule::WitnessFails,
            CertificateRule::QueryMismatch,
            CertificateRule::ShapeMismatch,
            CertificateRule::ProofStepInvalid,
            CertificateRule::NotRefutation,
        ] {
            for part in [QueryPart::Unrolling, QueryPart::Base, QueryPart::Step] {
                for at in [
                    CertificateLocus::Query { part },
                    CertificateLocus::ProofStep { part, index: 0 },
                    CertificateLocus::ProofStep {
                        part,
                        index: u64::MAX,
                    },
                ] {
                    let cause = InconclusiveCause::CertificateRejected { rule, at };
                    let input = source(vec![TerminalRecord::from_value(
                        RequestIndex::new(4),
                        TerminalValue::Inconclusive(cause.clone()),
                    )]);
                    let read = read_backend_provider_envelope(&input, ReplayLimits::default())
                        .expect("the complete typed certificate rejection admits");
                    assert_eq!(read.len(), 1);
                    assert_eq!(read[0].category(), Category::Inconclusive);
                    assert_eq!(read[0].record(), &input.items[0]);
                    assert_eq!(
                        read[0].inconclusive_cause(),
                        Some(&ReportedInconclusiveCause::Cause(cause))
                    );
                    assert_eq!(
                        read[0].inconclusive_cause().unwrap().as_str(),
                        "certificate-rejected"
                    );
                }
            }
        }
    }

    #[trace("TC-522", "FR-127-AC-10", "FR-072-AC-2")]
    #[test]
    fn replay_verdict_disagreement_reads_as_replay_parity() {
        use crate::result::{EvaluatedValue, WitnessArmResult, WitnessCheck, WitnessSettlement};

        let proved = Verdict::from_category(Category::Violation);
        let replayed = Verdict::from_category(Category::Success);
        let replay = WitnessArmResult::settle(
            proved,
            replayed,
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            WitnessCheck::Agrees(None),
            Vec::new(),
            quire_exact::ScalarLimits {
                integer_bits: 0,
                decimal_digits: 0,
                scale_expansion: 0,
                text_input_bytes: 0,
                text_scalars: 0,
                normalized_scalars: 0,
                unit_edges: 0,
                value_occurrences: 0,
                work_units: 0,
                result_units: 0,
            },
        );
        assert_eq!(replay.settlement(), WitnessSettlement::Inconclusive);
        let expected = DisagreementCause::Verdicts { proved, replayed };
        assert_eq!(replay.disagreement(), Some(&expected));
        // The proof reader consumes a supplied terminal record and retains
        // the replay's typed disagreement without reclassifying it.
        let source = BackendProviderSource {
            backend_identity: "model-provider".to_owned(),
            items: vec![TerminalRecord::from_value(
                RequestIndex::new(0),
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
                    replay.disagreement().unwrap().clone(),
                )),
            )],
        };
        let envelopes = read_backend_provider_envelope(&source, ReplayLimits::default()).unwrap();
        assert_eq!(envelopes.len(), 1);
        assert_eq!(envelopes[0].category(), Category::Inconclusive);
        assert_eq!(
            envelopes[0].inconclusive_cause(),
            Some(&ReportedInconclusiveCause::Cause(
                InconclusiveCause::ReplayParity(expected)
            ))
        );
        assert_eq!(
            envelopes[0].inconclusive_cause().unwrap().as_str(),
            "replay-parity"
        );
    }

    #[trace("TC-522", "FR-127-AC-4")]
    #[test]
    fn cancelled_terminal_records_keep_requested_and_deadline_sources() {
        for source in [CancelCause::Requested, CancelCause::Deadline] {
            let value = TerminalValue::Incomplete(IncompleteCause::Cancelled { source });
            let input = BackendProviderSource {
                backend_identity: "model-provider".to_owned(),
                items: vec![TerminalRecord::from_value(
                    RequestIndex::new(0),
                    value.clone(),
                )],
            };
            let envelopes =
                read_backend_provider_envelope(&input, ReplayLimits::default()).unwrap();
            assert_eq!(envelopes.len(), 1);
            assert_eq!(envelopes[0].category(), Category::Incomplete);
            assert_eq!(envelopes[0].record().value(), &value);
            assert_eq!(IncompleteCause::Cancelled { source }.as_str(), "cancelled");
        }
    }

    /// FR-069-AC-1 (TC-177): every FR-331 wire value maps to its exact
    /// O-16 category, `proved`/`tested` stay distinct within `success`, and
    /// a vacuous `Proved` (zero SUCCESS checks) maps to `inconclusive`, not
    /// `success` -- distinguished from an ordinary `Proved` only by the
    /// SUCCESS-check count, not by a different outer tag. An `Inconclusive`
    /// value maps to `inconclusive` with its own typed cause, whichever it
    /// is, and each record is keyed by its `request_index`.
    #[trace("TC-177", "FR-069-AC-1")]
    #[test]
    fn tc_177_every_fr331_value_maps_to_its_exact_category() {
        let cases = [
            (
                TerminalValue::Proved {
                    basis: ProofBasis::Checks { success_checks: 1 },
                    certification: Certification::Certified,
                },
                Category::Success,
                None,
            ),
            (
                TerminalValue::Proved {
                    basis: ProofBasis::Checks { success_checks: 0 },
                    certification: Certification::Certified,
                },
                Category::Inconclusive,
                Some(ReportedInconclusiveCause::KaniVacuousProof),
            ),
            (TerminalValue::Tested, Category::Success, None),
            (TerminalValue::Refuted, Category::Violation, None),
            (
                TerminalValue::Declined {
                    cause: ProofRefusalCause::Refused,
                    code: DeclineCode::Qsl(Code::MissingDeclaration),
                },
                Category::Refusal,
                None,
            ),
            (
                TerminalValue::Declined {
                    cause: ProofRefusalCause::Refused,
                    code: DeclineCode::Std001(Std001Code::KANI_BOUND_INVALID),
                },
                Category::Refusal,
                None,
            ),
            (
                TerminalValue::Unsupported(UnavailabilityCause::SolverAbsent),
                Category::Unsupported,
                None,
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::TimedOut),
                Category::Incomplete,
                None,
            ),
            (
                TerminalValue::Incomplete(IncompleteCause::Cancelled {
                    source: CancelCause::Requested,
                }),
                Category::Incomplete,
                None,
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(parity_cause())),
                Category::Inconclusive,
                Some(ReportedInconclusiveCause::Cause(
                    InconclusiveCause::ReplayParity(parity_cause()),
                )),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                    Code::StaleDependency,
                )),
                Category::Inconclusive,
                Some(ReportedInconclusiveCause::Cause(
                    InconclusiveCause::ReplayRefused(Code::StaleDependency),
                )),
            ),
            (
                TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(agreement())),
                Category::Inconclusive,
                Some(ReportedInconclusiveCause::Cause(
                    InconclusiveCause::ScalarAgrees(agreement()),
                )),
            ),
            (TerminalValue::Failed, Category::InternalFailure, None),
        ];
        let items: Vec<TerminalRecord> = cases
            .iter()
            .enumerate()
            .map(|(i, (value, _, _))| {
                TerminalRecord::from_value(RequestIndex::new(i), value.clone())
            })
            .collect();
        let envelopes =
            read_backend_provider_envelope(&source(items), crate::ReplayLimits::default()).unwrap();
        for (i, (envelope, (value, expected_category, expected_cause))) in
            envelopes.iter().zip(cases.iter()).enumerate()
        {
            assert_eq!(
                envelope.category(),
                *expected_category,
                "{value:?} should map to {expected_category:?}"
            );
            assert_eq!(envelope.record().value(), value);
            assert_eq!(envelope.record().request_index(), RequestIndex::new(i));
            assert_eq!(
                envelope.inconclusive_cause(),
                expected_cause.as_ref(),
                "item {i}"
            );
        }
        // `tested` never gets rewritten to `proved`, despite sharing a
        // category with it.
        assert_eq!(
            envelopes[0].record().value(),
            &TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 1 },
                certification: Certification::Certified,
            }
        );
        assert_eq!(envelopes[2].record().value(), &TerminalValue::Tested);
        assert_ne!(envelopes[0].record().value(), envelopes[2].record().value());
        // An ordinary and a vacuous `Proved` record take different
        // categories, showing the reader inspects the SUCCESS-check count
        // and not merely the outer `Proved` tag.
        assert_ne!(envelopes[0].category(), envelopes[1].category());
    }

    /// FR-121-AC-16 (ADR-013 C-09): a replay refusal after a backend run
    /// settles `Inconclusive(ReplayRefused)` with the refusal's own code; a
    /// fault settles `Failed`.
    #[trace("TC-516", "FR-121-AC-16")]
    #[test]
    fn a_replay_refusal_settles_inconclusive_with_its_code_and_a_fault_failed() {
        use qsl_foundation::diagnostic::InternalFault;

        let refusal = ReplayRefusal::UnboundParameter(WireNodeId::from_digest([1; 32]));
        assert_eq!(
            TerminalValue::from_replay_refusal(&refusal),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(refusal.code()))
        );
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
        assert_eq!(
            TerminalValue::from_replay_refusal(&refusal).category(),
            Category::Inconclusive
        );

        let fault = ReplayRefusal::Fault(InternalFault::new("replay", "broken"));
        assert_eq!(
            TerminalValue::from_replay_refusal(&fault),
            TerminalValue::Failed
        );
        let admission_fault = ReplayRefusal::Admission(AdmissionFailure::Fault(
            InternalFault::new("admission", "broken"),
        ));
        assert_eq!(
            TerminalValue::from_replay_refusal(&admission_fault),
            TerminalValue::Failed
        );
    }

    /// FR-285-AC-1 (TC-769 step 1): a `refuted` terminal record reports
    /// its category as the one `Category::Violation`, which the exit
    /// function maps to 10; no FR-331 value is undefined.
    #[trace("TC-769", "FR-285-AC-1")]
    #[test]
    fn tc_769_refuted_terminal_record_is_violation_exit_10() {
        let envelopes = read_backend_provider_envelope(
            &source(vec![TerminalRecord::from_value(
                RequestIndex::new(0),
                TerminalValue::Refuted,
            )]),
            crate::ReplayLimits::default(),
        )
        .unwrap();
        assert_eq!(envelopes[0].category(), Category::Violation);
        assert_eq!(envelopes[0].category().exit_code(), 10);
        for value in [
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 1 },
                certification: Certification::Certified,
            },
            TerminalValue::Proved {
                basis: ProofBasis::Checks { success_checks: 0 },
                certification: Certification::Certified,
            },
            TerminalValue::Tested,
            TerminalValue::Refuted,
            TerminalValue::Declined {
                cause: ProofRefusalCause::Refused,
                code: DeclineCode::Qsl(Code::MissingDeclaration),
            },
            TerminalValue::Unsupported(UnavailabilityCause::SolverAbsent),
            TerminalValue::Incomplete(IncompleteCause::TimedOut),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency)),
            TerminalValue::Failed,
        ] {
            assert_ne!(value.category(), Category::Undefined, "{value:?}");
        }
    }

    /// FR-069-AC-4 (TC-178): an oversized encoding refuses before any item
    /// is read, with no partial envelope returned.
    #[trace("TC-178", "FR-069-AC-4")]
    #[test]
    fn tc_178_refuses_an_oversized_envelope() {
        // B3: the bound check measures the source's own content -- there is
        // no `encoded_bytes` field a caller could understate -- so an
        // oversized source has to actually carry oversized content.
        let mut oversized = source(vec![TerminalRecord::from_value(
            RequestIndex::new(0),
            TerminalValue::Tested,
        )]);
        oversized.backend_identity = "x".repeat(DEFAULT_INPUT_BYTES + 1);
        let result = read_backend_provider_envelope(&oversized, crate::ReplayLimits::default());
        assert!(matches!(result, Err(ProofResultRefusal::BoundExceeded(_))));
    }

    fn refuted_source_with_region(authority: &str, identity: String) -> BackendProviderSource {
        let source_ref = RawSourceRef::new(
            authority,
            identity,
            DigestRecord::mint(DigestDomain::SourceBytesV1, [1; 32]),
        )
        .unwrap();
        let region = SourceRegion::new(source_ref, 0, 1).unwrap();
        let replay = ReplayResult::Witness(crate::WitnessArmResult::settle_range_violation(
            None,
            vec![region],
            crate::spine::default_accounting(0),
        ));
        source(vec![TerminalRecord::new(
            RequestIndex::new(0),
            BackendId::new("kani-backend-1"),
            &replay,
        )
        .unwrap()])
    }

    /// FR-069-AC-4 (TC-178): retained replay source identity bytes count at
    /// the exact bound, one byte beyond it, and beyond the default 1 MiB.
    #[trace("TC-178", "FR-069-AC-4")]
    #[test]
    fn retained_replay_bytes_obey_exact_envelope_bound() {
        let at_limit = refuted_source_with_region("registry", "source".to_owned());
        let expected = at_limit.backend_identity.len()
            + std::mem::size_of::<RequestIndex>()
            + std::mem::size_of::<TerminalValue>()
            + "kani-backend-1".len()
            + "registry".len()
            + "source".len();
        let limits = ReplayLimits::default().with_input_bytes(expected as u64);
        assert_eq!(
            read_backend_provider_envelope(&at_limit, limits)
                .unwrap()
                .len(),
            1
        );

        let over_limit = refuted_source_with_region("registry", "source!".to_owned());
        assert_eq!(
            read_backend_provider_envelope(&over_limit, limits),
            Err(ProofResultRefusal::BoundExceeded(BoundExceeded {
                bound: expected as u64,
                actual: (expected + 1) as u128,
            }))
        );

        let huge_identity =
            refuted_source_with_region("registry", "x".repeat(DEFAULT_INPUT_BYTES + 1));
        assert!(matches!(
            read_backend_provider_envelope(&huge_identity, ReplayLimits::default()),
            Err(ProofResultRefusal::BoundExceeded(_))
        ));
    }

    /// A reproduced witness's nested value path remains part of the proof
    /// envelope after the replay result is retained in a refuted record.
    #[trace("TC-178", "FR-069-AC-4")]
    #[test]
    fn retained_witness_path_cannot_bypass_envelope_bound() {
        use qsl_foundation::source::provenance::OccurrenceKey;
        use qsl_foundation::witness::{RuntimeValuePath, ValuePathStep, ValuePathSubject};
        use quire_exact::{Origin, Role, Value};

        let occurrence = OccurrenceKey::new(
            WireNodeId::from_digest([7; 32]),
            Origin::new(Role::new("expression"), 0),
        );
        let record = crate::SeparatingWitnessRecord {
            quantifier: occurrence.clone(),
            deciding_element: Value::Boolean(false),
            index: Some(0),
            value_path: RuntimeValuePath {
                observation: qsl_foundation::witness::ObservationIdentity {
                    authority: "source".to_owned(),
                    identity: "item".to_owned(),
                    revision_namespace: "revision".to_owned(),
                    revision: "1".to_owned(),
                },
                subject: ValuePathSubject::Built(occurrence),
                steps: vec![ValuePathStep::Member("x".repeat(DEFAULT_INPUT_BYTES + 1))],
            },
            trace_position: None,
        };
        let violation = Verdict::from_category(Category::Violation);
        let replay = ReplayResult::Witness(crate::WitnessArmResult::settle(
            violation,
            violation,
            Category::Violation,
            Some(crate::EvaluatedValue::Boolean(false)),
            crate::WitnessCheck::Agrees(Some(Box::new(record))),
            Vec::new(),
            crate::spine::default_accounting(0),
        ));
        let provider = source(vec![TerminalRecord::new(
            RequestIndex::new(0),
            BackendId::new("kani-backend-1"),
            &replay,
        )
        .unwrap()]);
        assert!(matches!(
            read_backend_provider_envelope(&provider, ReplayLimits::default()),
            Err(ProofResultRefusal::BoundExceeded(_))
        ));
    }

    /// A range-violation witness retains its optional incomplete record;
    /// the mathematical integer charge remains bounded by the proof reader.
    #[trace("TC-178", "FR-069-AC-4")]
    #[test]
    fn retained_replay_limit_charge_cannot_bypass_envelope_bound() {
        let replay = ReplayResult::Witness(
            crate::WitnessArmResult::settle_range_violation(
                None,
                Vec::new(),
                crate::spine::default_accounting(0),
            )
            .with_limit(quire_exact::Incomplete {
                limit_kind: quire_exact::LimitKind::WorkUnits,
                limit: 1,
                consumed: 1,
                next_charge: "99".parse().unwrap(),
                charge_point: quire_exact::ChargePoint::TextInputBytes,
            }),
        );
        let provider = source(vec![TerminalRecord::new(
            RequestIndex::new(0),
            BackendId::new("kani-backend-1"),
            &replay,
        )
        .unwrap()]);
        let base = provider.backend_identity.len()
            + std::mem::size_of::<RequestIndex>()
            + std::mem::size_of::<TerminalValue>()
            + "kani-backend-1".len();
        let limits = ReplayLimits::default().with_input_bytes((base + 1) as u64);
        assert_eq!(
            read_backend_provider_envelope(&provider, limits),
            Err(ProofResultRefusal::BoundExceeded(BoundExceeded {
                bound: (base + 1) as u64,
                actual: (base + 2) as u128,
            }))
        );

        let mut huge_limit = replay.limit().unwrap().clone();
        huge_limit.next_charge = quire_exact::Integer::power_of_ten(DEFAULT_INPUT_BYTES as u64);
        let huge_replay = replay.with_limit(huge_limit);
        let huge_source = source(vec![TerminalRecord::new(
            RequestIndex::new(0),
            BackendId::new("kani-backend-1"),
            &huge_replay,
        )
        .unwrap()]);
        assert!(matches!(
            read_backend_provider_envelope(&huge_source, ReplayLimits::default()),
            Err(ProofResultRefusal::BoundExceeded(_))
        ));
    }

    /// Every variable-length member retained in a proof result increases
    /// the structural byte count when its own encoded content grows.
    #[trace("TC-178", "FR-069-AC-4")]
    #[trace("TC-191", "FR-072-AC-5")]
    #[test]
    fn retained_field_growth_increases_measured_size() {
        use qsl_foundation::source::provenance::OccurrenceKey;
        use qsl_foundation::witness::{RuntimeValuePath, ValuePathStep, ValuePathSubject};
        use quire_exact::{Integer, Origin, Role, Value};

        fn replay_source(replay: ReplayResult) -> BackendProviderSource {
            source(vec![TerminalRecord::new(
                RequestIndex::new(0),
                BackendId::new("kani-backend-1"),
                &replay,
            )
            .unwrap()])
        }

        fn witness_source(record: crate::SeparatingWitnessRecord) -> BackendProviderSource {
            let verdict = Verdict::from_category(Category::Violation);
            replay_source(ReplayResult::Witness(crate::WitnessArmResult::settle(
                verdict,
                verdict,
                Category::Violation,
                Some(crate::EvaluatedValue::Boolean(false)),
                crate::WitnessCheck::Agrees(Some(Box::new(record))),
                Vec::new(),
                crate::spine::default_accounting(0),
            )))
        }

        fn base_witness() -> crate::SeparatingWitnessRecord {
            let quantifier = OccurrenceKey::new(
                WireNodeId::from_digest([7; 32]),
                Origin::new(Role::new("q"), 0),
            );
            crate::SeparatingWitnessRecord {
                quantifier: quantifier.clone(),
                deciding_element: Value::Integer(Integer::from(1_u64)),
                index: Some(0),
                value_path: RuntimeValuePath {
                    observation: qsl_foundation::witness::ObservationIdentity {
                        authority: "a".to_owned(),
                        identity: "i".to_owned(),
                        revision_namespace: "n".to_owned(),
                        revision: "r".to_owned(),
                    },
                    subject: ValuePathSubject::Built(quantifier),
                    steps: vec![ValuePathStep::Member("m".to_owned())],
                },
                trace_position: Some(crate::TracePosition::new("t".to_owned())),
            }
        }

        let base = replay_source(ReplayResult::Witness(
            crate::WitnessArmResult::settle_range_violation(
                None,
                Vec::new(),
                crate::spine::default_accounting(0),
            ),
        ));
        let mut cases: Vec<(&str, usize, usize)> = Vec::new();
        let mut check = |name, before: &BackendProviderSource, after: &BackendProviderSource| {
            cases.push((
                name,
                measured_encoded_bytes(before),
                measured_encoded_bytes(after),
            ));
        };

        let mut grown = base.clone();
        grown.backend_identity.push('x');
        check("provider identity String", &base, &grown);
        let mut grown = base.clone();
        grown.items.push(TerminalRecord::tested(
            RequestIndex::new(1),
            BackendId::new("kani-backend-1"),
        ));
        check("terminal records Vec", &base, &grown);
        let mut grown = base.clone();
        grown.items[0].backend = Some(BackendId::new("kani-backend-1x"));
        check("record backend String", &base, &grown);

        let bare = source(vec![TerminalRecord::from_value(
            RequestIndex::new(0),
            TerminalValue::Refuted,
        )]);
        check("retained ReplayResult", &bare, &base);

        let short_cause = DisagreementCause::Witness {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Violation),
            given: None,
            derived: None,
            failure: crate::WitnessFailure::Separation {
                step: qsl_foundation::witness::SeparationStep::Body,
                reason: crate::SeparationReason::Refused(crate::SeparationRefusal {
                    code: "code".to_owned(),
                    cause: "x".to_owned(),
                    fields: std::collections::BTreeMap::new(),
                }),
            },
        };
        let mut longer_cause = short_cause.clone();
        if let DisagreementCause::Witness {
            failure:
                crate::WitnessFailure::Separation {
                    reason: crate::SeparationReason::Refused(crate::SeparationRefusal { cause, .. }),
                    ..
                },
            ..
        } = &mut longer_cause
        {
            cause.push('x');
        }
        let cause_source = |cause| {
            source(vec![TerminalRecord::from_value(
                RequestIndex::new(0),
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(cause)),
            )])
        };
        check(
            "terminal cause String",
            &cause_source(short_cause),
            &cause_source(longer_cause),
        );

        let short_region = refuted_source_with_region("r", "i".to_owned());
        let long_region = refuted_source_with_region("r", "ii".to_owned());
        check("replay source identity String", &short_region, &long_region);
        check(
            "replay source authority String",
            &short_region,
            &refuted_source_with_region("rr", "i".to_owned()),
        );
        check("resolved regions Vec", &base, &short_region);

        let witness = base_witness();
        check(
            "nested witness record",
            &base,
            &witness_source(witness.clone()),
        );
        type WitnessEdit = (&'static str, fn(&mut crate::SeparatingWitnessRecord));
        let witness_edits: &[WitnessEdit] = &[
            ("quantifier role String", |record| {
                record.quantifier = OccurrenceKey::new(
                    WireNodeId::from_digest([7; 32]),
                    Origin::new(Role::new("qq"), 0),
                );
            }),
            ("deciding Integer", |record| {
                record.deciding_element = Value::Integer(Integer::from(10_u64));
            }),
            ("observation authority String", |record| {
                record.value_path.observation.authority.push('x');
            }),
            ("observation identity String", |record| {
                record.value_path.observation.identity.push('x');
            }),
            ("observation namespace String", |record| {
                record.value_path.observation.revision_namespace.push('x');
            }),
            ("observation revision String", |record| {
                record.value_path.observation.revision.push('x');
            }),
            ("subject role String", |record| {
                record.value_path.subject = ValuePathSubject::Built(OccurrenceKey::new(
                    WireNodeId::from_digest([7; 32]),
                    Origin::new(Role::new("qq"), 0),
                ));
            }),
            ("value path Vec", |record| {
                record.value_path.steps.push(ValuePathStep::OptionValue);
            }),
            ("value path member String", |record| {
                record.value_path.steps[0] = ValuePathStep::Member("mm".to_owned());
            }),
            ("trace position String", |record| {
                record.trace_position = Some(crate::TracePosition::new("tt".to_owned()));
            }),
        ];
        for (name, edit) in witness_edits {
            let mut grown = witness.clone();
            edit(&mut grown);
            check(
                name,
                &witness_source(witness.clone()),
                &witness_source(grown),
            );
        }

        let region = SourceRegion::new(
            RawSourceRef::new(
                "a",
                "i",
                DigestRecord::mint(DigestDomain::SourceBytesV1, [1; 32]),
            )
            .unwrap(),
            0,
            1,
        )
        .unwrap();
        let input = |regions| {
            ReplayResult::Input(crate::InputArmResult::settle_range_violation(
                None,
                regions,
                crate::spine::default_accounting(0),
            ))
        };
        cases.push((
            "input resolved regions Vec",
            crate::result::measured_encoded_bytes(&input(Vec::new())),
            crate::result::measured_encoded_bytes(&input(vec![region])),
        ));

        let disagreeing = |cause: &str, is_witness: bool| {
            let verdict = Verdict::from_category(Category::Violation);
            let disagreement = crate::WitnessCheck::Disagrees {
                given: None,
                derived: None,
                failure: crate::WitnessFailure::Separation {
                    step: qsl_foundation::witness::SeparationStep::Body,
                    reason: crate::SeparationReason::Refused(crate::SeparationRefusal {
                        code: "code".to_owned(),
                        cause: cause.to_owned(),
                        fields: std::collections::BTreeMap::new(),
                    }),
                },
            };
            if is_witness {
                ReplayResult::Witness(crate::WitnessArmResult::settle(
                    verdict,
                    verdict,
                    Category::Violation,
                    Some(crate::EvaluatedValue::Boolean(false)),
                    disagreement,
                    Vec::new(),
                    crate::spine::default_accounting(0),
                ))
            } else {
                ReplayResult::Input(crate::InputArmResult::settle(
                    verdict,
                    verdict,
                    Category::Violation,
                    Some(crate::EvaluatedValue::Boolean(false)),
                    &disagreement,
                    Vec::new(),
                    crate::spine::default_accounting(0),
                ))
            }
        };
        for (name, is_witness) in [
            ("witness disagreement String", true),
            ("input disagreement String", false),
        ] {
            cases.push((
                name,
                crate::result::measured_encoded_bytes(&disagreeing("x", is_witness)),
                crate::result::measured_encoded_bytes(&disagreeing("xx", is_witness)),
            ));
        }

        for (name, is_witness) in [
            ("witness limit Integer", true),
            ("input limit Integer", false),
        ] {
            let replay = |digits: &str| {
                let limit = quire_exact::Incomplete {
                    limit_kind: quire_exact::LimitKind::WorkUnits,
                    limit: 1,
                    consumed: 1,
                    next_charge: digits.parse().unwrap(),
                    charge_point: quire_exact::ChargePoint::TextInputBytes,
                };
                if is_witness {
                    ReplayResult::Witness(
                        crate::WitnessArmResult::settle_range_violation(
                            None,
                            Vec::new(),
                            crate::spine::default_accounting(0),
                        )
                        .with_limit(limit),
                    )
                } else {
                    ReplayResult::Input(
                        crate::InputArmResult::settle_range_violation(
                            None,
                            Vec::new(),
                            crate::spine::default_accounting(0),
                        )
                        .with_limit(limit),
                    )
                }
            };
            cases.push((
                name,
                crate::result::measured_encoded_bytes(&replay("9")),
                crate::result::measured_encoded_bytes(&replay("99")),
            ));
        }

        for (name, before, after) in cases {
            assert!(after > before, "{name}: {before} -> {after}");
        }
    }

    /// FR-121-AC-17 (ADR-013 C-09): a call-site refusal before any backend
    /// run settles `Declined` with cause `InvalidInput` and its own code;
    /// `CallSiteRefusal::Fault` settles `Failed`.
    #[trace("TC-516", "FR-121-AC-17")]
    #[test]
    fn a_call_site_refusal_settles_declined_with_its_code_and_a_fault_failed() {
        use crate::identity::QualifiedName;
        use qsl_foundation::diagnostic::InternalFault;
        use qsl_foundation::digest::{DigestDomain, DigestRecord};
        use quire_exact::Identifier;

        let compile = CallSiteRefusal::Compile {
            code: Code::StaleDependency,
            message: "refused".to_owned(),
        };
        assert_eq!(
            TerminalValue::from_call_site_refusal(&compile),
            TerminalValue::Declined {
                cause: ProofRefusalCause::InvalidInput,
                code: DeclineCode::Qsl(Code::StaleDependency),
            }
        );
        let unknown = CallSiteRefusal::UnknownFunction {
            selection: QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
            package: DigestRecord::mint(DigestDomain::SourceBytesV1, [1; 32]),
        };
        let settled = TerminalValue::from_call_site_refusal(&unknown);
        assert_eq!(
            settled,
            TerminalValue::Declined {
                cause: ProofRefusalCause::InvalidInput,
                code: DeclineCode::Qsl(Code::MissingDeclaration),
            }
        );
        assert_eq!(settled.category(), Category::Refusal);

        let fault = CallSiteRefusal::Fault(InternalFault::new("call_site", "broken"));
        assert_eq!(
            TerminalValue::from_call_site_refusal(&fault),
            TerminalValue::Failed
        );
    }

    /// FR-069-AC-1 (TC-177): a `Declined` with a STD-001 code, even an
    /// unregistered one, builds, compares by the code, and is distinct from a
    /// QSL catalog code of the same cause: no code is remapped across
    /// registries.
    #[trace("TC-177", "FR-069-AC-1")]
    #[test]
    fn a_declined_value_carries_a_std001_code_without_remapping_it() {
        let parsed = Std001Code::new("kani_corpus_identity_collision").unwrap();
        assert!(!parsed.is_registered());

        let std001 = TerminalValue::Declined {
            cause: ProofRefusalCause::Refused,
            code: DeclineCode::Std001(parsed),
        };
        let same = TerminalValue::Declined {
            cause: ProofRefusalCause::Refused,
            code: DeclineCode::Std001(std001_code!("kani_corpus_identity_collision")),
        };
        let qsl = TerminalValue::Declined {
            cause: ProofRefusalCause::Refused,
            code: DeclineCode::Qsl(Code::MissingDeclaration),
        };
        assert_eq!(std001, same);
        assert_ne!(std001, qsl);
        assert_eq!(std001.category(), Category::Refusal);
        let record = TerminalRecord::from_value(RequestIndex::new(3), std001.clone());
        assert_eq!(record.value(), &std001);
    }

    /// FR-069-AC-4 (TC-178): an oversized replay-parity cause refuses even
    /// though the record's own fixed size is small -- the bound counts the
    /// cause's failure strings.
    #[trace("TC-178", "FR-069-AC-4")]
    #[test]
    fn tc_178_refuses_an_oversized_replay_parity_cause() {
        use crate::result::{SeparationReason, SeparationRefusal, WitnessFailure};
        use qsl_foundation::witness::SeparationStep;
        use std::collections::BTreeMap;

        let cause = DisagreementCause::Witness {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Violation),
            given: None,
            derived: None,
            failure: WitnessFailure::Separation {
                step: SeparationStep::Body,
                reason: SeparationReason::Refused(SeparationRefusal {
                    code: "invalid_runtime_input".to_owned(),
                    cause: "x".repeat(DEFAULT_INPUT_BYTES + 1),
                    fields: BTreeMap::new(),
                }),
            },
        };
        let oversized = source(vec![TerminalRecord::from_value(
            RequestIndex::new(0),
            TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(cause)),
        )]);
        assert!(matches!(
            read_backend_provider_envelope(&oversized, crate::ReplayLimits::default()),
            Err(ProofResultRefusal::BoundExceeded(_))
        ));
    }

    /// FR-069-AC-4 (TC-178), FR-357-AC-10: a scalar-agreement cause is
    /// measured by its bindings, so an oversized one refuses.
    #[trace("TC-178", "FR-069-AC-4", "FR-357-AC-10")]
    #[test]
    fn tc_178_refuses_an_oversized_scalar_agreement_cause() {
        let oversized = ScalarAgreement::new(
            function_claim(vec![
                CanonicalAssignment {
                    parameter: WireNodeId::from_digest([0; 32]),
                    value: WitnessValue::Integer(0),
                };
                DEFAULT_INPUT_BYTES / 40 + 1
            ]),
            ScalarOutcome::OtherRefusal,
        );
        let source = source(vec![TerminalRecord::from_value(
            RequestIndex::new(0),
            TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(oversized)),
        )]);
        assert!(matches!(
            read_backend_provider_envelope(&source, crate::ReplayLimits::default()),
            Err(ProofResultRefusal::BoundExceeded(_))
        ));
    }

    /// FR-069-AC-3 (TC-179): a positive envelope's construct -> serialize ->
    /// read round trip -- construct via [`read_backend_provider_envelope`],
    /// serialize via [`ProofResultEnvelope::to_source`] (#231 builds no
    /// byte-level wire serializer for the backend envelope, so this is the
    /// in-process shape that stands in for one), read via
    /// [`read_backend_provider_envelope`] again -- preserves the `backend`
    /// member and every per-item disposition byte for byte, with
    /// no re-derivation of the identity. N2: this is a real
    /// construct/serialize/read round trip through `to_source`, not two
    /// reads of the same untouched source.
    #[trace("TC-179", "FR-069-AC-3")]
    #[test]
    fn tc_179_round_trip_preserves_backend_and_dispositions() {
        let items = vec![
            TerminalRecord::from_value(
                RequestIndex::new(0),
                TerminalValue::Proved {
                    basis: ProofBasis::Checks { success_checks: 3 },
                    certification: Certification::Certified,
                },
            ),
            TerminalRecord::from_value(RequestIndex::new(1), TerminalValue::Refuted),
            TerminalRecord::from_value(
                RequestIndex::new(2),
                TerminalValue::Inconclusive(InconclusiveCause::ReplayRefused(
                    Code::StaleDependency,
                )),
            ),
            TerminalRecord::from_value(
                RequestIndex::new(3),
                TerminalValue::Inconclusive(InconclusiveCause::ScalarAgrees(agreement())),
            ),
        ];
        let original = source(items);
        let first =
            read_backend_provider_envelope(&original, crate::ReplayLimits::default()).unwrap();
        let serialized = ProofResultEnvelope::to_source(&first).unwrap();
        let second =
            read_backend_provider_envelope(&serialized, crate::ReplayLimits::default()).unwrap();
        assert_eq!(first, second);
        for envelope in &first {
            assert_eq!(envelope.backend().as_str(), "kani-backend-1");
        }
    }

    /// `to_source` on an empty slice returns [`EmptyEnvelopeSet`] instead of
    /// panicking: there is no envelope to take the `backend` from.
    #[test]
    fn to_source_refuses_an_empty_envelope_set() {
        assert_eq!(
            ProofResultEnvelope::to_source(&[]).unwrap_err(),
            EmptyEnvelopeSet
        );
    }
}

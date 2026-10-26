// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-182 (ADR-023 HV-1 to HV-8): the one settlement map from an EN-1 run's
//! [`HyperOutcome`] over an HP-2, HP-3 or HP-6 item to its terminal record.
//!
//! A `Violated` outcome settles `refuted` only on its replay (FR-183): the
//! caller's `replay` supplies the result, in production
//! [`replay_hyper_trace_tuple`](crate::replay_hyper_trace_tuple). A refused
//! replay settles `inconclusive` by `ReplayRefused`, or `failed` for a
//! fault, and carries no counterexample. Every other row is built by FR-127's
//! own row builders, so the two maps settle each shared row alike.
//!
//! [`settle_hyper_smt`] settles an EN-3 [`HyperSmtOutcome`]. Only the
//! `Inductive` and `InductionNotClosed` results are built; a non-empty `μ_U`
//! settles `VacuousMatch` until FR-183's match witness is built.
//!
//! A `Holds` or `HoldsReduced` outcome settles `proved` only after FR-163's product-closure
//! certificate checker accepts its certificate. Neither the certificate nor
//! the checker is built, so the map refuses them rather than settle a
//! `proved` no checker stands behind.

use crate::execute::ReplayRefusal;
use crate::model_check::hyper_counterexample::HyperCounterexample;
use crate::model_check::hyper_outcome::{BoundCovers, HyperOutcome, HyperReductionCause};
use crate::model_check::hyper_smt_outcome::{HyperSmtOutcome, HyperSmtResult};
use crate::model_check::hyper_smt_query::{HyperProductIdentity, HyperQueryRequest};
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits, ReachedLimit};
use crate::model_check::replay::ModelTraceSettlement;
use crate::model_check::settle::{Execution, ResultLabel, Row, Truth, EXPLICIT_STATE, K_INDUCTION};
use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::smt::SmtEncodingRefusal;
use qsl_semantics::check::HyperForm;

use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
    UnavailabilityCause,
};

/// Whether a limit's value came from the request or from its published
/// default (FR-184, FR-182).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LimitOrigin {
    /// The request set the limit.
    Requested,
    /// The request set none and the published default applied.
    PublishedDefault,
}

/// One limit of the run with the value it used.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UsedLimit {
    /// The limit.
    pub limit: ModelCheckLimit,
    /// Its value in the run.
    pub value: u64,
    /// Where the value came from.
    pub origin: LimitOrigin,
}

/// One hyper item's terminal record (FR-182 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperRecord {
    /// The terminal value.
    pub value: TerminalValue,
    /// Its QSpec FR-360 result label.
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
    /// What a V-5 record's horizon covers.
    pub covers: Option<BoundCovers>,
    /// Each limit of the run, FR-126's and FR-184's, with the value it used.
    pub limits: [UsedLimit; 5],
    /// The limit the run reached, when one stopped it.
    pub limit_reached: Option<ReachedLimit>,
    /// The count a counted budget reached (FR-184): one past its value.
    pub count: Option<u64>,
    /// The counterexample a `refuted` record carries.
    pub counterexample: Option<HyperCounterexample>,
}

/// FR-173, FR-182: a hyper item S3 classified into HP-4 settles
/// `unsupported`, `unsupported-requested-capability` at negotiation, naming its
/// rule, and no engine runs for it. `None` for every admitted form, which an
/// engine settles.
pub fn settle_hyper_form(form: &HyperForm) -> Option<TerminalValue> {
    form.unsupported_reason().map(|reason| {
        TerminalValue::Unsupported(UnavailabilityCause::UnsupportedHyperForm { reason })
    })
}

/// Why a hyper outcome settles no record.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HyperSettleRefusal {
    /// A `Holds` outcome settles `proved` only after FR-163's product-closure
    /// certificate checker accepts its certificate, and the certificate and
    /// the checker are not built.
    #[error("a hyper `Holds` settles only after FR-163's product-closure checker accepts its certificate, which is not built")]
    ProductClosureCheckerNotBuilt,
    /// A reduction whose preservation row reads "no" for a hyper form settles
    /// `ReductionNotPreserving` naming that form (ADR-023 section 5), and the
    /// cause's `form` member is a `PropertyForm`, which has no hyper member.
    ///
    /// Deferred: QSL-929. FR-182's V-6 row says this outcome is inconclusive
    /// `ReductionNotPreserving`; representing it needs an owner decision on the
    /// external `PropertyForm` type, so the refusal stands until then.
    #[error("a hyper `ReductionNotPreserving` names a hyper form, which the cause's `PropertyForm` member cannot hold")]
    ReductionFormNotRepresentable,
    /// An HP-2 SMT outcome whose product identity is not the item's is no
    /// evidence for the item (FR-182).
    #[error("the outcome answers another product: its {member} differs from the item's")]
    ProductMismatch {
        /// The first member that differs.
        member: ProductMember,
    },
    /// The item's own identity was refused (FR-315 "Refusals and their
    /// order").
    #[error("the item has no product identity: {0}")]
    ItemIdentity(SmtEncodingRefusal),
}

/// A member of an HP-2 product identity (FR-315 "Product identity").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProductMember {
    /// The clause.
    Clause,
    /// The ordered subjects.
    Subjects,
    /// The object-parameter instance.
    Instance,
}

impl std::fmt::Display for ProductMember {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Clause => "clause",
            Self::Subjects => "subjects",
            Self::Instance => "instance",
        })
    }
}

impl ProductMember {
    /// The first member, in the order clause, subjects, instance, in which
    /// the two identities differ.
    fn first_difference(a: &HyperProductIdentity, b: &HyperProductIdentity) -> Option<Self> {
        if a.clause() != b.clause() {
            Some(Self::Clause)
        } else if a.subjects() != b.subjects() {
            Some(Self::Subjects)
        } else if a.instance() != b.instance() {
            Some(Self::Instance)
        } else {
            None
        }
    }
}

/// FR-182: settle `outcome`, a run under `limits`, as one terminal record.
/// `requested` names the limits the request set; any other limit used its
/// published default. A `Violated` outcome's counterexample is handed to
/// `replay` (FR-183) and settles by its result: reproduced is `Refuted`; a
/// disagreement is `Inconclusive(ReplayParity)`; a replay a limit stopped is
/// `Incomplete(LimitReached)`; a refusal is `Inconclusive(ReplayRefused)`,
/// except an internal fault, which is `Failed`.
///
/// # Errors
///
/// [`HyperSettleRefusal`] for a `Holds` or `HoldsReduced` outcome, or a
/// reduction a hyper form's row refuses.
pub fn settle_hyper_check(
    outcome: &HyperOutcome,
    limits: &ModelCheckLimits,
    requested: &[ModelCheckLimit],
    replay: impl FnOnce(&HyperCounterexample) -> Result<ModelTraceSettlement, ReplayRefusal>,
) -> Result<HyperRecord, HyperSettleRefusal> {
    let mut counterexample = None;
    let mut depth = None;
    let mut covers = None;
    let mut count = None;
    let row = match outcome {
        HyperOutcome::Holds { .. } | HyperOutcome::HoldsReduced { .. } => {
            return Err(HyperSettleRefusal::ProductClosureCheckerNotBuilt)
        }
        HyperOutcome::Reduction { cause, .. } => match cause {
            HyperReductionCause::NotPreserving { .. } => {
                return Err(HyperSettleRefusal::ReductionFormNotRepresentable)
            }
            HyperReductionCause::Unsupported { reduction } => {
                Row::unsupported(UnavailabilityCause::UnsupportedReduction {
                    reduction: *reduction,
                })
            }
        },
        HyperOutcome::Violated {
            counterexample: payload,
            ..
        } => match replay(payload) {
            Ok(ModelTraceSettlement::Reproduced { .. }) => {
                counterexample = Some(payload.as_ref().clone());
                Row::refuted(SettlementBasis::DecisiveCounterexample)
            }
            Ok(ModelTraceSettlement::Inconclusive(cause)) => {
                Row::inconclusive(InconclusiveCause::ReplayParity(cause))
            }
            Ok(ModelTraceSettlement::Stopped {
                reached,
                count: held,
            }) => {
                count = held;
                Row::stopped_run(IncompleteCause::ResourceExhausted, Some(reached))
            }
            Err(refusal) => Row::refused(&refusal),
        },
        HyperOutcome::BoundReached {
            depth: reached,
            covers: reading,
            ..
        } => {
            depth = Some(*reached);
            covers = Some(*reading);
            Row::inconclusive(InconclusiveCause::BoundReached { depth: *reached })
        }
        HyperOutcome::Undecided { cause, .. } => Row::inconclusive(cause.inconclusive()),
        HyperOutcome::Stopped {
            cause,
            limit,
            count: reached,
            ..
        } => {
            count = *reached;
            Row::stopped_run(*cause, *limit)
        }
    };
    Ok(HyperRecord {
        value: row.value,
        label: row.label,
        basis: row.basis,
        execution: row.execution,
        truth: row.truth,
        method: EXPLICIT_STATE,
        depth,
        covers,
        limits: used(limits, requested),
        limit_reached: row.limit_reached,
        count,
        counterexample,
    })
}

/// Each limit of the run with its value and origin, in member order.
fn used(limits: &ModelCheckLimits, requested: &[ModelCheckLimit]) -> [UsedLimit; 5] {
    let [states, transitions, automaton] = limits.used();
    let origin = |limit| {
        if requested.contains(&limit) {
            LimitOrigin::Requested
        } else {
            LimitOrigin::PublishedDefault
        }
    };
    [
        states,
        transitions,
        automaton,
        (ModelCheckLimit::MaxWitnessSet, limits.max_witness_set),
        (
            ModelCheckLimit::MaxRelationTuples,
            limits.max_relation_tuples,
        ),
    ]
    .map(|(limit, value)| UsedLimit {
        limit,
        value,
        origin: origin(limit),
    })
}

/// FR-182: settle `outcome`, an EN-3 k-induction run over the HP-2 `item`, as
/// one terminal record with method `k-induction`.
///
/// The outcome must carry the item's own product identity. An
/// `Inductive{depth}` outcome settles V-3, `proved`, `decisive-witness`,
/// `Uncertified`, when every subject has an initial state and the clause's
/// `Î¼_U` is empty; no match witness is built yet, so a non-empty `Î¼_U`
/// settles `VacuousMatch`. An `InductionNotClosed{depth}` outcome settles
/// `inconclusive`.
///
/// # Errors
///
/// [`HyperSettleRefusal::ItemIdentity`] when the item has no identity, and
/// [`HyperSettleRefusal::ProductMismatch`] when the outcome answers another
/// product. An internal fault in computing the identity settles `failed`.
pub fn settle_hyper_smt(
    item: &HyperQueryRequest<'_>,
    outcome: &HyperSmtOutcome,
    limits: &ModelCheckLimits,
    requested: &[ModelCheckLimit],
) -> Result<HyperRecord, HyperSettleRefusal> {
    let row = match HyperProductIdentity::of(item) {
        Err(refusal) if refusal.code().is_none() => Row::failed(),
        Err(refusal) => return Err(HyperSettleRefusal::ItemIdentity(refusal)),
        Ok(identity) => {
            if let Some(member) = ProductMember::first_difference(&identity, &outcome.product) {
                return Err(HyperSettleRefusal::ProductMismatch { member });
            }
            match outcome.result {
                HyperSmtResult::InductionNotClosed { depth } => {
                    Row::inconclusive(InconclusiveCause::InductionNotClosed { depth })
                }
                HyperSmtResult::Inductive { depth } => inductive_row(item, depth),
            }
        }
    };
    Ok(HyperRecord {
        value: row.value,
        label: row.label,
        basis: row.basis,
        execution: row.execution,
        truth: row.truth,
        method: K_INDUCTION,
        depth: None,
        covers: None,
        limits: used(limits, requested),
        limit_reached: row.limit_reached,
        count: None,
        counterexample: None,
    })
}

/// V-3 and its guards for an `Inductive{depth}` outcome, in FR-182's order.
fn inductive_row(item: &HyperQueryRequest<'_>, depth: u64) -> Row {
    for subject in item.subjects {
        match ModelSystem::new(subject.simulation.clone()) {
            Ok(system) if system.initial().is_empty() => {
                return Row::inconclusive(InconclusiveCause::NoInitialState)
            }
            Ok(_) => {}
            Err(_) => return Row::failed(),
        }
    }
    if !item.clause.mu_universal().is_empty() {
        return Row::inconclusive(InconclusiveCause::VacuousMatch);
    }
    Row::proof(
        ProofBasis::Inductive { depth },
        Certification::Uncertified,
        SettlementBasis::DecisiveWitness,
    )
}

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
//! A `Holds` outcome settles `proved` only after FR-163's product-closure
//! certificate checker accepts its certificate. Neither the certificate nor
//! the checker is built, so the map refuses a `Holds` rather than settle a
//! `proved` no checker stands behind.

use crate::execute::ReplayRefusal;
use crate::model_check::hyper_counterexample::HyperCounterexample;
use crate::model_check::hyper_outcome::{BoundCovers, HyperOutcome};
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits, ReachedLimit};
use crate::model_check::replay::ModelTraceSettlement;
use crate::model_check::settle::{Execution, ResultLabel, Row, Truth, EXPLICIT_STATE};
use crate::proof_result::{IncompleteCause, InconclusiveCause, SettlementBasis, TerminalValue};

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

/// Why a hyper outcome settles no record.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HyperSettleRefusal {
    /// A `Holds` outcome settles `proved` only after FR-163's product-closure
    /// certificate checker accepts its certificate, and the certificate and
    /// the checker are not built.
    #[error("a hyper `Holds` settles only after FR-163's product-closure checker accepts its certificate, which is not built")]
    ProductClosureCheckerNotBuilt,
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
/// [`HyperSettleRefusal`] for a `Holds` outcome.
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
        HyperOutcome::Holds { .. } => {
            return Err(HyperSettleRefusal::ProductClosureCheckerNotBuilt)
        }
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
            Ok(ModelTraceSettlement::Stopped(reached)) => {
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

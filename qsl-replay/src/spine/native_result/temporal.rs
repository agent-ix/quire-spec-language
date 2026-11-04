// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-267 "Temporal disposition wire": the closed `disposition.temporal`
//! member of a `Temporal` selection's stage `evaluate` disposition, its
//! writer and its strict reader.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use super::{
    Basis, CallLocationRead, CallLocationWire, Category, DeclarationKeyRead, DispositionRead,
    DocumentRefRead, DocumentRefWire, NativeRunReadError, NativeRunResultError, Outcome,
    OutcomeRead, Stage,
};
use crate::model_check::counterexample::{ObservedTemporalCounterexample, ObservedTemporalKind};
use crate::spine::{
    ContradictedDimension, ObservedPosition, ProgressClosure, ProgressCompleteness,
    ProgressContradiction, ProgressLimitReached, ProgressRefusal, ProgressValue,
    TemporalIndeterminateReason, TemporalPositionRefusal, TemporalTraceRefusal,
    TemporalUndefinedCause, TemporalVerdict, TemporalWindowRefusal,
};
use qsl_eval::temporal::ObservedLocus;

/// The largest integer RFC 8785 serializes exactly (QSpec's `Count` bound).
const MAX_COUNT: u64 = 9_007_199_254_740_991;

/// The incomplete limit a window aggregate reports (FR-194).
const MAX_WINDOW_VALUES: &str = "max_window_values";

/// The undefined reason of a window aggregate short of data (FR-414).
const INSUFFICIENT_DATA: &str = "insufficient-data";

/// The setting that raises `max_window_values`.
const MAX_WINDOW_VALUES_SETTING: &str = "monitor.max_window_values";

/// FR-502's covered-position limit and its setting.
const COVERED_POSITIONS_LIMIT: &str = "node count";
const COVERED_POSITIONS_SETTING: &str = "progress.covered_positions";

/// The undefined cause an FR-100 `reason` names: a kernel reason, or a
/// family reason with the empty payload (the outcome carries no `fields`).
fn undefined_cause(reason: &str) -> Option<TemporalUndefinedCause> {
    use quire_exact::Undefined;
    use quire_semantic_value::catalog::{UndefinedReason, UndefinedRecord};
    let kernel = [
        Undefined::DivisionByZero,
        Undefined::IeeeNotFinite,
        Undefined::EmptyReduction,
        Undefined::NoneValue,
        Undefined::SumOutOfDomain,
    ]
    .into_iter()
    .find(|undefined| crate::spine::kernel_undefined_reason(undefined.clone()) == reason)
    .map(TemporalUndefinedCause::Kernel);
    kernel.or_else(|| {
        [
            UndefinedReason::PreconditionFalse,
            UndefinedReason::AbsentKey,
            UndefinedReason::IncompleteClosure,
            UndefinedReason::OnlyNotOne,
        ]
        .into_iter()
        .find(|family| family.as_str() == reason)
        .map(|reason| {
            TemporalUndefinedCause::Family(UndefinedRecord {
                reason,
                fields: BTreeMap::new(),
            })
        })
    })
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum TemporalWire {
    Holds,
    Violated {
        position: u64,
        counterexample: CounterexampleWire,
    },
    Undefined {
        position: u64,
        counterexample: CounterexampleWire,
    },
    InsufficientData {
        position: u64,
        count: u64,
        min_count: u64,
    },
    Pending,
    PendingWindow {
        position: u64,
        deadline: u64,
    },
    PendingNextEvent {
        position: u64,
    },
    Indeterminate {
        position: u64,
        reason: ReasonWire,
    },
    MissingFairnessPremise {
        constraint: Box<RawValue>,
    },
    Incomplete,
    ActivationIncomplete {
        position: u64,
        guard: CallLocationWire,
    },
    WindowLimitReached {
        limit: &'static str,
        value: u64,
        setting: &'static str,
    },
    ProgressLimitReached {
        limit: &'static str,
        value: u64,
        setting: &'static str,
    },
    ProgressContradicted {
        dimension: &'static str,
        clock: String,
        retained: ProgressValueWire,
        asserted: ProgressValueWire,
    },
    TraceRefused {
        variant: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        expected: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        found: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        profile: Option<&'static str>,
        #[serde(skip_serializing_if = "Option::is_none")]
        position: Option<u64>,
    },
}

/// FR-501's indeterminate reason as the result report spells it.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(crate) enum ReasonWire {
    MissingValuation {
        position: u64,
        atom: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        instance: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        also_held_by: Option<Vec<String>>,
    },
    ConflictingValuation {
        position: u64,
        atom: String,
    },
    IncompleteInput,
    MissingHistory,
    StraddlesBound,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub(super) enum ProgressValueWire {
    Watermark { watermark: u64 },
    Complete { through: u64 },
    Incomplete,
    DecisionScope { closure: &'static str },
    SurroundingExecution { closure: &'static str },
    Position { identity: String, stamp: u64 },
}

#[derive(Serialize)]
pub(super) struct CounterexampleWire {
    trace: &'static str,
    kind: CounterexampleKindWire,
    initial_state: (),
    prefix: Vec<StepWire>,
    #[serde(rename = "loop")]
    loop_steps: Vec<StepWire>,
    over_binding: (),
    fairness: Vec<Box<RawValue>>,
    interval: Option<IntervalWire>,
    trace_position: u64,
}

#[derive(Serialize)]
#[serde(untagged)]
enum CounterexampleKindWire {
    Formula(&'static str),
    Undefined(EnvelopeWire),
}

#[derive(Serialize)]
struct EnvelopeWire {
    #[serde(rename = "type")]
    envelope: &'static str,
    #[serde(rename = "where")]
    point: WhereWire,
    cause: &'static str,
}

#[derive(Serialize)]
struct WhereWire {
    position: u64,
    document: DocumentRefWire,
    locus: LocusWire,
}

/// The undefined expression's locus: a `holds` atom by its formula node, the
/// `on each` activation guard, or a capture's initializer by its name.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
enum LocusWire {
    FormulaNode(usize),
    Guard(&'static str),
    Capture(String),
}

/// The `guard` locus value: the clause's one `on each` guard.
const ON_EACH: &str = "on-each";

#[derive(Serialize)]
struct StepWire {
    document: DocumentRefWire,
}

#[derive(Serialize)]
struct IntervalWire {
    lower: String,
    upper: String,
}

/// The canonical FR-364 fairness member `{kind, operation, granularity}`.
fn fairness_wire(
    constraint: &quire_semantic_value::fairness::FairnessConstraint,
) -> Result<Box<RawValue>, NativeRunResultError> {
    let bytes = quire_canonical::to_vec(constraint, quire_canonical::Limits::new(u64::MAX))?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// The undefined value's FR-100 reason.
fn undefined_reason(cause: &TemporalUndefinedCause) -> &'static str {
    cause.reason()
}

fn counterexample_wire(
    counterexample: &ObservedTemporalCounterexample,
    reason: Option<&'static str>,
) -> Result<CounterexampleWire, NativeRunResultError> {
    let steps: Vec<StepWire> = counterexample
        .prefix
        .iter()
        .map(|step| StepWire {
            document: DocumentRefWire::from(&step.document),
        })
        .collect();
    let split = counterexample
        .loop_start
        .and_then(|start| usize::try_from(start).ok())
        .unwrap_or(steps.len())
        .min(steps.len());
    let mut prefix = steps;
    let loop_steps = prefix.split_off(split);
    let position = counterexample.trace_position.index();
    let kind = match (&counterexample.kind, reason) {
        (ObservedTemporalKind::Formula, None) => CounterexampleKindWire::Formula("formula"),
        (ObservedTemporalKind::UndefinedEvaluation { locus, .. }, Some(cause)) => {
            let document = usize::try_from(position)
                .ok()
                .and_then(|index| counterexample.prefix.get(index))
                .ok_or(NativeRunResultError::Counterexample)?;
            CounterexampleKindWire::Undefined(EnvelopeWire {
                envelope: "undefined-evaluation",
                point: WhereWire {
                    position,
                    document: DocumentRefWire::from(&document.document),
                    locus: match locus {
                        ObservedLocus::Atom(atom) => LocusWire::FormulaNode(atom.index()),
                        ObservedLocus::Activation => LocusWire::Guard(ON_EACH),
                        ObservedLocus::Capture(name) => LocusWire::Capture(name.clone()),
                    },
                },
                cause,
            })
        }
        _ => return Err(NativeRunResultError::Counterexample),
    };
    Ok(CounterexampleWire {
        trace: "observed",
        kind,
        initial_state: (),
        prefix,
        loop_steps,
        over_binding: (),
        fairness: counterexample
            .fairness
            .iter()
            .map(fairness_wire)
            .collect::<Result<_, _>>()?,
        interval: counterexample.interval.map(|interval| IntervalWire {
            lower: interval.lower().to_string(),
            upper: match interval.upper() {
                qsl_foundation::bound::TemporalUpper::Finite(upper) => upper.to_string(),
                qsl_foundation::bound::TemporalUpper::Open => "*".to_owned(),
            },
        }),
        trace_position: position,
    })
}

fn closure(value: ProgressClosure) -> &'static str {
    match value {
        ProgressClosure::Closed => "closed",
        ProgressClosure::Open => "open",
    }
}

fn progress_value_wire(value: &ProgressValue) -> ProgressValueWire {
    match value {
        ProgressValue::Watermark(watermark) => ProgressValueWire::Watermark {
            watermark: *watermark,
        },
        ProgressValue::Completeness(ProgressCompleteness::Complete { through }) => {
            ProgressValueWire::Complete { through: *through }
        }
        ProgressValue::Completeness(ProgressCompleteness::Incomplete) => {
            ProgressValueWire::Incomplete
        }
        ProgressValue::DecisionScope(value) => ProgressValueWire::DecisionScope {
            closure: closure(*value),
        },
        ProgressValue::SurroundingExecution(value) => ProgressValueWire::SurroundingExecution {
            closure: closure(*value),
        },
        ProgressValue::Position(position) => ProgressValueWire::Position {
            identity: position.identity.clone(),
            stamp: position.stamp,
        },
    }
}

fn dimension(value: ContradictedDimension) -> &'static str {
    match value {
        ContradictedDimension::Watermark => "watermark",
        ContradictedDimension::Completeness => "completeness",
        ContradictedDimension::Closure => "closure",
    }
}

/// The `variant` spelling and payload of a trace refusal other than `Fault`.
fn trace_variant(refusal: &TemporalTraceRefusal) -> Option<TemporalWire> {
    let wire = |variant| TemporalWire::TraceRefused {
        variant,
        expected: None,
        found: None,
        profile: None,
        position: None,
    };
    Some(match refusal {
        TemporalTraceRefusal::EmptyTrace => wire("empty-trace"),
        TemporalTraceRefusal::TupleArity { expected, found } => TemporalWire::TraceRefused {
            variant: "tuple-arity",
            expected: Some(*expected),
            found: Some(*found),
            profile: None,
            position: None,
        },
        TemporalTraceRefusal::AlignSkipNotLockstep => wire("align-skip-not-lockstep"),
        TemporalTraceRefusal::NotProjectionAligned => wire("not-projection-aligned"),
        TemporalTraceRefusal::EmptyLoop => wire("empty-loop"),
        TemporalTraceRefusal::TraceTooLong => wire("trace-too-long"),
        TemporalTraceRefusal::LassoUnderBoundedProfile { profile } => TemporalWire::TraceRefused {
            variant: "lasso-under-bounded-profile",
            expected: None,
            found: None,
            profile: Some(profile.identity()),
            position: None,
        },
        TemporalTraceRefusal::ProfileNotEvaluated { profile } => TemporalWire::TraceRefused {
            variant: "profile-not-evaluated",
            expected: None,
            found: None,
            profile: Some(profile.identity()),
            position: None,
        },
        TemporalTraceRefusal::Position(TemporalPositionRefusal::NotDecimal) => {
            wire("position-not-decimal")
        }
        TemporalTraceRefusal::Position(TemporalPositionRefusal::LeadingZero) => {
            wire("position-leading-zero")
        }
        TemporalTraceRefusal::Position(TemporalPositionRefusal::Overflow) => {
            wire("position-overflow")
        }
        TemporalTraceRefusal::Position(TemporalPositionRefusal::OutsideTrace { position }) => {
            positioned("position-outside-trace", *position)
        }
        TemporalTraceRefusal::Window(TemporalWindowRefusal::OutsideTrace { position }) => {
            positioned("window-outside-trace", *position)
        }
        TemporalTraceRefusal::Window(TemporalWindowRefusal::WindowKindMismatch) => {
            wire("window-kind-mismatch")
        }
        TemporalTraceRefusal::Window(TemporalWindowRefusal::TimestampsDecrease) => {
            wire("timestamps-decrease")
        }
        TemporalTraceRefusal::SameStampOrderUnavailable { position } => {
            positioned("same-stamp-order-unavailable", *position)
        }
        TemporalTraceRefusal::Fault(_) => return None,
    })
}

fn positioned(variant: &'static str, position: u64) -> TemporalWire {
    TemporalWire::TraceRefused {
        variant,
        expected: None,
        found: None,
        profile: None,
        position: Some(position),
    }
}

impl From<&TemporalIndeterminateReason> for ReasonWire {
    fn from(reason: &TemporalIndeterminateReason) -> Self {
        match reason {
            TemporalIndeterminateReason::MissingValuation {
                position,
                atom,
                instance,
                also_held_by,
            } => Self::MissingValuation {
                position: *position,
                atom: atom.clone(),
                instance: instance.clone(),
                also_held_by: instance.as_ref().map(|_| also_held_by.clone()),
            },
            TemporalIndeterminateReason::ConflictingValuation { position, atom } => {
                Self::ConflictingValuation {
                    position: *position,
                    atom: atom.clone(),
                }
            }
            TemporalIndeterminateReason::IncompleteInput => Self::IncompleteInput,
            TemporalIndeterminateReason::MissingHistory => Self::MissingHistory,
            TemporalIndeterminateReason::StraddlesBound => Self::StraddlesBound,
        }
    }
}

/// The `temporal` member and `outcome` of a verdict's table row.
pub(super) fn verdict_wire(
    verdict: &TemporalVerdict,
    counterexample: Option<&ObservedTemporalCounterexample>,
) -> Result<(TemporalWire, Option<Outcome>), NativeRunResultError> {
    let checked = |at: u64, reason| {
        let counterexample = counterexample.ok_or(NativeRunResultError::Counterexample)?;
        if counterexample.trace_position.index() != at {
            return Err(NativeRunResultError::Counterexample);
        }
        counterexample_wire(counterexample, reason)
    };
    Ok(match verdict {
        TemporalVerdict::Holds => (TemporalWire::Holds, None),
        TemporalVerdict::Violated { at } => (
            TemporalWire::Violated {
                position: *at,
                counterexample: checked(*at, None)?,
            },
            None,
        ),
        TemporalVerdict::Undefined {
            at,
            cause: TemporalUndefinedCause::InsufficientData { count, min_count },
        } => (
            TemporalWire::InsufficientData {
                position: *at,
                count: *count,
                min_count: *min_count,
            },
            Some(Outcome::Undefined {
                reason: INSUFFICIENT_DATA,
            }),
        ),
        TemporalVerdict::Undefined { at, cause } => {
            let reason = undefined_reason(cause);
            (
                TemporalWire::Undefined {
                    position: *at,
                    counterexample: checked(*at, Some(reason))?,
                },
                Some(Outcome::Undefined { reason }),
            )
        }
        TemporalVerdict::Pending => (TemporalWire::Pending, None),
        TemporalVerdict::PendingWindow { at, deadline } => (
            TemporalWire::PendingWindow {
                position: *at,
                deadline: *deadline,
            },
            None,
        ),
        TemporalVerdict::PendingNextEvent { at } => {
            (TemporalWire::PendingNextEvent { position: *at }, None)
        }
        TemporalVerdict::Indeterminate { at, reason } => (
            TemporalWire::Indeterminate {
                position: *at,
                reason: ReasonWire::from(reason),
            },
            None,
        ),
        TemporalVerdict::MissingFairnessPremise { constraint } => (
            TemporalWire::MissingFairnessPremise {
                constraint: fairness_wire(constraint)?,
            },
            Some(Outcome::Refused {
                code: "unsupported_projection",
                cause: "missing-fairness-premise",
                fields: None,
                locus: None,
                location: None,
            }),
        ),
        TemporalVerdict::Incomplete(incomplete) => (
            TemporalWire::Incomplete,
            Some(Outcome::Incomplete {
                limit: incomplete.limit_kind.as_str(),
            }),
        ),
        TemporalVerdict::ActivationIncomplete { at, guard } => (
            TemporalWire::ActivationIncomplete {
                position: *at,
                guard: CallLocationWire::from(guard),
            },
            None,
        ),
        TemporalVerdict::WindowLimitReached {
            limit,
            value,
            setting,
        } => (
            TemporalWire::WindowLimitReached {
                limit,
                value: *value,
                setting,
            },
            Some(Outcome::Incomplete {
                limit: MAX_WINDOW_VALUES,
            }),
        ),
        TemporalVerdict::ProgressLimitReached(reached) => (
            TemporalWire::ProgressLimitReached {
                limit: reached.limit,
                value: reached.value,
                setting: reached.setting,
            },
            None,
        ),
    })
}

/// The `temporal` member and `outcome` of a trace refusal, or `None` for a
/// `Fault`, which is written as an internal failure.
pub(super) fn refusal_wire(refusal: &TemporalTraceRefusal) -> Option<(TemporalWire, Outcome)> {
    let wire = trace_variant(refusal)?;
    let code = refusal.catalog_code();
    Some((
        wire,
        Outcome::Refused {
            code: code.code(),
            cause: code.cause(),
            fields: None,
            locus: None,
            location: None,
        },
    ))
}

/// The `temporal` member and `outcome` of an FR-502 contradiction.
pub(super) fn contradiction_wire(refusal: &ProgressRefusal) -> (TemporalWire, Outcome) {
    let contradiction = &refusal.contradiction;
    (
        TemporalWire::ProgressContradicted {
            dimension: dimension(contradiction.dimension),
            clock: contradiction.clock.clone(),
            retained: progress_value_wire(&contradiction.retained),
            asserted: progress_value_wire(&contradiction.asserted),
        },
        Outcome::Refused {
            code: refusal.record.catalog.code(),
            cause: refusal.record.catalog.cause(),
            fields: Some(refusal.record.fields.clone()),
            locus: None,
            location: None,
        },
    )
}

// ---------------------------------------------------------------- reader

/// The strict reader's typed `disposition.temporal` member (FR-267). Its
/// wire form is a closed object tagged by `kind`; [`decode`] reads it with
/// every failing member's path.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub enum TemporalRecord {
    /// `Completed(true)`.
    Holds {},
    /// `Completed(false)` at `position`.
    Violated {
        /// The failing trace position.
        position: u64,
        /// The observed-arm counterexample.
        counterexample: CounterexampleRead,
    },
    /// An `Undefined` that is a violation.
    Undefined {
        /// The first undefined position.
        position: u64,
        /// The observed-arm counterexample.
        counterexample: CounterexampleRead,
    },
    /// A window aggregate held fewer values than its `min_count`.
    InsufficientData {
        /// The activation position.
        position: u64,
        /// The values the window held.
        count: u64,
        /// The term's `min_count`.
        min_count: u64,
    },
    /// A finite prefix that can show no more.
    Pending {},
    /// A finite-window activation whose deadline is still ahead.
    PendingWindow {
        /// The activation position.
        position: u64,
        /// The first watermark tick that can establish the activation.
        deadline: u64,
    },
    /// An event-position activation that only a further event resolves.
    PendingNextEvent {
        /// The activation position.
        position: u64,
    },
    /// A closed finite window whose evidence leaves the truth unresolved.
    Indeterminate {
        /// The activation position.
        position: u64,
        /// Why the evidence is unresolved.
        reason: IndeterminateReasonRead,
    },
    /// The trace carries no enabledness for the clause's fairness set.
    MissingFairnessPremise {
        /// The first constraint of the fairness set.
        constraint: FairnessRead,
    },
    /// The work budget ran out.
    Incomplete {},
    /// An `on each` guard could not be established.
    ActivationIncomplete {
        /// The guard's trace position.
        position: u64,
        /// The guard's checked location.
        guard: CallLocationRead,
    },
    /// A window aggregate outgrew `max_window_values`.
    WindowLimitReached {
        /// The limit's name.
        limit: String,
        /// The limit's value in the run.
        value: u64,
        /// The setting that raises it.
        setting: String,
    },
    /// The progress ledger outgrew `progress.covered_positions`.
    ProgressLimitReached {
        /// The limit's name.
        limit: String,
        /// The capacity in the run.
        value: u64,
        /// The setting that raises it.
        setting: String,
    },
    /// The supplied progress contradicts the retained ledger.
    ProgressContradicted {
        /// The contradicted dimension.
        dimension: DimensionRead,
        /// The binding's clock name.
        clock: String,
        /// The retained value.
        retained: ProgressValueRead,
        /// The asserted value.
        asserted: ProgressValueRead,
    },
    /// The evaluator refused the supplied trace.
    TraceRefused {
        /// The `TemporalRefusal` variant spelling.
        variant: String,
        /// `tuple-arity`: the clause's trace variables.
        #[serde(default, deserialize_with = "super::present")]
        expected: Option<usize>,
        /// `tuple-arity`: the components supplied.
        #[serde(default, deserialize_with = "super::present")]
        found: Option<usize>,
        /// The profile identity of a profile refusal.
        #[serde(default, deserialize_with = "super::present")]
        profile: Option<String>,
        /// The refused position of a position or window refusal.
        #[serde(default, deserialize_with = "super::present")]
        position: Option<u64>,
    },
}

/// FR-501's indeterminate reason as read.
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum IndeterminateReasonRead {
    /// A required atom valuation is absent.
    MissingValuation {
        /// Position containing the absent valuation.
        position: u64,
        /// The atom's source text.
        atom: String,
        /// The assessed instance of an instance-bound atom.
        #[serde(default, deserialize_with = "super::present")]
        instance: Option<String>,
        /// The other instances holding a valuation, beside `instance`.
        #[serde(default, deserialize_with = "super::present")]
        also_held_by: Option<Vec<String>>,
    },
    /// Conflicting valuations at one position.
    ConflictingValuation {
        /// Position containing conflicting values.
        position: u64,
        /// The atom's source text.
        atom: String,
    },
    /// The completeness assertion does not cover the window.
    IncompleteInput {},
    /// A past operator reaches before a history cutoff.
    MissingHistory {},
    /// A counter cover straddles an interval boundary.
    StraddlesBound {},
}

impl IndeterminateReasonRead {
    /// The typed reason this record restores.
    pub fn typed(&self) -> TemporalIndeterminateReason {
        match self {
            Self::MissingValuation {
                position,
                atom,
                instance,
                also_held_by,
            } => TemporalIndeterminateReason::MissingValuation {
                position: *position,
                atom: atom.clone(),
                instance: instance.clone(),
                also_held_by: also_held_by.clone().unwrap_or_default(),
            },
            Self::ConflictingValuation { position, atom } => {
                TemporalIndeterminateReason::ConflictingValuation {
                    position: *position,
                    atom: atom.clone(),
                }
            }
            Self::IncompleteInput {} => TemporalIndeterminateReason::IncompleteInput,
            Self::MissingHistory {} => TemporalIndeterminateReason::MissingHistory,
            Self::StraddlesBound {} => TemporalIndeterminateReason::StraddlesBound,
        }
    }
}

/// FR-502's contradicted dimension as read.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DimensionRead {
    /// A regressing watermark.
    Watermark,
    /// A retreating completeness assertion or an uncovered position.
    Completeness,
    /// A reopened closure.
    Closure,
}

/// One side of an FR-502 contradiction as read.
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case", deny_unknown_fields)]
pub enum ProgressValueRead {
    /// A watermark.
    Watermark {
        /// The watermark tick.
        watermark: u64,
    },
    /// A completeness assertion through `through`.
    Complete {
        /// The last covered tick.
        through: u64,
    },
    /// No completeness asserted.
    Incomplete {},
    /// The decision scope's closure.
    DecisionScope {
        /// `closed` or `open`.
        closure: ClosureRead,
    },
    /// The surrounding execution's closure.
    SurroundingExecution {
        /// `closed` or `open`.
        closure: ClosureRead,
    },
    /// A position the retained assertion did not cover.
    Position {
        /// The position's identity.
        identity: String,
        /// The position's stamp.
        stamp: u64,
    },
}

/// A closure as read.
#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClosureRead {
    /// Nothing more will be added.
    Closed,
    /// More may still be added.
    Open,
}

impl ClosureRead {
    fn typed(self) -> ProgressClosure {
        match self {
            Self::Closed => ProgressClosure::Closed,
            Self::Open => ProgressClosure::Open,
        }
    }
}

impl ProgressValueRead {
    fn typed(&self) -> ProgressValue {
        match self {
            Self::Watermark { watermark } => ProgressValue::Watermark(*watermark),
            Self::Complete { through } => {
                ProgressValue::Completeness(ProgressCompleteness::Complete { through: *through })
            }
            Self::Incomplete {} => ProgressValue::Completeness(ProgressCompleteness::Incomplete),
            Self::DecisionScope { closure } => ProgressValue::DecisionScope(closure.typed()),
            Self::SurroundingExecution { closure } => {
                ProgressValue::SurroundingExecution(closure.typed())
            }
            Self::Position { identity, stamp } => ProgressValue::Position(ObservedPosition {
                identity: identity.clone(),
                stamp: *stamp,
            }),
        }
    }
}

impl TemporalRecord {
    /// The typed FR-502 contradiction a `progress-contradicted` record
    /// restores.
    pub fn contradiction(&self) -> Option<ProgressContradiction> {
        let Self::ProgressContradicted {
            dimension,
            clock,
            retained,
            asserted,
        } = self
        else {
            return None;
        };
        Some(ProgressContradiction {
            dimension: match dimension {
                DimensionRead::Watermark => ContradictedDimension::Watermark,
                DimensionRead::Completeness => ContradictedDimension::Completeness,
                DimensionRead::Closure => ContradictedDimension::Closure,
            },
            clock: clock.clone(),
            retained: retained.typed(),
            asserted: asserted.typed(),
        })
    }

    /// The typed verdict a verdict row restores, equal to the one written.
    /// `None` for `progress-contradicted` and `trace-refused` (see
    /// [`Self::contradiction`] and [`Self::trace_refusal`]) and for
    /// `incomplete`, whose FR-100 outcome carries the limit kind only, not
    /// the budget's counters. A family undefined cause restores with an
    /// empty `fields`: the outcome carries its reason only.
    pub fn verdict(&self) -> Option<TemporalVerdict> {
        Some(match self {
            Self::Holds {} => TemporalVerdict::Holds,
            Self::Violated { position, .. } => TemporalVerdict::Violated { at: *position },
            Self::Undefined {
                position,
                counterexample,
            } => TemporalVerdict::Undefined {
                at: *position,
                cause: undefined_cause(counterexample.undefined_reason()?)?,
            },
            Self::InsufficientData {
                position,
                count,
                min_count,
            } => TemporalVerdict::Undefined {
                at: *position,
                cause: TemporalUndefinedCause::InsufficientData {
                    count: *count,
                    min_count: *min_count,
                },
            },
            Self::Pending {} => TemporalVerdict::Pending,
            Self::PendingWindow { position, deadline } => TemporalVerdict::PendingWindow {
                at: *position,
                deadline: *deadline,
            },
            Self::PendingNextEvent { position } => {
                TemporalVerdict::PendingNextEvent { at: *position }
            }
            Self::Indeterminate { position, reason } => TemporalVerdict::Indeterminate {
                at: *position,
                reason: reason.typed(),
            },
            Self::MissingFairnessPremise { constraint } => {
                TemporalVerdict::MissingFairnessPremise {
                    constraint: constraint.typed(),
                }
            }
            Self::ActivationIncomplete { position, guard } => {
                TemporalVerdict::ActivationIncomplete {
                    at: *position,
                    guard: guard.typed(),
                }
            }
            Self::WindowLimitReached { value, .. } => TemporalVerdict::WindowLimitReached {
                limit: MAX_WINDOW_VALUES,
                value: *value,
                setting: MAX_WINDOW_VALUES_SETTING,
            },
            Self::ProgressLimitReached { value, .. } => {
                TemporalVerdict::ProgressLimitReached(ProgressLimitReached {
                    limit: COVERED_POSITIONS_LIMIT,
                    value: *value,
                    setting: COVERED_POSITIONS_SETTING,
                })
            }
            Self::Incomplete {} | Self::ProgressContradicted { .. } | Self::TraceRefused { .. } => {
                return None
            }
        })
    }

    /// The typed trace refusal a `trace-refused` record restores, equal to
    /// the one written.
    pub fn trace_refusal(&self) -> Option<TemporalTraceRefusal> {
        let Self::TraceRefused {
            variant,
            expected,
            found,
            profile,
            position,
        } = self
        else {
            return None;
        };
        trace_refusal(variant, *expected, *found, profile.as_deref(), *position).ok()
    }

    /// The row's `kind` spelling.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Holds {} => "holds",
            Self::Violated { .. } => "violated",
            Self::Undefined { .. } => "undefined",
            Self::InsufficientData { .. } => "insufficient-data",
            Self::Pending {} => "pending",
            Self::PendingWindow { .. } => "pending-window",
            Self::PendingNextEvent { .. } => "pending-next-event",
            Self::Indeterminate { .. } => "indeterminate",
            Self::MissingFairnessPremise { .. } => "missing-fairness-premise",
            Self::Incomplete {} => "incomplete",
            Self::ActivationIncomplete { .. } => "activation-incomplete",
            Self::WindowLimitReached { .. } => "window-limit-reached",
            Self::ProgressLimitReached { .. } => "progress-limit-reached",
            Self::ProgressContradicted { .. } => "progress-contradicted",
            Self::TraceRefused { .. } => "trace-refused",
        }
    }
}

/// QSpec FR-364's fairness member as read.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
pub struct FairnessRead {
    /// `weak` or `strong`.
    pub kind: FairnessKindRead,
    /// `whole` or `each`.
    pub granularity: FairnessGranularityRead,
    /// The resolved operation.
    pub operation: FairnessOperationRead,
}

impl FairnessRead {
    /// The typed constraint this record restores.
    fn typed(&self) -> quire_semantic_value::fairness::FairnessConstraint {
        use quire_semantic_value::fairness::{
            FairnessConstraint, FairnessGranularity, FairnessKind, FairnessOperation,
        };
        FairnessConstraint::new(
            match self.kind {
                FairnessKindRead::Weak => FairnessKind::Weak,
                FairnessKindRead::Strong => FairnessKind::Strong,
            },
            FairnessOperation::new(
                self.operation.declaring_type.typed(),
                self.operation.name.clone(),
            ),
            match self.granularity {
                FairnessGranularityRead::Whole => FairnessGranularity::Whole,
                FairnessGranularityRead::Each => FairnessGranularity::Each,
            },
        )
    }
}

impl EffectiveIdRead {
    /// The typed identity, built from its root owner down without recursion.
    fn typed(&self) -> quire_exact::EffectiveId {
        let key = |original: &DeclarationKeyRead| {
            quire_exact::OriginalDeclarationKey::new(
                original.package.clone(),
                original.node.clone(),
            )
        };
        let mut members = Vec::new();
        let mut root = self;
        while let Some(owner) = root.owner_effective_type.as_deref() {
            members.push(&root.original);
            root = owner;
        }
        members.iter().rev().fold(
            quire_exact::EffectiveId::new(None, key(&root.original)),
            |owner, original| quire_exact::EffectiveId::new(Some(owner), key(original)),
        )
    }
}

/// A fairness kind as read.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FairnessKindRead {
    /// Weak fairness.
    Weak,
    /// Strong fairness.
    Strong,
}

/// A fairness granularity as read.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FairnessGranularityRead {
    /// Whole-operation fairness.
    Whole,
    /// Per-invocation fairness.
    Each,
}

/// A fairness operation as read.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
pub struct FairnessOperationRead {
    /// The declaring type's FR-201 effective identity.
    pub declaring_type: EffectiveIdRead,
    /// The resolved operation name.
    pub name: String,
}

/// FR-201's recursive effective identity as read.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
pub struct EffectiveIdRead {
    /// The owner's effective type, `null` at the root.
    #[serde(deserialize_with = "nullable")]
    owner_effective_type: Option<Box<EffectiveIdRead>>,
    /// The original declaration key.
    original: DeclarationKeyRead,
}

/// QSpec FR-364's observed-arm counterexample as read.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(
    dead_code,
    reason = "the closed read shape validates every field during deserialization"
)]
pub struct CounterexampleRead {
    trace: ObservedArm,
    kind: CounterexampleKindRead,
    initial_state: (),
    prefix: Vec<StepRead>,
    #[serde(rename = "loop")]
    loop_steps: Vec<StepRead>,
    over_binding: (),
    fairness: Vec<FairnessRead>,
    #[serde(deserialize_with = "nullable")]
    interval: Option<IntervalRead>,
    trace_position: u64,
}

impl CounterexampleRead {
    /// The prefix's document identities, in order.
    pub fn prefix(&self) -> impl Iterator<Item = (&str, &str)> {
        self.prefix.iter().map(|step| {
            (
                step.document.authority.as_str(),
                step.document.identity.as_str(),
            )
        })
    }

    /// The loop's document identities, in order.
    pub fn loop_documents(&self) -> impl Iterator<Item = (&str, &str)> {
        self.loop_steps.iter().map(|step| {
            (
                step.document.authority.as_str(),
                step.document.identity.as_str(),
            )
        })
    }

    /// The failing operator interval's `lower` and `upper` spellings.
    pub fn interval(&self) -> Option<(&str, &str)> {
        self.interval
            .as_ref()
            .map(|interval| (interval.lower.as_str(), interval.upper.as_str()))
    }

    /// The trace position.
    pub fn trace_position(&self) -> u64 {
        self.trace_position
    }

    /// The locus of an undefined evaluation's first undefined expression;
    /// `None` for a formula counterexample.
    pub fn undefined_locus(&self) -> Option<&UndefinedLocusRead> {
        match &self.kind {
            CounterexampleKindRead::Undefined(envelope) => Some(&envelope.point.locus),
            CounterexampleKindRead::Formula(_) => None,
        }
    }

    /// The undefined-evaluation envelope's `cause`; `None` for a formula
    /// counterexample.
    fn undefined_reason(&self) -> Option<&str> {
        match &self.kind {
            CounterexampleKindRead::Undefined(envelope) => Some(&envelope.cause),
            CounterexampleKindRead::Formula(_) => None,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ObservedArm {
    Observed,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum CounterexampleKindRead {
    Formula(FormulaTag),
    Undefined(EnvelopeRead),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum FormulaTag {
    Formula,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EnvelopeRead {
    #[serde(rename = "type")]
    envelope: EnvelopeTag,
    #[serde(rename = "where")]
    point: WhereRead,
    cause: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum EnvelopeTag {
    UndefinedEvaluation,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WhereRead {
    position: u64,
    document: DocumentRefRead,
    locus: UndefinedLocusRead,
}

/// The undefined-evaluation envelope's `locus` as read: exactly one member.
#[derive(Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum UndefinedLocusRead {
    /// A `holds` atom, by the checked clause's formula node index.
    FormulaNode(usize),
    /// The `when` condition of the clause's `on each` activation.
    Guard(GuardRead),
    /// A capture's initializer, by the capture's name.
    Capture(String),
}

/// The `guard` locus value.
#[derive(Debug, Deserialize, Eq, PartialEq)]
pub enum GuardRead {
    /// The clause's one `on each` guard.
    #[serde(rename = "on-each")]
    OnEach,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct StepRead {
    document: DocumentRefRead,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct IntervalRead {
    lower: String,
    upper: String,
}

/// A member that must be present and may be `null`.
fn nullable<'de, D, T>(decoder: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(decoder)
}

/// Read `disposition.temporal`: select the row by its `kind`, then read the
/// row's other members as one closed object, naming any failing member by
/// its path.
pub(super) fn decode(value: &serde_json::Value) -> Result<TemporalRecord, NativeRunReadError> {
    let serde_json::Value::Object(members) = value else {
        return Err(invalid("disposition.temporal", "expected an object"));
    };
    let Some(serde_json::Value::String(kind)) = members.get("kind") else {
        return Err(invalid(
            "disposition.temporal.kind",
            "expected the row's kind",
        ));
    };
    let mut rest = members.clone();
    rest.remove("kind");
    let tagged = serde_json::Value::Object(serde_json::Map::from_iter([(
        kind.clone(),
        serde_json::Value::Object(rest),
    )]));
    serde_path_to_error::deserialize(tagged).map_err(|error| {
        let path = error.path().to_string();
        // The first segment is the row's kind, which selected the variant.
        let member = path
            .split_once('.')
            .map_or_else(String::new, |(_, rest)| format!(".{rest}"));
        NativeRunReadError::MalformedMember {
            member: format!("disposition.temporal{member}"),
            source: error.into_inner(),
        }
    })
}

fn invalid(member: &'static str, reason: &'static str) -> NativeRunReadError {
    NativeRunReadError::Invalid { member, reason }
}

/// QSpec FR-364's checks before replay, and FR-267's `trace_position` rule.
fn validate_counterexample(
    counterexample: &CounterexampleRead,
    position: u64,
    reason: Option<&str>,
) -> Result<(), NativeRunReadError> {
    const MEMBER: &str = "disposition.temporal.counterexample";
    if counterexample.trace_position > MAX_COUNT {
        return Err(invalid(
            "disposition.temporal.counterexample.trace_position",
            "trace position exceeds the Count bound",
        ));
    }
    if counterexample.trace_position != position {
        return Err(invalid(
            "disposition.temporal.counterexample.trace_position",
            "trace position differs from position",
        ));
    }
    if counterexample.prefix.is_empty() && counterexample.loop_steps.is_empty() {
        return Err(invalid(
            "disposition.temporal.counterexample.prefix",
            "an observed counterexample holds at least one document",
        ));
    }
    match (&counterexample.kind, reason) {
        (CounterexampleKindRead::Formula(FormulaTag::Formula), None) => Ok(()),
        (CounterexampleKindRead::Undefined(envelope), Some(reason)) => {
            let EnvelopeTag::UndefinedEvaluation = envelope.envelope;
            let last = u64::try_from(counterexample.prefix.len())
                .ok()
                .and_then(|len| len.checked_sub(1));
            if !counterexample.loop_steps.is_empty() || last != Some(position) {
                return Err(invalid(
                    "disposition.temporal.counterexample.loop",
                    "an undefined evaluation is a finite prefix ending at its position",
                ));
            }
            if counterexample.interval.is_some() {
                return Err(invalid(
                    "disposition.temporal.counterexample.interval",
                    "an undefined evaluation carries no interval",
                ));
            }
            if envelope.point.position != position
                || counterexample
                    .prefix
                    .last()
                    .is_none_or(|step| step.document.typed() != envelope.point.document.typed())
            {
                return Err(invalid(
                    "disposition.temporal.counterexample.kind.where",
                    "the evaluation point is not the prefix's last position",
                ));
            }
            if envelope.cause != reason {
                return Err(invalid(
                    "disposition.temporal.counterexample.kind.cause",
                    "the envelope cause differs from the outcome reason",
                ));
            }
            Ok(())
        }
        _ => Err(invalid(
            MEMBER,
            "counterexample kind disagrees with the row",
        )),
    }
}

/// The trace refusal a `trace-refused` record names, checking that exactly
/// the variant's payload members are present.
fn trace_refusal(
    variant: &str,
    expected: Option<usize>,
    found: Option<usize>,
    profile: Option<&str>,
    position: Option<u64>,
) -> Result<TemporalTraceRefusal, NativeRunReadError> {
    let member = "disposition.temporal";
    let none = |refusal| {
        if expected.is_some() || found.is_some() || profile.is_some() || position.is_some() {
            Err(invalid(member, "trace refusal variant carries no payload"))
        } else {
            Ok(refusal)
        }
    };
    let positioned =
        |refusal: fn(u64) -> TemporalTraceRefusal| match (expected, found, profile, position) {
            (None, None, None, Some(position)) => Ok(refusal(position)),
            _ => Err(invalid(
                "disposition.temporal.position",
                "trace refusal variant needs exactly its position",
            )),
        };
    let profiled =
        |refusal: fn(qsl_semantics::check::TemporalProfile) -> TemporalTraceRefusal| match (
            expected, found, profile, position,
        ) {
            (None, None, Some(identity), None) => {
                qsl_semantics::check::TemporalProfile::from_identity(identity)
                    .map(refusal)
                    .ok_or_else(|| {
                        invalid(
                            "disposition.temporal.profile",
                            "unknown temporal profile identity",
                        )
                    })
            }
            _ => Err(invalid(
                "disposition.temporal.profile",
                "trace refusal variant needs exactly its profile",
            )),
        };
    match variant {
        "empty-trace" => none(TemporalTraceRefusal::EmptyTrace),
        "tuple-arity" => match (expected, found, profile, position) {
            (Some(expected), Some(found), None, None) => {
                Ok(TemporalTraceRefusal::TupleArity { expected, found })
            }
            _ => Err(invalid(
                "disposition.temporal.expected",
                "tuple-arity needs exactly expected and found",
            )),
        },
        "align-skip-not-lockstep" => none(TemporalTraceRefusal::AlignSkipNotLockstep),
        "not-projection-aligned" => none(TemporalTraceRefusal::NotProjectionAligned),
        "empty-loop" => none(TemporalTraceRefusal::EmptyLoop),
        "trace-too-long" => none(TemporalTraceRefusal::TraceTooLong),
        "lasso-under-bounded-profile" => {
            profiled(|profile| TemporalTraceRefusal::LassoUnderBoundedProfile { profile })
        }
        "profile-not-evaluated" => {
            profiled(|profile| TemporalTraceRefusal::ProfileNotEvaluated { profile })
        }
        "position-not-decimal" => none(TemporalTraceRefusal::Position(
            TemporalPositionRefusal::NotDecimal,
        )),
        "position-leading-zero" => none(TemporalTraceRefusal::Position(
            TemporalPositionRefusal::LeadingZero,
        )),
        "position-overflow" => none(TemporalTraceRefusal::Position(
            TemporalPositionRefusal::Overflow,
        )),
        "position-outside-trace" => positioned(|position| {
            TemporalTraceRefusal::Position(TemporalPositionRefusal::OutsideTrace { position })
        }),
        "window-outside-trace" => positioned(|position| {
            TemporalTraceRefusal::Window(TemporalWindowRefusal::OutsideTrace { position })
        }),
        "window-kind-mismatch" => none(TemporalTraceRefusal::Window(
            TemporalWindowRefusal::WindowKindMismatch,
        )),
        "timestamps-decrease" => none(TemporalTraceRefusal::Window(
            TemporalWindowRefusal::TimestampsDecrease,
        )),
        "same-stamp-order-unavailable" => {
            positioned(|position| TemporalTraceRefusal::SameStampOrderUnavailable { position })
        }
        _ => Err(invalid(
            "disposition.temporal.variant",
            "unknown trace refusal variant",
        )),
    }
}

/// What a row fixes beside its members.
struct Row {
    category: Category,
    truth: Option<bool>,
    basis: Basis,
}

/// Check a disposition carrying `temporal` against its FR-267 row, and the
/// document's basis and witness presence.
pub(super) fn validate_temporal(
    value: &DispositionRead,
    temporal: &TemporalRecord,
    basis: &Basis,
    has_witness: bool,
) -> Result<(), NativeRunReadError> {
    if !matches!(value.stage, Stage::Evaluate) {
        return Err(invalid(
            "disposition.temporal",
            "a temporal disposition is written at stage evaluate only",
        ));
    }
    if has_witness {
        return Err(invalid(
            "witness",
            "a temporal disposition carries no separating witness",
        ));
    }
    if value.refusal.is_some() {
        return Err(invalid(
            "disposition.refusal",
            "a temporal disposition carries no refusal",
        ));
    }
    if value.internal_fault.is_some() {
        return Err(invalid(
            "disposition.internal_fault",
            "a temporal disposition carries no internal fault",
        ));
    }
    let outcome = value.outcome.as_ref();
    let no_outcome = || match outcome {
        None => Ok(()),
        Some(_) => Err(invalid(
            "disposition.outcome",
            "this temporal row carries no outcome",
        )),
    };
    let undefined = |expected: Option<&str>| match outcome {
        Some(OutcomeRead::Undefined { reason })
            if expected.map_or(reason != INSUFFICIENT_DATA, |expected| reason == expected) =>
        {
            Ok(reason.as_str())
        }
        _ => Err(invalid(
            "disposition.outcome",
            "outcome disagrees with the temporal row",
        )),
    };
    let incomplete = |window: bool| match outcome {
        Some(OutcomeRead::Incomplete { limit }) if (limit == MAX_WINDOW_VALUES) == window => Ok(()),
        _ => Err(invalid(
            "disposition.outcome",
            "outcome disagrees with the temporal row",
        )),
    };
    let refused = |code: &str, cause: &str, fields: Option<&BTreeMap<String, String>>| match outcome
    {
        Some(OutcomeRead::Refused {
            code: read_code,
            cause: read_cause,
            fields: read_fields,
            locus: None,
            location: None,
        }) if read_code == code && read_cause == cause && read_fields.as_ref() == fields => Ok(()),
        _ => Err(invalid(
            "disposition.outcome",
            "outcome disagrees with the temporal row",
        )),
    };
    let unsettled = Row {
        category: Category::Pending,
        truth: None,
        basis: Basis::Unsettled,
    };
    let unavailable = |category| Row {
        category,
        truth: None,
        basis: Basis::Unavailable,
    };
    let row = match temporal {
        TemporalRecord::Holds {} => {
            no_outcome()?;
            Row {
                category: Category::Success,
                truth: Some(true),
                basis: Basis::ClosedScope,
            }
        }
        TemporalRecord::Violated {
            position,
            counterexample,
        } => {
            no_outcome()?;
            validate_counterexample(counterexample, *position, None)?;
            Row {
                category: Category::Violation,
                truth: Some(false),
                basis: Basis::ClosedScope,
            }
        }
        TemporalRecord::Undefined {
            position,
            counterexample,
        } => {
            let reason = undefined(None)?;
            if undefined_cause(reason).is_none() {
                return Err(invalid(
                    "disposition.outcome.reason",
                    "unknown undefined reason",
                ));
            }
            validate_counterexample(counterexample, *position, Some(reason))?;
            Row {
                category: Category::Violation,
                truth: Some(false),
                basis: Basis::ClosedScope,
            }
        }
        TemporalRecord::InsufficientData { .. } => {
            undefined(Some(INSUFFICIENT_DATA))?;
            unavailable(Category::Undefined)
        }
        TemporalRecord::Pending {}
        | TemporalRecord::PendingWindow { .. }
        | TemporalRecord::PendingNextEvent { .. } => {
            no_outcome()?;
            unsettled
        }
        TemporalRecord::Indeterminate { reason, .. } => {
            no_outcome()?;
            if let IndeterminateReasonRead::MissingValuation {
                instance,
                also_held_by,
                ..
            } = reason
            {
                if instance.is_some() != also_held_by.is_some() {
                    return Err(invalid(
                        "disposition.temporal.reason.also_held_by",
                        "instance and also_held_by are present together",
                    ));
                }
            }
            unavailable(Category::Indeterminate)
        }
        TemporalRecord::MissingFairnessPremise { .. } => {
            refused("unsupported_projection", "missing-fairness-premise", None)?;
            unavailable(Category::Unsupported)
        }
        TemporalRecord::Incomplete {} => {
            incomplete(false)?;
            unavailable(Category::Incomplete)
        }
        TemporalRecord::ActivationIncomplete { .. } => {
            no_outcome()?;
            unavailable(Category::Incomplete)
        }
        TemporalRecord::ProgressLimitReached { limit, setting, .. } => {
            if limit != COVERED_POSITIONS_LIMIT || setting != COVERED_POSITIONS_SETTING {
                return Err(invalid(
                    "disposition.temporal.limit",
                    "progress limit names node count and progress.covered_positions",
                ));
            }
            no_outcome()?;
            unavailable(Category::Incomplete)
        }
        TemporalRecord::WindowLimitReached { limit, setting, .. } => {
            if limit != MAX_WINDOW_VALUES || setting != MAX_WINDOW_VALUES_SETTING {
                return Err(invalid(
                    "disposition.temporal.limit",
                    "window limit names max_window_values and its setting",
                ));
            }
            incomplete(true)?;
            unavailable(Category::Incomplete)
        }
        TemporalRecord::ProgressContradicted { .. } => {
            let contradiction = temporal
                .contradiction()
                .ok_or_else(|| invalid("disposition.temporal", "contradiction restores"))?;
            let record = ProgressRefusal::from(contradiction).record;
            let fields: BTreeMap<String, String> = record
                .fields
                .iter()
                .map(|(name, value)| ((*name).to_owned(), value.clone()))
                .collect();
            refused(record.catalog.code(), record.catalog.cause(), Some(&fields))?;
            unavailable(Category::Refusal)
        }
        TemporalRecord::TraceRefused {
            variant,
            expected,
            found,
            profile,
            position,
        } => {
            let refusal = trace_refusal(variant, *expected, *found, profile.as_deref(), *position)?;
            let code = refusal.catalog_code();
            refused(code.code(), code.cause(), None)?;
            let category = match qsl_foundation::diagnostic::Code::from_code(code.code())
                .map(qsl_foundation::diagnostic::Code::category)
            {
                Some(quire_semantic_value::catalog::Category::Unsupported) => Category::Unsupported,
                _ => Category::Refusal,
            };
            unavailable(category)
        }
    };
    if value.category != row.category {
        return Err(invalid(
            "disposition.category",
            "category disagrees with the temporal row",
        ));
    }
    if value.truth != row.truth {
        return Err(invalid(
            "disposition.truth",
            "truth disagrees with the temporal row",
        ));
    }
    if *basis != row.basis {
        return Err(invalid("basis", "basis disagrees with the temporal row"));
    }
    Ok(())
}

/// FR-267-AC-16: the undefined reason `insufficient-data` and the incomplete
/// limit `max_window_values` belong to their temporal rows only.
pub(super) fn refuse_temporal_only_outcome(
    outcome: Option<&OutcomeRead>,
) -> Result<(), NativeRunReadError> {
    match outcome {
        Some(OutcomeRead::Undefined { reason }) if reason == INSUFFICIENT_DATA => Err(invalid(
            "disposition.outcome.reason",
            "insufficient-data is read in the insufficient-data row only",
        )),
        Some(OutcomeRead::Incomplete { limit }) if limit == MAX_WINDOW_VALUES => Err(invalid(
            "disposition.outcome.limit",
            "max_window_values is read in the window-limit-reached row only",
        )),
        _ => Ok(()),
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-491: the facade's state-clause evaluation entry. [`evaluate_clause`]
//! evaluates one authored state clause at one observation and returns a typed
//! [`ClauseEvaluation`] (`Completed`, `Undefined`, `Refused` or `Incomplete`).
//!
//! It is built on FR-109's `spine::run_clause` (the `Clause` selection) and
//! reuses its admission (FR-106) and evaluation (FR-107) unchanged. A
//! consumer reaches this module's items through the `qsl_replay` root and
//! never names [`crate::spine`] (ADR-011 FB-05, T-12 rule (a)).
//!
//! The observation is of the clause's own kind (FR-106 check 2): an invariant
//! is evaluated at a [`ClauseObservation::Current`] snapshot, a precondition
//! at a [`ClauseObservation::PreCall`] one. A mismatch is a refusal, never a
//! guess. The request supplies the snapshot document's bytes and the result
//! carries no identity of any document, package or source.

use std::collections::BTreeMap;

use qsl_foundation::{Code, SourceIdentity};
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::observation::{
    ClauseSelection, ClauseSelectionInput, ObservationLimits, SelectedAnchor, SelectedObject,
    SnapshotValue,
};
use quire_exact::{Identifier, ScalarLimits};
use quire_semantic_value::catalog::Category;

use crate::spine::{
    run_clause, CallIncomplete, CallOutcome, CallValue, ClauseDisposition, ClauseRunRefusal,
    ClauseRunRequest, ClauseRunSelection, ClauseRunSource, ClauseRunStage, DependencyInput,
    SpineLimits, SuppliedObservation,
};
use qsl_semantics::model::observation::AdmissionFailure;

/// The observation a clause is evaluated at, of the clause's own kind. Each
/// variant carries the exact bytes of one `quire.state.snapshot/v1` document.
#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum ClauseObservation {
    /// An invariant's or constraint's current snapshot.
    Current {
        /// The snapshot document's bytes.
        snapshot: Vec<u8>,
        /// The selected anchor.
        anchor: SelectedAnchor,
        /// The self object; `None` for a state machine's context.
        self_object: Option<SelectedObject>,
    },
    /// A precondition's pre-call observation: the operation's pre state, the
    /// self object and the parameter values, before the operation runs.
    PreCall {
        /// The pre snapshot document's bytes.
        snapshot: Vec<u8>,
        /// The self object; `None` for a state machine's context.
        self_object: Option<SelectedObject>,
        /// Each declared parameter's value, by parameter name.
        parameters: BTreeMap<Identifier, SnapshotValue>,
    },
}

/// One clause evaluation: the unit, the clause and the observation.
#[derive(Debug)]
pub struct ClauseEvaluationRequest {
    /// The unit's four FR-001 labels.
    pub source: SourceIdentity,
    /// The unit's display path; only displayed, never opened.
    pub path: String,
    /// The unit's source bytes.
    pub bytes: Vec<u8>,
    /// The domain package documents the unit selects, as supplied.
    pub packages: Vec<Vec<u8>>,
    /// The unit's dependency input.
    pub dependencies: DependencyInput,
    /// The authored state clause's declared name.
    pub clause: String,
    /// The observation to evaluate at.
    pub observation: ClauseObservation,
    /// The spine's stage limits.
    pub limits: SpineLimits,
    /// FR-106's observation limits; `document_bytes` also bounds the
    /// snapshot bytes read before admission.
    pub observation_limits: ObservationLimits,
    /// The evaluation meter's accounting limits.
    pub accounting: ScalarLimits,
}

/// Why no value was computed (the `Refused` result).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ClauseRefusal {
    /// The unit's source is empty.
    EmptySource,
    /// The observation bytes are not a readable `quire.state.snapshot/v1`
    /// document.
    UnreadableObservation,
    /// The observation is of the wrong kind for the clause (`wrong_snapshot`,
    /// `wrong-observation`): an invariant read at a pre-call observation or a
    /// precondition at a current one.
    WrongObservation,
    /// The unit names no state clause `name`.
    MissingClause {
        /// The requested clause name.
        name: String,
    },
    /// A refusal with a catalog code and cause: a compile refusal, an
    /// admission refusal, or an evaluation refusal.
    Coded {
        /// The stage that refused.
        stage: ClauseRunStage,
        /// The ADR-013 O-16 category.
        category: Category,
        /// The catalog code.
        code: &'static str,
        /// The catalog cause, empty when the stage names none.
        cause: &'static str,
    },
    /// A broken invariant of QSL itself.
    Fault {
        /// The stage that raised it.
        stage: &'static str,
        /// The invariant that broke.
        invariant: &'static str,
    },
}

/// Why the evaluation could not finish (the `Incomplete` result).
#[derive(Debug)]
#[non_exhaustive]
pub enum ClauseIncomplete {
    /// An accounting limit was exhausted during evaluation; the value names
    /// the counter that raises it.
    Accounting(CallIncomplete),
    /// The evaluation's inputs stopped at a resource bound or a cancel
    /// before a value was computed.
    Stopped {
        /// The catalog code.
        code: &'static str,
    },
    /// The observation lacks evidence the clause reads (an incomplete
    /// population or an unresolved closure), at admission, or its snapshot
    /// exceeds `observation_limits.document_bytes` (`resource_exhausted`,
    /// `observation-input-bytes`) and is not read.
    Observation {
        /// The catalog code.
        code: &'static str,
        /// The catalog cause.
        cause: &'static str,
    },
}

/// The typed result of [`evaluate_clause`].
#[derive(Debug)]
#[non_exhaustive]
pub enum ClauseEvaluation {
    /// The clause evaluated: `true` when it holds at the observation,
    /// `false` when it does not.
    Completed(bool),
    /// The clause is undefined at the observation, for the reason's tabled
    /// spelling.
    Undefined {
        /// The reason's tabled spelling.
        reason: &'static str,
    },
    /// The request was refused before a value was computed.
    Refused(ClauseRefusal),
    /// The evaluation could not finish.
    Incomplete(ClauseIncomplete),
}

/// FR-491: evaluate `request.clause` at `request.observation`.
pub fn evaluate_clause(request: ClauseEvaluationRequest) -> ClauseEvaluation {
    let ClauseEvaluationRequest {
        source,
        path,
        bytes,
        packages,
        dependencies,
        clause,
        observation,
        limits,
        observation_limits,
        accounting,
    } = request;
    if bytes.is_empty() {
        return ClauseEvaluation::Refused(ClauseRefusal::EmptySource);
    }
    let (snapshot_bytes, input) = match observation {
        ClauseObservation::Current {
            snapshot,
            anchor,
            self_object,
        } => (
            snapshot,
            Select::Current {
                anchor,
                self_object,
            },
        ),
        ClauseObservation::PreCall {
            snapshot,
            self_object,
            parameters,
        } => (
            snapshot,
            Select::PreCall {
                self_object,
                parameters,
            },
        ),
    };
    if u64::try_from(snapshot_bytes.len())
        .map_or(true, |len| len > observation_limits.document_bytes)
    {
        return ClauseEvaluation::Incomplete(ClauseIncomplete::Observation {
            code: Code::ResourceExhausted.as_str(),
            cause: "observation-input-bytes",
        });
    }
    let supplied = match SuppliedObservation::from_document("snapshot", snapshot_bytes) {
        Ok(supplied) if supplied.format == "quire.state.snapshot/v1" => supplied,
        _ => return ClauseEvaluation::Refused(ClauseRefusal::UnreadableObservation),
    };
    let reference = supplied.reference.clone();
    let input = match input {
        Select::Current {
            anchor,
            self_object,
        } => ClauseSelectionInput::Current {
            snapshot: reference.clone(),
            anchor,
            self_object,
        },
        Select::PreCall {
            self_object,
            parameters,
        } => ClauseSelectionInput::PreCall {
            snapshot: reference.clone(),
            self_object,
            parameters,
        },
    };
    let packages = match package_input(packages.iter().map(Vec::as_slice)) {
        Ok(packages) => packages,
        Err(refusal) => {
            return coded(
                ClauseRunStage::Compile,
                refusal.code.category(),
                refusal.code.as_str(),
                "",
            );
        }
    };
    let run = ClauseRunRequest {
        source: ClauseRunSource::Program {
            identity: source,
            path,
            bytes,
        },
        packages,
        dependencies,
        snapshots: BTreeMap::from([(reference, supplied.bytes)]),
        invocations: BTreeMap::new(),
        selection: ClauseRunSelection::Clause(ClauseSelection {
            name: clause,
            input,
        }),
        expected_package_id: None,
        expected_package_id_claim: None,
        limits,
        observation_limits,
        accounting,
    };
    match run_clause(run) {
        Ok(report) => settle(report.disposition),
        Err(ClauseRunRefusal::EmptySource) => ClauseEvaluation::Refused(ClauseRefusal::EmptySource),
    }
}

/// The observation half, before its snapshot is read.
enum Select {
    Current {
        anchor: SelectedAnchor,
        self_object: Option<SelectedObject>,
    },
    PreCall {
        self_object: Option<SelectedObject>,
        parameters: BTreeMap<Identifier, SnapshotValue>,
    },
}

fn coded(
    stage: ClauseRunStage,
    category: Category,
    code: &'static str,
    cause: &'static str,
) -> ClauseEvaluation {
    ClauseEvaluation::Refused(ClauseRefusal::Coded {
        stage,
        category,
        code,
        cause,
    })
}

/// A disposition a `Clause` selection over a program source never reaches.
fn unreachable_disposition() -> ClauseEvaluation {
    ClauseEvaluation::Refused(ClauseRefusal::Fault {
        stage: "clause-evaluation",
        invariant: "clause-selection-disposition",
    })
}

fn settle(disposition: ClauseDisposition) -> ClauseEvaluation {
    let stage = disposition.stage();
    let category = disposition.category();
    match disposition {
        ClauseDisposition::Evaluate(CallOutcome::Completed(CallValue::Boolean(value))) => {
            ClauseEvaluation::Completed(value)
        }
        ClauseDisposition::Evaluate(CallOutcome::Undefined { reason }) => {
            ClauseEvaluation::Undefined { reason }
        }
        ClauseDisposition::Evaluate(CallOutcome::Incomplete(incomplete)) => {
            ClauseEvaluation::Incomplete(ClauseIncomplete::Accounting(incomplete))
        }
        ClauseDisposition::Evaluate(CallOutcome::Refused(refusal)) => {
            let (crate::spine::CallRefusal::Record { code, .. }
            | crate::spine::CallRefusal::Family { code, .. }) = &refusal;
            coded(stage, category, code.code(), code.cause())
        }
        ClauseDisposition::Evaluate(CallOutcome::Completed(_)) => unreachable_disposition(),
        ClauseDisposition::Compile(refusal) => coded(
            stage,
            category,
            refusal.code().as_str(),
            refusal.cause().unwrap_or(""),
        ),
        ClauseDisposition::MissingName { name } => {
            ClauseEvaluation::Refused(ClauseRefusal::MissingClause { name })
        }
        ClauseDisposition::Admit(AdmissionFailure::Refused(record)) => {
            if record.code == Code::WrongSnapshot && record.cause == "wrong-observation" {
                ClauseEvaluation::Refused(ClauseRefusal::WrongObservation)
            } else {
                coded(stage, category, record.code.as_str(), record.cause)
            }
        }
        ClauseDisposition::Admit(AdmissionFailure::Incomplete(record)) => {
            ClauseEvaluation::Incomplete(ClauseIncomplete::Observation {
                code: record.code.as_str(),
                cause: record.cause,
            })
        }
        ClauseDisposition::Admit(AdmissionFailure::Fault(fault))
        | ClauseDisposition::EvaluateFault(fault) => {
            ClauseEvaluation::Refused(ClauseRefusal::Fault {
                stage: fault.stage(),
                invariant: fault.invariant(),
            })
        }
        // A refusal of the evaluation's own inputs (a dangling reference, a
        // value of the wrong kind, a supplied-value or storage stop): its
        // catalog code, as `Coded`, or `Incomplete` when it is a resource
        // stop (category incomplete).
        ClauseDisposition::ArgumentRefusal(refusal) => {
            let code = refusal.code().as_str();
            if category == Category::Incomplete {
                ClauseEvaluation::Incomplete(ClauseIncomplete::Stopped { code })
            } else {
                coded(stage, category, code, "")
            }
        }
        ClauseDisposition::UnknownLanguage { .. }
        | ClauseDisposition::StalePackage { .. }
        | ClauseDisposition::NotAPredicate { .. }
        | ClauseDisposition::FrameViolation(_)
        | ClauseDisposition::Temporal(_)
        | ClauseDisposition::TemporalRefusal(_) => unreachable_disposition(),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// FR-491-AC-4: an S6a `Undefined` outcome of the clause is the typed
    /// `Undefined` result, carrying the reason's tabled spelling, and is
    /// neither a value nor a refusal.
    #[trace("TC-1274", "FR-491-AC-4")]
    #[test]
    fn an_undefined_evaluation_settles_as_undefined() {
        let undefined = settle(ClauseDisposition::Evaluate(CallOutcome::Undefined {
            reason: "division-by-zero",
        }));
        assert!(
            matches!(
                undefined,
                ClauseEvaluation::Undefined {
                    reason: "division-by-zero"
                }
            ),
            "{undefined:?}"
        );
        let holds = settle(ClauseDisposition::Evaluate(CallOutcome::Completed(
            CallValue::Boolean(true),
        )));
        assert!(
            matches!(holds, ClauseEvaluation::Completed(true)),
            "{holds:?}"
        );
    }

    /// FR-491-AC-7: a refusal of the evaluation's inputs settles by its own
    /// cause, not as a fault.
    #[trace("TC-1274", "FR-491-AC-7")]
    #[test]
    fn an_input_refusal_settles_by_its_cause() {
        use crate::spine::RunRefusal;
        let dangling = settle(ClauseDisposition::ArgumentRefusal(Box::new(
            RunRefusal::DanglingReference { position: 0 },
        )));
        assert!(
            matches!(
                dangling,
                ClauseEvaluation::Refused(ClauseRefusal::Coded {
                    stage: ClauseRunStage::Admit,
                    code: "dangling_reference",
                    ..
                })
            ),
            "{dangling:?}"
        );
        let wrong_kind = settle(ClauseDisposition::ArgumentRefusal(Box::new(
            RunRefusal::WrongValueKind { position: 1 },
        )));
        assert!(
            matches!(
                wrong_kind,
                ClauseEvaluation::Refused(ClauseRefusal::Coded {
                    code: "invalid_runtime_input",
                    ..
                })
            ),
            "{wrong_kind:?}"
        );
        let stopped = settle(ClauseDisposition::ArgumentRefusal(Box::new(
            RunRefusal::Cancelled(quire_exact::CancelCause::Requested),
        )));
        assert!(
            matches!(
                stopped,
                ClauseEvaluation::Incomplete(ClauseIncomplete::Stopped { code: "cancelled" })
            ),
            "{stopped:?}"
        );
    }
}

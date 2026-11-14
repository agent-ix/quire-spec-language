// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-20, FR-573: the settlement map of a protocol analysis. Both
//! kinds settle with the QSpec FR-360 labels through the FR-241, FR-242 and
//! FR-243 combination FR-360 names, by the native-engine rule (ADR-011 S6c,
//! ADR-018 SM-7, PC-1). A `refuted` outcome reaches this map only after its
//! counterexample replayed: each engine settles a refutation whose replay
//! disagrees `inconclusive` before returning it.

use super::{AnalysisOutcome, ProtocolAnalysisKey, ProtocolAnalysisResult};
use crate::model_check::settle::{Execution, ResultLabel, Truth};
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
    UnavailabilityCause,
};

/// One settled protocol analysis: the FR-331 terminal value and the FR-360
/// label with its execution, truth and basis axes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolAnalysisSettlement {
    /// The FR-331 terminal value; a `proved` one is `Uncertified`.
    pub value: TerminalValue,
    /// The FR-360 label.
    pub label: ResultLabel,
    /// The FR-243 basis.
    pub basis: SettlementBasis,
    /// The FR-241 execution.
    pub execution: Execution,
    /// The FR-242 truth.
    pub truth: Truth,
}

impl ProtocolAnalysisSettlement {
    /// The settlement of `result` (FR-573's map).
    #[must_use]
    pub fn of<A, C>(result: &ProtocolAnalysisResult<A, C>) -> Self {
        match &result.outcome {
            AnalysisOutcome::Proved(_) => Self {
                // No core checker exists for either engine (ADR-018 PC-1).
                value: TerminalValue::Proved {
                    basis: ProofBasis::Exhaustive,
                    certification: Certification::Uncertified,
                },
                label: ResultLabel::Proved,
                basis: SettlementBasis::ClosedScope,
                execution: Execution::Completed,
                truth: Truth::Satisfied,
            },
            AnalysisOutcome::Refuted(_) => Self {
                value: TerminalValue::Refuted,
                label: ResultLabel::Refuted,
                basis: SettlementBasis::DecisiveCounterexample,
                execution: Execution::Completed,
                truth: Truth::Violated,
            },
            AnalysisOutcome::Inconclusive(cause) => Self {
                value: TerminalValue::Inconclusive(match cause {
                    super::InconclusiveCause::Horizon => InconclusiveCause::BoundReached {
                        depth: max_depth(&result.key),
                    },
                    other => InconclusiveCause::ProtocolAnalysis(other.clone()),
                }),
                label: ResultLabel::Inconclusive,
                basis: SettlementBasis::Unsettled,
                execution: Execution::Completed,
                truth: Truth::Pending,
            },
            AnalysisOutcome::Unsupported(cause) => Self::unsupported(cause.clone()),
            AnalysisOutcome::Failed(limit) => Self::resource_incomplete(IncompleteCause::LimitReached {
                limit: limit.limit,
                value: limit.value,
            }),
        }
    }

    /// `unsupported`: a construct outside the `bounded-static` profile
    /// (FR-575).
    #[must_use]
    pub fn unsupported(cause: super::UnsupportedCause) -> Self {
        Self {
            value: TerminalValue::Unsupported(UnavailabilityCause::ProtocolAnalysis(cause)),
            label: ResultLabel::Unsupported,
            basis: SettlementBasis::Unavailable,
            execution: Execution::Unsupported,
            truth: Truth::Unavailable,
        }
    }

    /// `failed`, `resource-incomplete`: a run limit, the evaluation meter or
    /// a cancellation stopped the analysis before its method completed.
    #[must_use]
    pub fn resource_incomplete(cause: IncompleteCause) -> Self {
        Self {
            value: TerminalValue::Incomplete(cause),
            label: ResultLabel::Failed,
            basis: SettlementBasis::Unavailable,
            execution: Execution::ResourceIncomplete,
            truth: Truth::Unavailable,
        }
    }

    /// `failed`, execution `failed`: the engine itself failed.
    #[must_use]
    pub fn failed() -> Self {
        Self {
            value: TerminalValue::Failed,
            label: ResultLabel::Failed,
            basis: SettlementBasis::Unavailable,
            execution: Execution::Failed,
            truth: Truth::Unavailable,
        }
    }
}

fn max_depth(key: &ProtocolAnalysisKey) -> u64 {
    match key {
        ProtocolAnalysisKey::Projection(key) => key.bounds.max_depth,
        ProtocolAnalysisKey::Realizability(key) => key.bounds.max_depth,
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-5's product-build refusal settlement (FR-203).

use qsl_eval::simulation::model::ClauseOutcome;
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop};
use qsl_foundation::{BackendId, RequestIndex};
use quire_exact::{CancelCause, Rational};
use quire_semantic_value::catalog::CatalogCode;

use crate::model_check::limits::{meter_bound, ModelCheckLimit};
use crate::proof_result::{
    IncompleteCause, InconclusiveCause, TerminalRecord, TerminalValue, UnavailabilityCause,
};
use crate::statistical::SubjectLimits;

/// The build result's settlement-relevant cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactProbBuildIssue {
    /// An exact-product budget stopped exploration.
    Limit {
        /// The request limit reached.
        limit: ModelCheckLimit,
        /// The configured value.
        value: u64,
    },
    /// The request was cancelled.
    Cancelled(CancelCause),
    /// Dyadic refinement reached its maximum precision with this interval
    /// still containing the threshold.
    PrecisionBudget {
        /// Lower endpoint.
        lower: Rational,
        /// Upper endpoint.
        upper: Rational,
    },
    /// The workload does not define a Markov chain.
    NotMarkov,
    /// A product successor's contract was undecided.
    UndecidedSuccessor,
    /// State expansion failed or reached a subject-owned limit.
    Expansion(ExpansionStop),
    /// A claim predicate was refused or incomplete.
    Predicate(ClauseOutcome),
    /// An internal product invariant, automaton translation, or evaluator
    /// invariant failed.
    BrokenInvariant,
    /// A refusal carrying its own catalog diagnostic.
    Failed(CatalogCode),
}

/// One settled EN-5 build issue. It is always non-refuted: a counterexample
/// can settle only after FR-202 replays its evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbBuildRecord {
    value: TerminalValue,
    diagnostic: Option<CatalogCode>,
}

impl ExactProbBuildRecord {
    /// The FR-331 result kind this stop settles.
    pub fn value(&self) -> &TerminalValue {
        &self.value
    }

    /// Its one diagnostic, when this is a failed result.
    pub fn diagnostic(&self) -> Option<CatalogCode> {
        self.diagnostic
    }

    /// Build this non-refuted EN-5 result's terminal record.
    pub fn into_terminal(self, request_index: RequestIndex, backend: BackendId) -> TerminalRecord {
        match (self.value, self.diagnostic) {
            (TerminalValue::Incomplete(cause), None) => {
                TerminalRecord::incomplete(request_index, backend, cause)
            }
            (TerminalValue::Inconclusive(cause), None) => {
                TerminalRecord::inconclusive(request_index, backend, cause)
            }
            (TerminalValue::Unsupported(cause), None) => {
                TerminalRecord::unsupported(request_index, backend, cause)
            }
            (TerminalValue::Failed, Some(diagnostic)) => {
                TerminalRecord::failed_with(request_index, backend, diagnostic)
            }
            _ => unreachable!("an exact build issue has a non-refuted terminal value"),
        }
    }
}

/// FR-203's exhaustive map for FR-196's build stops and predicate refusals.
pub fn settle_exact_prob_build_issue(
    issue: &ExactProbBuildIssue,
    subject: SubjectLimits<'_>,
) -> ExactProbBuildRecord {
    let incomplete = |limit, value| ExactProbBuildRecord {
        value: TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value }),
        diagnostic: None,
    };
    let failed = |diagnostic| ExactProbBuildRecord {
        value: TerminalValue::Failed,
        diagnostic: Some(diagnostic),
    };
    match issue {
        ExactProbBuildIssue::Limit { limit, value } => incomplete(*limit, *value),
        ExactProbBuildIssue::Cancelled(source) => ExactProbBuildRecord {
            value: TerminalValue::Incomplete(IncompleteCause::Cancelled { source: *source }),
            diagnostic: None,
        },
        ExactProbBuildIssue::PrecisionBudget { lower, upper } => ExactProbBuildRecord {
            value: TerminalValue::Incomplete(IncompleteCause::PrecisionBudget {
                lower: Box::new(crate::proof_result::ExactRational::new(lower.clone())),
                upper: Box::new(crate::proof_result::ExactRational::new(upper.clone())),
            }),
            diagnostic: None,
        },
        ExactProbBuildIssue::NotMarkov => ExactProbBuildRecord {
            value: TerminalValue::Unsupported(UnavailabilityCause::NotMarkov),
            diagnostic: None,
        },
        ExactProbBuildIssue::UndecidedSuccessor => ExactProbBuildRecord {
            value: TerminalValue::Inconclusive(InconclusiveCause::UndecidedSuccessor),
            diagnostic: None,
        },
        ExactProbBuildIssue::Expansion(stop) => match stop.limit {
            Some(ExpansionLimit::EvaluationMeter(counter)) => incomplete(
                ModelCheckLimit::EvaluationMeter { counter },
                meter_bound(subject.meter, counter),
            ),
            Some(ExpansionLimit::MaxCandidates) => incomplete(
                ModelCheckLimit::MaxCandidates,
                subject.expansion.max_candidates,
            ),
            None if stop.is_internal_failure() => failed(BROKEN),
            None => failed(stop.cause),
        },
        ExactProbBuildIssue::Predicate(outcome) => predicate(outcome, incomplete, failed),
        ExactProbBuildIssue::BrokenInvariant => failed(BROKEN),
        ExactProbBuildIssue::Failed(diagnostic) => failed(*diagnostic),
    }
}

const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");

fn predicate(
    outcome: &ClauseOutcome,
    incomplete: impl Fn(ModelCheckLimit, u64) -> ExactProbBuildRecord,
    failed: impl Fn(CatalogCode) -> ExactProbBuildRecord,
) -> ExactProbBuildRecord {
    match outcome {
        ClauseOutcome::KernelRefused(refusal) => match (refusal.code(), refusal.cause()) {
            (Some(code), Some(cause)) => failed(CatalogCode::new(code, cause)),
            _ => failed(BROKEN),
        },
        ClauseOutcome::FamilyRefused(code) => failed(*code),
        ClauseOutcome::Incomplete(value) => incomplete(
            ModelCheckLimit::EvaluationMeter {
                counter: value.limit_kind,
            },
            value.limit,
        ),
        ClauseOutcome::KernelUndefined(_)
        | ClauseOutcome::FamilyUndefined(_)
        | ClauseOutcome::NotBoolean => failed(BROKEN),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_eval::simulation::model::ExpansionLimits;
    use quire_exact::{Integer, IntegerInterval, ScalarLimits};

    #[trace("TC-638", "FR-203-AC-6", "FR-203-AC-7")]
    #[test]
    fn build_stops_and_refusals_have_the_fr203_terminal_map() {
        let meter = ScalarLimits {
            integer_bits: 64,
            decimal_digits: 64,
            scale_expansion: 64,
            text_input_bytes: 64,
            text_scalars: 64,
            normalized_scalars: 64,
            unit_edges: 64,
            value_occurrences: 64,
            work_units: 4_096,
            result_units: 64,
        };
        let subject = SubjectLimits {
            expansion: ExpansionLimits { max_candidates: 2 },
            meter: &meter,
        };
        let cases = [
            (
                ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxStates,
                    value: 1_000,
                },
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::ExactProbMaxStates,
                    value: 1_000,
                }),
                None,
            ),
            (
                ExactProbBuildIssue::NotMarkov,
                TerminalValue::Unsupported(UnavailabilityCause::NotMarkov),
                None,
            ),
            (
                ExactProbBuildIssue::UndecidedSuccessor,
                TerminalValue::Inconclusive(InconclusiveCause::UndecidedSuccessor),
                None,
            ),
            (
                ExactProbBuildIssue::Expansion(ExpansionStop {
                    cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                    limit: Some(ExpansionLimit::MaxCandidates),
                }),
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::MaxCandidates,
                    value: 2,
                }),
                None,
            ),
            (
                ExactProbBuildIssue::Expansion(ExpansionStop {
                    cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                    limit: None,
                }),
                TerminalValue::Failed,
                Some(CatalogCode::new(
                    "resource_exhausted",
                    "insufficient-next-charge",
                )),
            ),
            (
                ExactProbBuildIssue::BrokenInvariant,
                TerminalValue::Failed,
                Some(BROKEN),
            ),
            (
                ExactProbBuildIssue::Cancelled(CancelCause::Requested),
                TerminalValue::Incomplete(IncompleteCause::Cancelled {
                    source: CancelCause::Requested,
                }),
                None,
            ),
            (
                ExactProbBuildIssue::PrecisionBudget {
                    lower: quire_exact::Rational::new(1_i64.into(), 3_i64.into())
                        .expect("a nonzero denominator"),
                    upper: quire_exact::Rational::new(2_i64.into(), 3_i64.into())
                        .expect("a nonzero denominator"),
                },
                TerminalValue::Incomplete(IncompleteCause::PrecisionBudget {
                    lower: Box::new(crate::proof_result::ExactRational::new(
                        quire_exact::Rational::new(1_i64.into(), 3_i64.into())
                            .expect("a nonzero denominator"),
                    )),
                    upper: Box::new(crate::proof_result::ExactRational::new(
                        quire_exact::Rational::new(2_i64.into(), 3_i64.into())
                            .expect("a nonzero denominator"),
                    )),
                }),
                None,
            ),
            (
                ExactProbBuildIssue::Expansion(ExpansionStop {
                    cause: CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
                    limit: Some(ExpansionLimit::EvaluationMeter(
                        quire_exact::LimitKind::WorkUnits,
                    )),
                }),
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::EvaluationMeter {
                        counter: quire_exact::LimitKind::WorkUnits,
                    },
                    value: 4_096,
                }),
                None,
            ),
            (
                ExactProbBuildIssue::Predicate(ClauseOutcome::KernelRefused(
                    quire_exact::Refusal::IntegerOutOfDomain {
                        target: Box::new(IntegerInterval::spanning(
                            Integer::from(0_i64),
                            Integer::from(1_i64),
                        )),
                    },
                )),
                TerminalValue::Failed,
                Some(CatalogCode::new("integer_out_of_domain", "outside-domain")),
            ),
            (
                ExactProbBuildIssue::Predicate(ClauseOutcome::KernelRefused(
                    quire_exact::Refusal::CheckedInvariant {
                        cause: quire_exact::CheckedInvariantCause::NativeIntegerCapacityBreach,
                    },
                )),
                TerminalValue::Failed,
                Some(BROKEN),
            ),
            (
                ExactProbBuildIssue::Predicate(ClauseOutcome::NotBoolean),
                TerminalValue::Failed,
                Some(BROKEN),
            ),
            (
                ExactProbBuildIssue::Failed(CatalogCode::new(
                    "invalid_runtime_input",
                    "invalid-value",
                )),
                TerminalValue::Failed,
                Some(CatalogCode::new("invalid_runtime_input", "invalid-value")),
            ),
            (
                ExactProbBuildIssue::Predicate(ClauseOutcome::Incomplete(
                    quire_exact::Incomplete {
                        limit_kind: quire_exact::LimitKind::WorkUnits,
                        limit: 9,
                        consumed: 9,
                        next_charge: quire_exact::Integer::from(1_i64),
                        charge_point: quire_exact::ChargePoint::TextInputBytes,
                    },
                )),
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::EvaluationMeter {
                        counter: quire_exact::LimitKind::WorkUnits,
                    },
                    value: 9,
                }),
                None,
            ),
        ];

        for (issue, expected, diagnostic) in cases {
            let record = settle_exact_prob_build_issue(&issue, subject);
            assert_eq!(record.value(), &expected);
            assert_eq!(record.diagnostic(), diagnostic);
            assert_ne!(
                record.value().category(),
                quire_semantic_value::catalog::Category::Success
            );
        }
    }
}

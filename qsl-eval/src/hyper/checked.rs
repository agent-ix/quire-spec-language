// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-487 "The clause seams": the production [`StepMatch`] of a checked hyper
//! clause, which evaluates each conjunct of `match` through the one clause
//! evaluator (FR-107).

use quire_exact::Meter;
use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedHyperClause;

use crate::hyper::{
    MatchIncomplete, MatchPart, MatchReading, ParameterBinding, StepLabel, StepMatch,
};
use crate::simulation::model::{verdict, ClauseOutcome, Verdict, BROKEN};
use crate::value::CheckedPackageEvaluation;

/// The `match` conjuncts of one checked hyper clause, evaluated over the
/// labels of a joint step.
///
/// The conjuncts of a part are evaluated in index order, each once, sharing
/// the one `meter`, and the first that is not `Completed(true)` ends the
/// reading: a conjunct after it is neither evaluated nor charged, as FR-107
/// skips the right operand of a decided `and`. A step label is a transition
/// identity, so a conjunct reads no state and the instance is not read
/// (ADR-023 HS-5).
pub struct CheckedStepMatch<'s> {
    package: &'s CheckedPackage,
    clause: &'s CheckedHyperClause,
}

impl<'s> CheckedStepMatch<'s> {
    /// The `match` of `clause`, evaluated through `package`.
    pub fn new(package: &'s CheckedPackage, clause: &'s CheckedHyperClause) -> Self {
        Self { package, clause }
    }

    fn read(
        &self,
        part: MatchPart,
        labels: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, MatchIncomplete> {
        let count = match part {
            MatchPart::Universal => self.clause.mu_universal().len(),
            MatchPart::Existential => self.clause.mu_existential().len(),
        };
        for conjunct in 0..count {
            let evaluation = match self
                .package
                .evaluate_step_predicate(self.clause, part, conjunct, labels, meter)
            {
                Ok(evaluation) => evaluation,
                // The labels and the conjunct are the product's own, so an
                // evaluator fault is a broken invariant; the seam has no
                // fault channel, and the product reads a refusal.
                Err(_) => {
                    return Ok(MatchReading::Refused {
                        conjunct,
                        outcome: ClauseOutcome::FamilyRefused(BROKEN),
                    })
                }
            };
            match verdict(&evaluation.outcome) {
                Verdict::True => {}
                Verdict::False => return Ok(MatchReading::Fails),
                Verdict::Undecided(outcome) => {
                    if let Some(cause) = outcome.undefined_cause() {
                        return Ok(MatchReading::Undefined { conjunct, cause });
                    }
                    return match outcome {
                        ClauseOutcome::Incomplete(incomplete) => Err(MatchIncomplete {
                            conjunct,
                            incomplete,
                        }),
                        outcome => Ok(MatchReading::Refused { conjunct, outcome }),
                    };
                }
            }
        }
        Ok(MatchReading::Holds)
    }
}

impl StepMatch for CheckedStepMatch<'_> {
    fn universal(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, MatchIncomplete> {
        self.read(MatchPart::Universal, steps, meter)
    }

    fn existential(
        &self,
        _instance: &[ParameterBinding],
        steps: &[StepLabel<'_>],
        meter: &mut Meter,
    ) -> Result<MatchReading, MatchIncomplete> {
        self.read(MatchPart::Existential, steps, meter)
    }
}

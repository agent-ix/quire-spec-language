// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330 (FR-327, FR-328, FR-329): a temporal clause evaluated over
//! observed snapshots, the one clause evaluator (FR-107) filling the
//! [`TemporalLetters`] seam.
//!
//! Each position of the trace is the object environment its snapshot
//! admitted. A `holds` atom reads its condition through
//! [`CheckedPackageEvaluation::evaluate_holds`] with the clause's `over`
//! object bound; a lasso position past the represented ones reads the
//! environment its wrapped index names.

use qsl_foundation::InternalFault;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{FormulaNodeId, TemporalClauseNode};
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Meter, ObjectReference, Outcome, Value};

use super::{
    evaluate, AtomReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalRefusal, TemporalTrace, UndefinedCause,
};
use crate::value::CheckedPackageEvaluation;

/// Evaluate `clause`, a temporal clause of `package`, over `environments`,
/// the object environment of each represented position, prefix first, then
/// loop. `trace` is their shape and `over` the object the clause's `over`
/// parameter is bound to.
///
/// A trace whose shape and environments disagree is a fault, not a
/// refusal: the caller builds both from one trace.
pub fn evaluate_observed(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    environments: &[ObjectEnvironment],
    trace: TemporalTrace,
    over: &ObjectReference,
    meter: &mut Meter,
) -> Result<TemporalOutcome, TemporalRefusal> {
    let mut letters = ObservedLetters {
        package,
        clause,
        environments,
        trace,
        over,
    };
    evaluate(clause.clause(), trace, &mut letters, meter)
}

struct ObservedLetters<'a> {
    package: &'a CheckedPackage,
    clause: &'a TemporalClauseNode,
    environments: &'a [ObjectEnvironment],
    trace: TemporalTrace,
    over: &'a ObjectReference,
}

impl ObservedLetters<'_> {
    /// The environment position `position` observes: a lasso wraps past its
    /// last represented position into the loop.
    fn environment(&self, position: TemporalPosition) -> Result<&ObjectEnvironment, LetterStop> {
        let index = position.index();
        let slot = match self.trace {
            TemporalTrace::Finite { .. } => Some(index),
            TemporalTrace::Lasso { prefix, loop_len } => {
                if index < prefix {
                    Some(index)
                } else {
                    // `loop_len` is at least 1: the evaluator refuses an empty
                    // loop before it reads a letter.
                    (index - prefix)
                        .checked_rem(loop_len)
                        .and_then(|offset| prefix.checked_add(offset))
                }
            }
        };
        slot.and_then(|slot| usize::try_from(slot).ok())
            .and_then(|slot| self.environments.get(slot))
            .ok_or_else(|| {
                LetterStop::Fault(InternalFault::new(
                    "temporal.observed",
                    "position-has-an-environment",
                ))
            })
    }
}

impl TemporalLetters for ObservedLetters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let objects = self.environment(position)?;
        let evaluation = self
            .package
            .evaluate_holds(
                self.clause,
                atom,
                Value::Reference(self.over.clone()),
                objects,
                meter,
            )
            .map_err(|failure| {
                LetterStop::Fault(match failure {
                    crate::value::CallFailure::Fault(fault) => fault,
                    _ => InternalFault::new("temporal.observed", "holds-condition-evaluates"),
                })
            })?;
        let fault = |code: &'static str| LetterStop::Fault(InternalFault::new("temporal.observed", code));
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true))) => {
                Ok(AtomReading::True)
            }
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false))) => {
                Ok(AtomReading::False)
            }
            FamilyOutcome::Evaluated(Outcome::Completed(_)) => Err(fault("holds-condition-boolean")),
            FamilyOutcome::Evaluated(Outcome::Undefined(undefined)) => {
                Ok(AtomReading::Undefined(UndefinedCause::Kernel(undefined)))
            }
            FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                Err(LetterStop::Incomplete(incomplete))
            }
            FamilyOutcome::Evaluated(Outcome::Refused(_)) => Err(fault("holds-condition-refused")),
            FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => Ok(
                AtomReading::Undefined(UndefinedCause::Family(cause.undefined_record())),
            ),
            FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => {
                Err(fault("holds-condition-refused"))
            }
            #[cfg(seam_probe_eval_downstream)]
            FamilyOutcome::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        // An emitted clause activates `on origin` (FR-337), so it has no
        // `when` condition to read.
        Err(LetterStop::Fault(InternalFault::new(
            "temporal.observed",
            "emitted-clause-has-no-activation-condition",
        )))
    }
}

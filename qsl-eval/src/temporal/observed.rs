// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-330 (FR-327, FR-328, FR-329): a temporal clause evaluated over
//! observed snapshots, the one clause evaluator (FR-107) filling the
//! [`TemporalLetters`] seam.
//!
//! Each position of the trace is the object environment its snapshot
//! admitted. A `holds` atom reads its condition through
//! [`CheckedPackageEvaluation::evaluate_holds`] with the clause's `over`
//! object bound, and an `on each` guard through
//! [`CheckedPackageEvaluation::evaluate_guard`] with both binders bound to
//! it; a lasso position past the represented ones reads the environment its
//! wrapped index names.

use qsl_package::CheckedPackage;
use qsl_semantics::check::{FormulaNodeId, TemporalClauseNode};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{FamilyOutcome, FamilyResult};
use quire_exact::{Meter, ObjectReference, Outcome, Value};
use quire_semantic_value::catalog::InternalFault;

use super::finite_window::{evaluate_finite_window, TimestampedTrace, Valuation, WindowEvaluation};
use super::{
    AtomReading, CaptureReading, LetterStop, TemporalLetters, TemporalOutcome, TemporalPosition,
    TemporalRefusal, TemporalTrace, UndefinedCause,
};
use crate::value::{evaluate_capture, CheckedPackageEvaluation};
use quire_eval::{CallFailure, Evaluation};

/// Why a `holds` condition stopped the run without a reading, kept whole so
/// the caller reports it as FR-109 reports the same S6a result of a state
/// clause (a refusal with its record; a failed call by its own mapping).
#[derive(Debug)]
pub enum HoldsStop {
    /// The condition's S6a result was a refusal at `at`, the position of
    /// the condition (FR-327 "Refused condition").
    Refused {
        /// The position the refused condition was read at.
        at: TemporalPosition,
        /// The refused S6a evaluation, whole.
        evaluation: Evaluation,
    },
    /// The call failed before or during S6a.
    Failed(CallFailure),
}

/// What [`evaluate_observed`] returns: the evaluator's result and, when a
/// condition stopped the run, why (the result is then the stop's fault).
#[derive(Debug)]
pub struct ObservedRun {
    /// The evaluator's result.
    pub result: Result<TemporalOutcome, TemporalRefusal>,
    /// The condition stop that ended the run, if one did.
    pub stopped: Option<HoldsStop>,
    /// The first undefined reading, with its locus, trace position and cause.
    pub undefined: Option<FirstUndefined>,
}

/// The expression an observed run first read as undefined: a `holds` atom,
/// the `on each` activation guard or a capture initializer (FR-337).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservedLocus {
    /// A `holds` atom, by its formula node.
    Atom(FormulaNodeId),
    /// The `when` condition of the clause's guarded `on each` activation.
    Activation,
    /// A capture's initializer, by the capture's name.
    Capture(String),
}

/// The first undefined reading of an observed run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FirstUndefined {
    /// The expression read as undefined.
    pub locus: ObservedLocus,
    /// The trace position it was read at.
    pub position: TemporalPosition,
    /// Why it had no value.
    pub cause: UndefinedCause,
}

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
) -> ObservedRun {
    evaluate_observed_goal(
        package,
        clause,
        environments,
        trace,
        over,
        super::engine::Goal::Clause,
        meter,
    )
}

/// Evaluate the temporal formula at one represented position of an
/// observed trace. Unlike [`evaluate_observed`], this does not apply the
/// clause's activation rule: a replay packet names the position whose
/// claimed value is being checked (FR-331).
pub fn evaluate_observed_at(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    environments: &[ObjectEnvironment],
    trace: TemporalTrace,
    over: &ObjectReference,
    position: TemporalPosition,
    meter: &mut Meter,
) -> ObservedRun {
    evaluate_observed_goal(
        package,
        clause,
        environments,
        trace,
        over,
        super::engine::Goal::At(position),
        meter,
    )
}

fn evaluate_observed_goal(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    environments: &[ObjectEnvironment],
    trace: TemporalTrace,
    over: &ObjectReference,
    goal: super::engine::Goal,
    meter: &mut Meter,
) -> ObservedRun {
    let mut letters = ObservedLetters {
        package,
        clause,
        environments,
        trace,
        over,
        stopped: None,
        undefined: None,
        absent: &[],
        classes: &[],
        silent_from: None,
    };
    let result = super::engine::run(
        super::engine::Subject::of_clause(clause.clause()),
        trace,
        goal,
        &mut letters,
        meter,
    );
    // A refusal the result does not stop at was passed over: an activation
    // before it evaluated false, or an atom before it was undefined, and
    // that result is the run's (FR-327 "Refused condition").
    let stopped = match (letters.stopped, &result) {
        (Some(HoldsStop::Refused { .. }), Ok(_)) => None,
        (stopped, _) => stopped,
    };
    ObservedRun {
        result,
        stopped,
        undefined: letters.undefined,
    }
}

/// What a finite-window run reads of each atom beyond the observations.
#[derive(Clone, Copy, Debug)]
pub struct WindowValuations<'a> {
    /// The `(position, atom node)` valuations absent from the admitted
    /// observations.
    pub absent: &'a [(u64, u64)],
    /// Each formula node's atom class: a node index shared by the nodes that
    /// are one atom (the same `holds` condition), so they read one
    /// valuation; empty makes every node its own atom.
    pub classes: &'a [usize],
    /// The first position that is a silent sample (FR-504): it carries no
    /// observation and every `holds` atom there reads false. `None` when
    /// every position is an observation.
    pub silent_from: Option<u64>,
}

/// What [`evaluate_observed_finite_window`] returns: the evaluator's result
/// and, when a condition stopped the run, why (the result is then the stop's
/// fault).
#[derive(Debug)]
pub struct ObservedWindowRun {
    /// The evaluator's result.
    pub result: Result<WindowEvaluation, TemporalRefusal>,
    /// The condition stop that ended the run, if one did.
    pub stopped: Option<HoldsStop>,
    /// The first undefined reading, with its locus, trace position and cause.
    pub undefined: Option<FirstUndefined>,
}

/// Evaluate `clause` under the finite-window profile (FR-501) over the object
/// environment of each position of `trace` in canonical order, reading the
/// possibility sets of its atoms, and return the full result report: verdict,
/// basis, decision support, premises and result identity. `valuations` names
/// the `(position, atom node)` valuations absent from the admitted
/// observations; two observations of one stamp that read differently are a
/// conflict the evaluator finds itself.
pub fn evaluate_observed_finite_window(
    package: &CheckedPackage,
    clause: &TemporalClauseNode,
    environments: &[ObjectEnvironment],
    trace: &TimestampedTrace,
    over: &ObjectReference,
    valuations: WindowValuations<'_>,
    meter: &mut Meter,
) -> ObservedWindowRun {
    let mut letters = ObservedLetters {
        package,
        clause,
        environments,
        trace: TemporalTrace::Finite {
            len: u64::try_from(trace.len()).unwrap_or(u64::MAX),
        },
        over,
        stopped: None,
        undefined: None,
        absent: valuations.absent,
        classes: valuations.classes,
        silent_from: valuations.silent_from,
    };
    let result = evaluate_finite_window(
        &crate::temporal::facts::ClauseFacts::of_checked(clause.clause()),
        trace,
        &mut letters,
        meter,
    );
    // As `evaluate_observed`: a refusal the result passed over is dropped.
    let stopped = match (letters.stopped, &result) {
        (Some(HoldsStop::Refused { .. }), Ok(_)) => None,
        (stopped, _) => stopped,
    };
    ObservedWindowRun {
        result,
        stopped,
        undefined: letters.undefined,
    }
}

/// The fault codes of a `holds` condition: a failed call and a non-Boolean
/// completion. A refusal is a refused condition, not a fault.
const HOLDS_CODES: [&str; 2] = ["holds-condition-call-failed", "holds-condition-boolean"];

/// The fault codes of an `on each` guard, as [`HOLDS_CODES`].
const ACTIVATION_CODES: [&str; 2] = [
    "activation-condition-call-failed",
    "activation-condition-boolean",
];

struct ObservedLetters<'a> {
    package: &'a CheckedPackage,
    clause: &'a TemporalClauseNode,
    environments: &'a [ObjectEnvironment],
    trace: TemporalTrace,
    over: &'a ObjectReference,
    stopped: Option<HoldsStop>,
    /// The first undefined reading (the evaluator reports its position and
    /// cause, not its locus).
    undefined: Option<FirstUndefined>,
    /// The `(position, atom node)` valuations the finite-window input names
    /// as absent from the admitted observation (QSpec FR-160).
    absent: &'a [(u64, u64)],
    /// Each node's atom class (empty: every node is its own atom).
    classes: &'a [usize],
    /// The first silent sample, where every `holds` atom reads false.
    silent_from: Option<u64>,
}

impl ObservedLetters<'_> {
    /// Keep the first undefined reading at the earliest position read, the
    /// position the evaluator reports (ADR-018 UE-1).
    fn note(
        &mut self,
        locus: impl FnOnce() -> ObservedLocus,
        position: TemporalPosition,
        cause: &UndefinedCause,
    ) {
        if self
            .undefined
            .as_ref()
            .is_none_or(|first| position.index() < first.position.index())
        {
            self.undefined = Some(FirstUndefined {
                locus: locus(),
                position,
                cause: cause.clone(),
            });
        }
    }

    /// Whether `position` is a silent sample (FR-504).
    fn is_silent(&self, position: TemporalPosition) -> bool {
        self.silent_from
            .is_some_and(|first| position.index() >= first)
    }

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

    /// The reading of a condition's S6a `evaluation`; the fault codes name
    /// the condition (`HOLDS_CODES`, `ACTIVATION_CODES`). A refusal or a
    /// failed call is kept whole in `stopped` and ends the run.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn reading(
        stopped: &mut Option<HoldsStop>,
        position: TemporalPosition,
        evaluation: Result<Evaluation, CallFailure>,
        [failed, boolean]: [&'static str; 2],
    ) -> Result<AtomReading, LetterStop> {
        let stop =
            |code: &'static str| LetterStop::Fault(InternalFault::new("temporal.observed", code));
        let evaluation = match evaluation {
            Ok(evaluation) => evaluation,
            Err(failure) => {
                *stopped = Some(HoldsStop::Failed(failure));
                return Err(stop(failed));
            }
        };
        let refused = matches!(
            evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Refused(_))
                | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))
        );
        if refused {
            *stopped = Some(HoldsStop::Refused {
                at: position,
                evaluation,
            });
            return Err(LetterStop::Refused);
        }
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true))) => {
                Ok(AtomReading::True)
            }
            FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false))) => {
                Ok(AtomReading::False)
            }
            FamilyOutcome::Evaluated(Outcome::Completed(_)) => Err(stop(boolean)),
            FamilyOutcome::Evaluated(Outcome::Undefined(undefined)) => {
                Ok(AtomReading::Undefined(UndefinedCause::Kernel(undefined)))
            }
            FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => {
                Err(LetterStop::Incomplete(incomplete))
            }
            FamilyOutcome::Evaluated(Outcome::Refused(_))
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => Err(LetterStop::Refused),
            FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => Ok(
                AtomReading::Undefined(UndefinedCause::Family(cause.undefined_record())),
            ),
            #[cfg(seam_probe_eval_downstream)]
            FamilyOutcome::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }
}

impl TemporalLetters for ObservedLetters<'_> {
    #[deny(clippy::wildcard_enum_match_arm)]
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        if self.is_silent(position) {
            return Ok(AtomReading::False);
        }
        let objects = self.environment(position)?;
        let evaluation = self.package.evaluate_holds(
            self.clause,
            atom,
            Value::Reference(self.over.clone()),
            objects,
            meter,
        );
        let reading = Self::reading(&mut self.stopped, position, evaluation, HOLDS_CODES)?;
        if let AtomReading::Undefined(cause) = &reading {
            self.note(|| ObservedLocus::Atom(atom), position, cause);
        }
        Ok(reading)
    }

    fn valuation(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<Valuation, LetterStop> {
        let key = u64::try_from(atom.index())
            .map(|atom| (position.index(), atom))
            .map_err(|_| {
                LetterStop::Fault(InternalFault::new("temporal.observed", "atom-in-u64"))
            })?;
        if self.absent.contains(&key) {
            return Ok(Valuation::Missing);
        }
        self.atom(atom, position, meter).map(Valuation::from)
    }

    fn atom_class(&self, atom: FormulaNodeId) -> usize {
        self.classes
            .get(atom.index())
            .copied()
            .unwrap_or(atom.index())
    }

    fn activation(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let objects = self.environment(position)?;
        let evaluation = self.package.evaluate_guard(
            self.clause,
            Value::Reference(self.over.clone()),
            objects,
            meter,
        );
        let reading = Self::reading(&mut self.stopped, position, evaluation, ACTIVATION_CODES)?;
        if let AtomReading::Undefined(cause) = &reading {
            self.note(|| ObservedLocus::Activation, position, cause);
        }
        Ok(reading)
    }

    fn captures(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        self.fix_captures(position, meter)
    }
}

impl ObservedLetters<'_> {
    /// FR-337 "Capture observation": the clause's captures at `position`, in
    /// source order, each through the clause evaluator with the `over`
    /// parameter and the trigger binder both `over` and each earlier capture
    /// bound to its value.
    fn fix_captures(
        &mut self,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<CaptureReading, LetterStop> {
        let clause = self.clause.clause();
        if clause.captures().len() == 0 {
            return Ok(CaptureReading::Fixed);
        }
        let objects = self.environment(position)?;
        let binders = 1 + usize::from(clause.trigger().is_some());
        let mut values: Vec<Value> = Vec::new();
        for (index, (name, _)) in clause.captures().enumerate() {
            let (body, slots) = self.clause.capture(index).ok_or_else(|| {
                LetterStop::Fault(InternalFault::new(
                    "temporal.observed",
                    "clause-has-its-typed-captures",
                ))
            })?;
            let mut arguments = vec![Value::Reference(self.over.clone()); binders];
            arguments.extend(values.iter().cloned());
            let evaluation = evaluate_capture(
                self.package,
                &Capture { body, slots },
                objects,
                arguments,
                None,
                meter,
            )
            .map_err(LetterStop::Fault)?;
            if let FamilyOutcome::Evaluated(Outcome::Completed(value)) = &evaluation.outcome {
                values.push(value.clone());
                continue;
            }
            return match Self::reading(&mut self.stopped, position, Ok(evaluation), CAPTURE_CODES)?
            {
                AtomReading::Undefined(cause) => {
                    self.note(|| ObservedLocus::Capture(name.to_owned()), position, &cause);
                    Ok(CaptureReading::Undefined {
                        capture: name.to_owned(),
                        cause,
                    })
                }
                AtomReading::True | AtomReading::False => Err(LetterStop::Fault(
                    InternalFault::new("temporal.observed", CAPTURE_CODES[1]),
                )),
            };
        }
        Ok(CaptureReading::Fixed)
    }
}

/// The fault codes of a capture's initializer, as [`HOLDS_CODES`].
const CAPTURE_CODES: [&str; 2] = [
    "capture-initializer-call-failed",
    "capture-initializer-completes-a-value",
];

/// One capture's initializer as a predicate of the clause evaluator.
struct Capture<'n> {
    body: qsl_semantics::check::CheckedNode<'n>,
    slots: usize,
}

impl qsl_semantics::check::StatePredicate for Capture<'_> {
    fn body(&self) -> qsl_semantics::check::CheckedNode<'_> {
        self.body
    }

    fn slots(&self) -> usize {
        self.slots
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{CheckedInvariantCause, Refusal};

    use super::*;

    /// FR-327-AC-6: a condition whose S6a evaluation is the kernel
    /// `Refusal::CheckedInvariant` at position 1 is kept whole as a refused
    /// condition at position 1, ending the run with `LetterStop::Refused`
    /// and not as a fault, as every other refusal is; FR-330 reports it as
    /// an internal failure (the report layer's mapping).
    #[trace("TC-1160", "FR-327-AC-6")]
    #[test]
    fn a_checked_invariant_refusal_is_a_refused_condition_at_its_position() {
        let mut stopped = None;
        let evaluation = Evaluation {
            outcome: FamilyOutcome::Evaluated(Outcome::Refused(Refusal::CheckedInvariant {
                cause: CheckedInvariantCause::EqualityOperandSourceNotAdmitted,
            })),
            location: None,
            losses: Vec::new(),
        };
        let reading = ObservedLetters::reading(
            &mut stopped,
            TemporalPosition::new(1),
            Ok(evaluation),
            HOLDS_CODES,
        );
        assert_eq!(reading, Err(LetterStop::Refused));
        let Some(HoldsStop::Refused { at, evaluation }) = stopped else {
            panic!("the refusal is kept whole, got {stopped:?}");
        };
        assert_eq!(at, TemporalPosition::new(1));
        assert!(matches!(
            evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Refused(Refusal::CheckedInvariant {
                cause: CheckedInvariantCause::EqualityOperandSourceNotAdmitted
            }))
        ));
    }

    /// A family refusal with the catalog code `invalid_runtime_input`/
    /// `absent-key`.
    #[derive(Debug)]
    struct FamilyCause;

    impl quire_semantic_value::catalog::CatalogCoded for FamilyCause {
        fn catalog_code(&self) -> quire_semantic_value::catalog::CatalogCode {
            quire_semantic_value::catalog::CatalogCode::new("invalid_runtime_input", "absent-key")
        }

        fn catalog_fields(&self) -> Option<std::collections::BTreeMap<&'static str, String>> {
            None
        }
    }

    /// FR-327 "Refused condition": a family's `FamilyResult::Refused` is a
    /// refused condition as a kernel refusal is, kept whole at its position.
    #[trace("TC-1160", "FR-327-AC-6")]
    #[test]
    fn a_family_refusal_is_a_refused_condition_at_its_position() {
        let mut stopped = None;
        let evaluation = Evaluation {
            outcome: FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(Box::new(FamilyCause))),
            location: None,
            losses: Vec::new(),
        };
        let reading = ObservedLetters::reading(
            &mut stopped,
            TemporalPosition::new(2),
            Ok(evaluation),
            HOLDS_CODES,
        );
        assert_eq!(reading, Err(LetterStop::Refused));
        let Some(HoldsStop::Refused { at, evaluation }) = stopped else {
            panic!("the refusal is kept whole, got {stopped:?}");
        };
        assert_eq!(at, TemporalPosition::new(2));
        assert!(matches!(
            evaluation.outcome,
            FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))
        ));
    }
}

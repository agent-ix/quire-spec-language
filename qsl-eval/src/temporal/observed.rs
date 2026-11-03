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
    /// The condition's S6a result was a refusal.
    Refused(Evaluation),
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
    /// The first undefined atom read, with its trace position and cause.
    pub undefined: Option<(FormulaNodeId, TemporalPosition, UndefinedCause)>,
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
        conflicting: &[],
    };
    let result = super::engine::run(
        super::engine::Subject::of_clause(clause.clause()),
        trace,
        goal,
        &mut letters,
        meter,
    );
    ObservedRun {
        result,
        stopped: letters.stopped,
        undefined: letters.undefined,
    }
}

/// What a finite-window run reads of each atom beyond the observations.
#[derive(Clone, Copy, Debug)]
pub struct WindowValuations<'a> {
    /// The `(position, atom node)` valuations absent from the admitted
    /// observations.
    pub absent: &'a [(u64, u64)],
    /// The `(position, atom node)` valuations admitted with two differing
    /// values.
    pub conflicting: &'a [(u64, u64)],
    /// Each formula node's atom class: a node index shared by the nodes that
    /// are one atom (the same `holds` condition), so they read one
    /// valuation; empty makes every node its own atom.
    pub classes: &'a [usize],
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
}

/// Evaluate `clause` under the finite-window profile (FR-501) over the object
/// environment of each position of `trace` in canonical order, reading the
/// possibility sets of its atoms, and return the full result report: verdict,
/// basis, decision support, premises and result identity. `absent` and
/// `conflicting` name the `(position, atom node)` valuations that are absent
/// from the admitted observations or admitted with two differing values.
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
        conflicting: valuations.conflicting,
    };
    let result = evaluate_finite_window(clause.clause(), trace, &mut letters, meter);
    ObservedWindowRun {
        result,
        stopped: letters.stopped,
    }
}

/// The fault codes of a `holds` condition: a failed call, a refusal and a
/// non-Boolean completion.
const HOLDS_CODES: [&str; 3] = [
    "holds-condition-call-failed",
    "holds-condition-refused",
    "holds-condition-boolean",
];

/// The fault codes of an `on each` guard, as [`HOLDS_CODES`].
const ACTIVATION_CODES: [&str; 3] = [
    "activation-condition-call-failed",
    "activation-condition-refused",
    "activation-condition-boolean",
];

struct ObservedLetters<'a> {
    package: &'a CheckedPackage,
    clause: &'a TemporalClauseNode,
    environments: &'a [ObjectEnvironment],
    trace: TemporalTrace,
    over: &'a ObjectReference,
    stopped: Option<HoldsStop>,
    undefined: Option<(FormulaNodeId, TemporalPosition, UndefinedCause)>,
    /// The `(position, atom node)` valuations the finite-window input names
    /// as absent from the admitted observation (QSpec FR-160).
    absent: &'a [(u64, u64)],
    /// Each node's atom class (empty: every node is its own atom).
    classes: &'a [usize],
    /// The `(position, atom node)` valuations at which two differing
    /// valuations were admitted (QSpec FR-160).
    conflicting: &'a [(u64, u64)],
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

    /// The reading of a condition's S6a `evaluation`; the fault codes name
    /// the condition (`HOLDS_CODES`, `ACTIVATION_CODES`). A refusal or a
    /// failed call is kept whole in `stopped` and ends the run.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn reading(
        &mut self,
        evaluation: Result<Evaluation, CallFailure>,
        [failed, refused_code, boolean]: [&'static str; 3],
    ) -> Result<AtomReading, LetterStop> {
        let stop =
            |code: &'static str| LetterStop::Fault(InternalFault::new("temporal.observed", code));
        let evaluation = match evaluation {
            Ok(evaluation) => evaluation,
            Err(failure) => {
                self.stopped = Some(HoldsStop::Failed(failure));
                return Err(stop(failed));
            }
        };
        let refused = matches!(
            evaluation.outcome,
            FamilyOutcome::Evaluated(Outcome::Refused(_))
                | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_))
        );
        if refused {
            self.stopped = Some(HoldsStop::Refused(evaluation));
            return Err(stop(refused_code));
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
            | FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(_)) => Err(stop(refused_code)),
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
        let objects = self.environment(position)?;
        let evaluation = self.package.evaluate_holds(
            self.clause,
            atom,
            Value::Reference(self.over.clone()),
            objects,
            meter,
        );
        let reading = self.reading(evaluation, HOLDS_CODES)?;
        if let AtomReading::Undefined(cause) = &reading {
            if self.undefined.is_none() {
                self.undefined = Some((atom, position, cause.clone()));
            }
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
        if self.conflicting.contains(&key) {
            return Ok(Valuation::Conflicting);
        }
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
        self.reading(evaluation, ACTIVATION_CODES)
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
            return match self.reading(Ok(evaluation), CAPTURE_CODES)? {
                AtomReading::Undefined(cause) => Ok(CaptureReading::Undefined {
                    capture: name.to_owned(),
                    cause,
                }),
                AtomReading::True | AtomReading::False => Err(LetterStop::Fault(
                    InternalFault::new("temporal.observed", CAPTURE_CODES[2]),
                )),
            };
        }
        Ok(CaptureReading::Fixed)
    }
}

/// The fault codes of a capture's initializer, as [`HOLDS_CODES`].
const CAPTURE_CODES: [&str; 3] = [
    "capture-initializer-call-failed",
    "capture-initializer-refused",
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

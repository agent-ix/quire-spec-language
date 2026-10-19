// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-124 "Deadlocked states" (ADR-018 DL-1, DL-2): how a model state reads
//! against the package's `terminal` declarations.
//!
//! A state is terminal when no transition identity is enabled at it, which
//! the caller reads from the state's own expansion. A terminal state is
//! intended when a declaration marks it so: `terminal any`, or `terminal
//! when P` with `P` true at the state. `P` is read only at a terminal state,
//! through the one clause evaluator (FR-107), and like an invariant it holds
//! at a state when it holds for every object of the state whose type
//! conforms to `P`'s `self`. The state graph alone decides; no fairness
//! constraint is read.

use qsl_semantics::check::{CheckedTerminalPredicate, TerminalDeclaration};
use quire_exact::{Meter, ValueType};

use super::contracts::{verdict, Verdict};
use super::walk::{exhausted, stop, walk_of};
use super::{ClauseOutcome, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::temporal::UndefinedCause;
use crate::value::evaluate_terminal_predicate;

/// FR-124's classification of one state of a subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TerminalClass {
    /// A transition identity is enabled at the state.
    NotTerminal,
    /// The state is terminal and a declaration intends it.
    Intended,
    /// The state is terminal and no declaration intends it: a deadlock.
    Deadlocked,
    /// A `terminal when` predicate is undefined at the terminal state, so
    /// the state is neither intended nor deadlocked (ADR-018 UE-6).
    Undefined {
        /// The model alias whose declaration it is.
        model: String,
        /// The first undefined evaluation, in object order.
        cause: UndefinedCause,
    },
    /// A `terminal when` predicate is refused at the terminal state, with
    /// none undefined and none true: whether the state is intended is not
    /// known (ADR-018 V-6 `UndecidedSuccessor`).
    Undecided {
        /// The model alias whose declaration it is.
        model: String,
    },
}

/// How one `terminal when` predicate read at one state.
enum Reading {
    True,
    False,
    Undefined(UndefinedCause),
    Refused,
}

impl ModelSystem<'_> {
    /// FR-124: the class of `state`, whose expansion gave a successor
    /// exactly when `has_successor`. The package's declarations are read in
    /// model-alias order: `Any` intends every terminal state; `When(P)`
    /// intends it when `P` is true. An undefined `P` reads before a later
    /// declaration is consulted, so the class is a function of the state.
    ///
    /// # Errors
    ///
    /// `resource_exhausted`/`insufficient-next-charge` naming the evaluation
    /// meter when `P`'s evaluation exhausts it;
    /// `runtime_invariant`/`established-invariant-broken` when the state
    /// cannot be observed or the evaluator faults.
    pub fn terminal_class(
        &self,
        state: &ModelSimState,
        has_successor: bool,
    ) -> Result<TerminalClass, ExpansionStop> {
        if has_successor {
            return Ok(TerminalClass::NotTerminal);
        }
        for terminal in self.package.graph().terminals() {
            match terminal.declaration() {
                TerminalDeclaration::None => {}
                TerminalDeclaration::Any => return Ok(TerminalClass::Intended),
                TerminalDeclaration::When(predicate) => match self.read(state, predicate)? {
                    Reading::True => return Ok(TerminalClass::Intended),
                    Reading::False => {}
                    Reading::Undefined(cause) => {
                        return Ok(TerminalClass::Undefined {
                            model: terminal.model().to_owned(),
                            cause,
                        })
                    }
                    Reading::Refused => {
                        return Ok(TerminalClass::Undecided {
                            model: terminal.model().to_owned(),
                        })
                    }
                },
            }
        }
        Ok(TerminalClass::Deadlocked)
    }

    /// `predicate` at `state`: each conforming object in ascending
    /// reference order, each with a fresh meter. The first undefined or
    /// refused object decides; otherwise any `false` object makes it false.
    fn read(
        &self,
        state: &ModelSimState,
        predicate: &CheckedTerminalPredicate,
    ) -> Result<Reading, ExpansionStop> {
        let Some((_, ValueType::Reference(context))) = predicate.parameters().first() else {
            return Err(stop(BROKEN));
        };
        let walk = walk_of(self, state, None)?;
        let mut objects: Vec<_> = walk
            .rows
            .iter()
            .map(|row| row.reference.clone())
            .filter(|reference| self.types.conforms(reference.object_type(), *context))
            .collect();
        objects.sort();
        let mut holds = true;
        for object in objects {
            let evaluation = evaluate_terminal_predicate(
                self.package,
                predicate,
                &walk.observation.environment,
                object,
                &mut Meter::new(self.meter),
            )
            .map_err(|_| stop(BROKEN))?;
            match verdict(&evaluation.outcome) {
                Verdict::True => {}
                Verdict::False => holds = false,
                Verdict::Undecided(outcome) => {
                    if let Some(cause) = outcome.undefined_cause() {
                        return Ok(Reading::Undefined(cause));
                    }
                    if matches!(outcome, ClauseOutcome::Incomplete(_)) {
                        return Err(exhausted(ExpansionLimit::EvaluationMeter));
                    }
                    return Ok(Reading::Refused);
                }
            }
        }
        Ok(if holds { Reading::True } else { Reading::False })
    }
}

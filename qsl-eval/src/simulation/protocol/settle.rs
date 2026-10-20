// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 FO-1 and FO-2, PS-6: settling. After a step, each thread it moved
//! or created takes structural moves until it reaches a rest point: into
//! and out of a `sequence`, into the one `case` of a `choice` whose guard is
//! true, through a `repeat`'s guard, body and `exhausted` control with its
//! counter, past a true `check` or to rest at a false one, into an
//! `await`'s `then` after its matched event, and to `end` or `finish`.
//!
//! A guard or `check` reads binders and constants only, through the one
//! clause evaluator, so settling is a function of the state. A guard the
//! evaluator does not decide ends the settling with its finding instead of
//! a rest point, and the step that caused it is not taken.

use qsl_semantics::check::{CheckedBody, ControlStep, ProtocolNodeId};

use super::identity::ThreadPath;
use super::memory::MemoryModel;
use super::program::{Cursor, Rest};
use super::{undetermined, Instance, ProtocolRefusal, ProtocolSystem};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::model::{ModelFinding, ModelSimState, Verdict};

/// Where a thread's settling ends.
pub(super) enum Settled {
    /// At a rest point.
    Rest(Rest),
    /// At a guard or `check` the evaluator does not decide, as its finding.
    Undecided(Box<ModelFinding>),
}

/// Why a settling stopped: an invariant of the checked protocol broke, or
/// an evaluation stopped. Kept apart so an initial state reports which.
#[derive(Debug)]
pub(super) enum SettleError {
    /// An invariant of the checked protocol broke.
    Refused(ProtocolRefusal),
    /// A guard's evaluation stopped.
    Stopped(ExpansionStop),
}

impl From<ProtocolRefusal> for SettleError {
    fn from(refusal: ProtocolRefusal) -> Self {
        Self::Refused(refusal)
    }
}

impl From<ExpansionStop> for SettleError {
    fn from(stop: ExpansionStop) -> Self {
        Self::Stopped(stop)
    }
}

impl From<SettleError> for ExpansionStop {
    fn from(error: SettleError) -> Self {
        match error {
            SettleError::Refused(refusal) => refusal.into(),
            SettleError::Stopped(stop) => stop,
        }
    }
}

/// A guard's value, or the finding of an undecided one.
enum Decision {
    Value(bool),
    Undecided(Box<ModelFinding>),
}

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// FO-1: the structural moves of the thread `path` of `instance` from
    /// `cursor` until a rest point, with the `repeat` counters it enters,
    /// advances and leaves written to `instance`. `model` is the state the
    /// guards are evaluated at; they read no model state (FO-2).
    pub(super) fn settle(
        &self,
        instance: &mut Instance,
        path: &ThreadPath,
        mut cursor: Cursor,
        model: &ModelSimState,
    ) -> Result<Settled, SettleError> {
        // Each move enters a child, leaves a node for its parent's
        // continuation or passes a `repeat`'s guard. A `repeat` body takes a
        // step on every cycle (PS-6), so a settling passes each control at
        // most twice per `repeat` it re-enters, and every `repeat` it
        // re-enters it re-enters once.
        let limit = self
            .program
            .controls
            .len()
            .saturating_add(1)
            .saturating_mul(6);
        for _ in 0..limit {
            cursor = match cursor {
                Cursor::Enter(node) => match self.program.control(node)?.step {
                    ControlStep::Sequence => match self.program.control(node)?.children.first() {
                        Some(first) => Cursor::Enter(*first),
                        None => Cursor::Exit(node),
                    },
                    ControlStep::Branch | ControlStep::Case => {
                        match self.program.control(node)?.children[..] {
                            [body] => Cursor::Enter(body),
                            _ => return Err(ProtocolRefusal::fault("a-label-has-one-body").into()),
                        }
                    }
                    ControlStep::Parallel => return Ok(Settled::Rest(Rest::Fork(node))),
                    ControlStep::Attempt
                    | ControlStep::Send
                    | ControlStep::Receive
                    | ControlStep::Event => return Ok(Settled::Rest(Rest::Node(node))),
                    ControlStep::Await => return Ok(Settled::Rest(Rest::Await(node))),
                    ControlStep::Check => {
                        let condition = self
                            .program
                            .checks
                            .get(&node)
                            .copied()
                            .ok_or_else(|| ProtocolRefusal::fault("a-check-has-a-condition"))?;
                        match self.decide(instance, path, node, condition, model)? {
                            Decision::Value(true) => Cursor::Exit(node),
                            Decision::Value(false) => return Ok(Settled::Rest(Rest::Check(node))),
                            Decision::Undecided(finding) => return Ok(Settled::Undecided(finding)),
                        }
                    }
                    ControlStep::Choice => {
                        let cases = self
                            .program
                            .choices
                            .get(&node)
                            .ok_or_else(|| ProtocolRefusal::fault("a-choice-has-cases"))?;
                        let mut entered = None;
                        for (case, guard) in cases {
                            match self.decide(instance, path, node, guard, model)? {
                                Decision::Value(true) => {
                                    if entered.replace(*case).is_some() {
                                        return Err(ProtocolRefusal::fault(
                                            "choice-guards-are-exclusive",
                                        )
                                        .into());
                                    }
                                }
                                Decision::Value(false) => {}
                                Decision::Undecided(finding) => {
                                    return Ok(Settled::Undecided(finding))
                                }
                            }
                        }
                        Cursor::Enter(entered.ok_or_else(|| {
                            ProtocolRefusal::fault("choice-guards-are-exhaustive")
                        })?)
                    }
                    ControlStep::Repeat => {
                        if self.program.repeat(node)?.max.is_some() {
                            instance.counters.insert((node, path.clone()), 0);
                        }
                        Cursor::Guard(node)
                    }
                    ControlStep::Unstepped(_) => {
                        return Err(ProtocolRefusal::fault("a-stepped-construct-is-admitted").into())
                    }
                },
                Cursor::Guard(node) => {
                    let repeat = self.program.repeat(node)?;
                    match self.decide(instance, path, node, repeat.guard, model)? {
                        Decision::Undecided(finding) => return Ok(Settled::Undecided(finding)),
                        Decision::Value(false) => Cursor::Exit(node),
                        Decision::Value(true) => match repeat.max {
                            None => Cursor::Enter(repeat.body),
                            Some(max) => {
                                let counter = instance
                                    .counters
                                    .get_mut(&(node, path.clone()))
                                    .ok_or_else(|| {
                                        ProtocolRefusal::fault("a-bounded-repeat-has-a-counter")
                                    })?;
                                if *counter < max {
                                    *counter = counter.saturating_add(1);
                                    Cursor::Enter(repeat.body)
                                } else {
                                    // With no `exhausted` control, a guard
                                    // true at the maximum leaves the repeat.
                                    repeat.exhausted.map_or(Cursor::Exit(node), Cursor::Enter)
                                }
                            }
                        },
                    }
                }
                Cursor::Exit(node) => {
                    if self.program.control(node)?.step == ControlStep::Repeat {
                        instance.counters.remove(&(node, path.clone()));
                    }
                    let Some(parent) = self.program.parent(node)? else {
                        return Ok(Settled::Rest(Rest::Finish));
                    };
                    let holder = self.program.control(parent)?;
                    match holder.step {
                        ControlStep::Sequence => {
                            let position = holder
                                .children
                                .iter()
                                .position(|child| *child == node)
                                .ok_or_else(|| ProtocolRefusal::fault("a-child-of-its-parent"))?;
                            match holder.children.get(position.saturating_add(1)) {
                                Some(next) => Cursor::Enter(*next),
                                None => Cursor::Exit(parent),
                            }
                        }
                        ControlStep::Branch => return Ok(Settled::Rest(Rest::End)),
                        ControlStep::Case | ControlStep::Choice => Cursor::Exit(parent),
                        ControlStep::Repeat => {
                            if self.program.repeat(parent)?.body == node {
                                Cursor::Guard(parent)
                            } else {
                                Cursor::Exit(parent)
                            }
                        }
                        ControlStep::Await => {
                            let awaited = self.program.awaited(parent)?;
                            if awaited.event == node {
                                Cursor::Enter(awaited.then)
                            } else {
                                Cursor::Exit(parent)
                            }
                        }
                        ControlStep::Parallel
                        | ControlStep::Attempt
                        | ControlStep::Check
                        | ControlStep::Send
                        | ControlStep::Receive
                        | ControlStep::Event
                        | ControlStep::Unstepped(_) => {
                            return Err(ProtocolRefusal::fault("a-parent-holds-controls").into())
                        }
                    }
                }
            };
        }
        Err(ProtocolRefusal::fault("settling-reaches-a-rest-point").into())
    }

    /// The value of the guard or condition `body` of `node`, read by the
    /// thread `path` over the binders it sees (FO-2).
    fn decide(
        &self,
        instance: &Instance,
        path: &ThreadPath,
        node: ProtocolNodeId,
        body: &CheckedBody,
        model: &ModelSimState,
    ) -> Result<Decision, ExpansionStop> {
        let frame = self.frame(instance, path, node)?;
        Ok(match self.evaluate(body, frame, model)? {
            Verdict::True => Decision::Value(true),
            Verdict::False => Decision::Value(false),
            Verdict::Undecided(outcome) => {
                Decision::Undecided(Box::new(undetermined(self.program.text(node)?, outcome)))
            }
        })
    }
}

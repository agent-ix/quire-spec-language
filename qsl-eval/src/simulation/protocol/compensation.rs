// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 CP-1 to CP-5, ST-11 and ST-12 (QSpec FR-427): compensation
//! registrations, their activation, compensation threads and their
//! `cattempt` and `cend` steps.
//!
//! Registration, activation and a commit's closure are folded into the step
//! that causes them (FO-1): the step of a template's forward `effect`
//! registers it; a later step that binds a record of a template's trigger
//! type activates each `registered` registration whose activation guard
//! holds over that record, creating its compensation thread; and a step of
//! a template's commit node closes its `registered` registrations and stops
//! later registrations and attempts. A compensation thread takes `cattempt`
//! (one FR-120 application of its template's operation, every clause of
//! the operation selected) up to the template's authored attempt count, and
//! ends with `cend` at its retry point, which records the recovery
//! predicate's value as the registration's status.
//!
//! A registration holds its ordinal while it is `registered` or
//! `activated` (CP-1's "live"); a closed or ended one stays in the key until
//! its instance finishes, or until a new registration of the same template
//! and thread takes its ordinal.

use qsl_semantics::check::{CheckedBody, ProtocolNodeId};
use qsl_semantics::family::FamilyOutcome;
use quire_exact::{Meter, Outcome, Value};

use super::identity::{ProtocolTransition, ThreadPath, WaitCause};
use super::memory::{Access, GatedStep, MemoryModel, Written};
use super::program::{Rest, Template};
use super::steps::Gathered;
use super::{
    undetermined, Bound, Compensating, Disposition, Instance, ProtocolRefusal, ProtocolState,
    ProtocolSystem, Registered, Registration, Status, Thread,
};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::model::encode::{reference_enc, value_enc};
use crate::simulation::model::{verdict, FirstApplication, ModelFinding, OperationRef, Verdict};
use crate::value::evaluate_protocol_slot;

/// A template expression's value, or the finding of an undecided one.
enum Decided {
    Value(Value),
    Undecided(Box<ModelFinding>),
}

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// The capture values of `registration`'s first `count` captures.
    fn captured(registration: &Registration, count: usize) -> Vec<Option<Value>> {
        registration
            .captures
            .iter()
            .take(count)
            .map(|bound| bound.value.clone())
            .collect()
    }

    /// One template expression's value over `frame`, at the model state of
    /// `next`; template expressions read no model state (FR-381).
    fn decide_template(
        &self,
        template: &Template<'_>,
        body: &CheckedBody,
        frame: Vec<Option<Value>>,
        next: &ProtocolState<M>,
    ) -> Result<Decided, ExpansionStop> {
        let observation = self.model.observation_of(&next.model)?;
        let evaluation = evaluate_protocol_slot(
            self.package,
            body,
            frame,
            &observation.environment,
            &mut Meter::new(self.model.meter()),
        )
        .map_err(|_| super::broken())?;
        if let FamilyOutcome::Evaluated(Outcome::Completed(value)) = &evaluation.outcome {
            return Ok(Decided::Value(value.clone()));
        }
        match verdict(&evaluation.outcome) {
            Verdict::Undecided(outcome) => Ok(Decided::Undecided(Box::new(undetermined(
                &template.name,
                outcome,
            )))),
            Verdict::True | Verdict::False => Err(super::broken()),
        }
    }

    /// A Boolean template expression's value: `None`, with the finding
    /// pushed, when it is undecided.
    fn holds(
        &self,
        template: &Template<'_>,
        body: &CheckedBody,
        frame: Vec<Option<Value>>,
        next: &ProtocolState<M>,
        findings: &mut Vec<ModelFinding>,
    ) -> Result<Option<bool>, ExpansionStop> {
        match self.decide_template(template, body, frame, next)? {
            Decided::Value(Value::Boolean(value)) => Ok(Some(value)),
            Decided::Value(_) => Err(super::broken()),
            Decided::Undecided(finding) => {
                findings.push(*finding);
                Ok(None)
            }
        }
    }

    /// FO-1's folded compensation effects of the step of the thread `path`
    /// at `node` of instance `ordinal`, applied to its post-state `next`
    /// after the step bound its binder, in this order: a commit node's
    /// closure (CP-4), so that a commit's own record activates nothing it
    /// closes; the activations its record causes (CP-2); and the
    /// registrations a forward effect makes (CP-1), which a step's own
    /// record does not activate, activation being by a later step. `false`,
    /// with the finding pushed, when an activation guard or a capture is
    /// undecided: the step is not taken.
    pub(super) fn fold(
        &self,
        next: &mut ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
        findings: &mut Vec<ModelFinding>,
    ) -> Result<bool, ExpansionStop> {
        if self.program.templates.is_empty() {
            return Ok(true);
        }
        let closes: Vec<usize> = self
            .program
            .templates
            .iter()
            .enumerate()
            .filter(|(_, template)| template.commit == Some(node))
            .map(|(index, _)| index)
            .collect();
        if !closes.is_empty() {
            let instance = Self::instance_mut(next, ordinal)?;
            instance.committed.insert(node);
            for (key, registration) in &mut instance.registrations {
                if closes.contains(&key.template) && registration.status == Status::Registered {
                    registration.status = Status::Closed;
                }
            }
        }
        let record = Self::instance(next, ordinal)?
            .entry(node, path)
            .and_then(|bound| bound.value.clone());
        if let (Some(record), Some(record_type)) = (&record, self.program.binder_type(node)) {
            for (index, template) in self.program.templates.iter().enumerate() {
                if template.trigger == *record_type
                    && !self.activate(next, ordinal, index, template, record, findings)?
                {
                    return Ok(false);
                }
            }
        }
        for (index, template) in self.program.templates.iter().enumerate() {
            if template.effect != node || Self::closed(Self::instance(next, ordinal)?, template) {
                continue;
            }
            let forward = record
                .clone()
                .ok_or_else(|| ProtocolRefusal::fault("an-effect-binds-its-record"))?;
            if !self.register(next, ordinal, path, index, forward, findings)? {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// CP-1: register `template` for the effect of the thread `path`, with
    /// the smallest ordinal no live registration of the template and thread
    /// holds, its registration captures evaluated over the forward record.
    fn register(
        &self,
        next: &mut ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        index: usize,
        forward: Value,
        findings: &mut Vec<ModelFinding>,
    ) -> Result<bool, ExpansionStop> {
        let template = self.program.template(index)?;
        let mut captures: Vec<Bound> = Vec::with_capacity(template.registration.len());
        for (_, initializer) in &template.registration {
            let mut frame = vec![Some(forward.clone()), Some(self.over.clone())];
            frame.extend(captures.iter().map(|bound| bound.value.clone()));
            match self.decide_template(template, initializer, frame, next)? {
                Decided::Value(value) => captures.push(Bound {
                    enc: value_enc(&value),
                    value: Some(value),
                }),
                Decided::Undecided(finding) => {
                    findings.push(*finding);
                    return Ok(false);
                }
            }
        }
        let instance = Self::instance_mut(next, ordinal)?;
        let free = (0u64..)
            .find(|candidate| {
                instance
                    .registrations
                    .get(&Registered {
                        template: index,
                        thread: path.clone(),
                        ordinal: *candidate,
                    })
                    .is_none_or(|registration| !registration.status.live())
            })
            .ok_or_else(|| ProtocolRefusal::fault("a-registration-ordinal-is-free"))?;
        instance.registrations.insert(
            Registered {
                template: index,
                thread: path.clone(),
                ordinal: free,
            },
            Registration {
                status: Status::Registered,
                captures,
            },
        );
        Ok(true)
    }

    /// CP-2: activate every `registered` registration of `template` whose
    /// activation guard holds over `trigger` and its registration captures,
    /// in ascending key order: its activation captures are evaluated, it
    /// becomes `activated`, and its compensation thread is created at its
    /// compensation attempt with the smallest ordinal no live compensation
    /// thread of the template holds (PS-2).
    fn activate(
        &self,
        next: &mut ProtocolState<M>,
        ordinal: u64,
        index: usize,
        template: &Template<'_>,
        trigger: &Value,
        findings: &mut Vec<ModelFinding>,
    ) -> Result<bool, ExpansionStop> {
        let waiting: Vec<(Registered, Registration)> = Self::instance(next, ordinal)?
            .registrations
            .iter()
            .filter(|(key, registration)| {
                key.template == index && registration.status == Status::Registered
            })
            .map(|(key, registration)| (key.clone(), registration.clone()))
            .collect();
        let count = template.registration.len();
        for (key, mut registration) in waiting {
            let mut frame = vec![Some(trigger.clone())];
            frame.extend(Self::captured(&registration, count));
            frame.push(Some(self.over.clone()));
            match self.holds(template, template.guard, frame, next, findings)? {
                None => return Ok(false),
                Some(false) => continue,
                Some(true) => {}
            }
            for (_, initializer) in &template.activation {
                let mut frame = vec![Some(trigger.clone()), Some(self.over.clone())];
                frame.extend(Self::captured(&registration, usize::MAX));
                match self.decide_template(template, initializer, frame, next)? {
                    Decided::Value(value) => registration.captures.push(Bound {
                        enc: value_enc(&value),
                        value: Some(value),
                    }),
                    Decided::Undecided(finding) => {
                        findings.push(*finding);
                        return Ok(false);
                    }
                }
            }
            registration.status = Status::Activated;
            let instance = Self::instance_mut(next, ordinal)?;
            let thread = Self::free_compensation(instance, ordinal, &template.name)?;
            instance.registrations.insert(key.clone(), registration);
            instance.threads.insert(
                thread.clone(),
                Thread {
                    rest: Rest::Compensate(index),
                    disposition: Disposition::Running,
                    owner: None,
                },
            );
            instance.compensating.insert(
                thread,
                Compensating {
                    registration: key,
                    count: 0,
                },
            );
        }
        Ok(true)
    }

    /// The path of the compensation thread of the template `name` that an
    /// activation creates in `instance` (PS-2): the smallest ordinal no live
    /// compensation thread of the template holds.
    pub(super) fn free_compensation(
        instance: &Instance,
        ordinal: u64,
        name: &str,
    ) -> Result<ThreadPath, ProtocolRefusal> {
        (0u64..)
            .map(|candidate| ThreadPath::compensation(ordinal, name, candidate))
            .find(|thread| !instance.threads.contains_key(thread))
            .ok_or_else(|| ProtocolRefusal::fault("a-compensation-ordinal-is-free"))
    }

    /// The template operation's application, as FR-120's application
    /// function takes it: the declaring type is the receiver type.
    fn compensation_operation<'t>(template: &'t Template<'_>) -> OperationRef<'t> {
        OperationRef {
            declaring: template.declaring,
            name: &template.operation,
            receiver: template.declaring,
        }
    }

    /// Whether a step of `template`'s commit node has occurred in
    /// `instance` (CP-4).
    fn closed(instance: &Instance, template: &Template<'_>) -> bool {
        template
            .commit
            .is_some_and(|commit| instance.committed.contains(&commit))
    }

    /// ST-11 and ST-12: the steps of the compensation thread `path` of
    /// instance `ordinal` resting at `rest`.
    pub(super) fn compensation(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        rest: Rest,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        let (index, retry) = match rest {
            Rest::Compensate(index) => (index, false),
            Rest::Retry(index) => (index, true),
            Rest::Node(_)
            | Rest::Await(_)
            | Rest::Check(_)
            | Rest::Fork(_)
            | Rest::Join(_)
            | Rest::Finish
            | Rest::End => {
                return Err(
                    ProtocolRefusal::fault("a-compensation-thread-rests-at-its-template").into(),
                )
            }
        };
        let template = self.program.template(index)?;
        let instance = Self::instance(state, ordinal)?;
        let compensating = instance
            .compensating
            .get(path)
            .ok_or_else(|| ProtocolRefusal::fault("a-compensation-thread-has-its-registration"))?;
        let registration = instance
            .registrations
            .get(&compensating.registration)
            .ok_or_else(|| ProtocolRefusal::fault("a-compensation-thread-has-its-registration"))?;
        let key = self.registration_key(&compensating.registration)?;
        if retry {
            self.cend(state, ordinal, path, template, registration, gathered)?;
        }
        if compensating.count >= template.attempts || Self::closed(instance, template) {
            return Ok(());
        }
        if retry {
            // The relation reads no attempt record when it can be reached:
            // `Program::new` refuses one that does with more than one
            // attempt.
            let mut frame = vec![None, None];
            frame.extend(Self::captured(registration, usize::MAX));
            frame.push(Some(self.over.clone()));
            if self.holds(
                template,
                template.retry,
                frame,
                state,
                &mut gathered.findings,
            )? != Some(true)
            {
                return Ok(());
            }
        }
        if !self.gate(state, path, GatedStep::Attempt) {
            return Ok(());
        }
        let operation = Self::compensation_operation(template);
        let observed = self.memory.observe(&state.memory, path, &state.model);
        let applications =
            self.model
                .apply_operation(&observed, &operation, &|_| true, &mut gathered.budget)?;
        gathered.findings.extend(applications.findings);
        for applied in applications.successors {
            let identity = ProtocolTransition::cattempt(
                path.clone(),
                key.clone(),
                applied.transition,
                reference_enc(&applied.receiver),
                applied.arguments.iter().map(value_enc).collect(),
            );
            // A compensation thread is no branch thread of any `parallel`,
            // so no resolved memory model governs it (ADR-025 MM-3): its
            // attempt is local, as an attempt outside every weak `parallel`
            // is (FR-220).
            let (model, memory) = match self.memory.write(
                &state.memory,
                path,
                &Access::local(),
                &state.model,
                applied.state,
            ) {
                Written::Applied(model, memory) => (model, memory),
                Written::Disabled => continue,
                Written::Cut => {
                    gathered.cut = true;
                    continue;
                }
            };
            let mut next = state.clone();
            next.model = model;
            next.memory = memory;
            let target = Self::instance_mut(&mut next, ordinal)?;
            if let Some(thread) = target.threads.get_mut(path) {
                thread.rest = Rest::Retry(index);
            }
            if let Some(entry) = target.compensating.get_mut(path) {
                entry.count = entry.count.saturating_add(1);
            }
            self.normalize(&mut next, ordinal)?;
            gathered.successors.push((identity, next));
        }
        Ok(())
    }

    /// ST-12: the thread ends and is removed; the registration's status
    /// records the recovery predicate's value over its captures.
    fn cend(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        template: &Template<'_>,
        registration: &Registration,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        // The predicate reads no recovery view: `Program::new` refuses one
        // that does.
        let mut frame = vec![None];
        frame.extend(Self::captured(registration, usize::MAX));
        frame.push(Some(self.over.clone()));
        let Some(recovered) = self.holds(
            template,
            template.recover,
            frame,
            state,
            &mut gathered.findings,
        )?
        else {
            return Ok(());
        };
        let mut next = state.clone();
        let target = Self::instance_mut(&mut next, ordinal)?;
        target.threads.remove(path);
        let compensating = target
            .compensating
            .remove(path)
            .ok_or_else(|| ProtocolRefusal::fault("a-compensation-thread-has-its-registration"))?;
        let key = self.registration_key(&compensating.registration)?;
        if let Some(entry) = target.registrations.get_mut(&compensating.registration) {
            entry.status = if recovered {
                Status::Recovered
            } else {
                Status::Unrecovered
            };
        }
        self.normalize(&mut next, ordinal)?;
        gathered
            .successors
            .push((ProtocolTransition::cend(path.clone(), key), next));
        Ok(())
    }

    /// PD-4 for a compensation thread at its compensation attempt with no
    /// enabled step: `CommitClosed` after its template's commit node has
    /// occurred, else as an attempt, `Precondition`, `NoPostState` or
    /// `Constraint`; `Gate` when the gate alone disables it.
    pub(super) fn compensation_cause(
        &self,
        state: &ProtocolState<M>,
        instance: &Instance,
        path: &ThreadPath,
        index: usize,
    ) -> Result<WaitCause, ExpansionStop> {
        let template = self.program.template(index)?;
        if let Some(cause) = self.no_role_instance(instance, template.role) {
            return Ok(cause);
        }
        if Self::closed(instance, template) {
            return Ok(WaitCause::CommitClosed {
                template: template.name.clone(),
            });
        }
        if !self.gate(state, path, GatedStep::Attempt) {
            return Ok(WaitCause::Gate);
        }
        let operation = Self::compensation_operation(template);
        let observed = self.memory.observe(&state.memory, path, &state.model);
        Ok(
            match self
                .model
                .first_application(&observed, &operation, &|_| true)?
            {
                FirstApplication::Precondition(clause) => WaitCause::Precondition { clause },
                FirstApplication::None | FirstApplication::Enabled(_) => WaitCause::NoPostState,
            },
        )
    }
}

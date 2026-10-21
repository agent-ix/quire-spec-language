// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 ST-13 to ST-15, AE-1 to AE-4, RR-1 to RR-4 (QSpec FR-428):
//! `activate`, `spawn` and `retire`, the steps `by` a replicated role, and
//! the retirement a lifetime folds into the step that ends it.
//!
//! A replicated role keeps, in each protocol instance, the status `live` or
//! `retired` of every object of its population it has spawned. A node owned
//! by the role is taken by each live instance in turn (RR-2): the node's
//! steps are enumerated once, and each is given to every live instance as its
//! actor. `workflow` lifetimes end with the instance that holds them (AE-3);
//! a `scope` lifetime ends in the step after which no running thread can
//! reach a node the role owns ([`Program::reaches_any`]); an `until(p)`
//! lifetime ends in a `retire` step.
//!
//! Under `activation on each`, the environment starts one protocol instance
//! per trigger record that satisfies the activation guard (ST-13), with the
//! smallest ordinal no live instance holds (AE-2). At `max_live_instances`
//! live instances the `activate` steps stay unexpanded and the state is
//! instance-limited (AE-4).

use quire_exact::{ObjectReference, Value};

use super::identity::{Actor, ProtocolTransition, StepKind, ThreadPath, WaitCause};
use super::memory::MemoryModel;
use super::program::{Cursor, Lifetime};
use super::settle::{SettleError, Settled};
use super::steps::Gathered;
use super::{
    undetermined, Bound, Disposition, Instance, ProtocolRefusal, ProtocolState, ProtocolSystem,
    RoleStatus, Thread,
};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::model::encode::value_enc;
use crate::simulation::model::{ModelFinding, ModelSimState, Verdict};

/// A protocol instance started, or the finding of a guard that stopped it.
pub(super) enum Started {
    /// The instance, its root thread settled from the start of `run`.
    Instance(Box<Instance>),
    /// A guard or `check` the evaluator does not decide.
    Undecided(Box<ModelFinding>),
}

/// Who takes the steps of a node (ADR-027 RR-2).
pub(super) enum Acting {
    /// A node no replicated role owns.
    Unowned,
    /// A node a replicated role owns: one step per live instance.
    By {
        role: String,
        actors: Vec<ObjectReference>,
    },
}

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// ADR-027 FO-3, ST-13: protocol instance `ordinal` with its root thread
    /// settled from the start of `run`, started by `trigger` under
    /// `activation on each`.
    pub(super) fn start(
        &self,
        ordinal: u64,
        trigger: Option<Bound>,
        model: &ModelSimState,
    ) -> Result<Started, SettleError> {
        let root = ThreadPath::root(ordinal);
        let mut instance = Instance {
            trigger,
            ..Instance::default()
        };
        let rest = match self.settle(
            &mut instance,
            &root,
            Cursor::Enter(self.program.root),
            model,
        )? {
            Settled::Rest(rest) => rest,
            Settled::Undecided(finding) => return Ok(Started::Undecided(finding)),
        };
        instance.threads.insert(
            root,
            Thread {
                rest,
                disposition: Disposition::Running,
                owner: None,
            },
        );
        Ok(Started::Instance(Box::new(instance)))
    }

    /// Who takes the steps of a node owned by the replicated role `owner`.
    fn acting(&self, instance: &Instance, owner: Option<usize>) -> Result<Acting, ProtocolRefusal> {
        let Some(index) = owner else {
            return Ok(Acting::Unowned);
        };
        let role = self
            .program
            .replicated
            .get(index)
            .ok_or_else(|| ProtocolRefusal::fault("an-owner-is-a-replicated-role"))?;
        let actors = instance
            .roles
            .get(&index)
            .into_iter()
            .flatten()
            .filter(|(_, status)| **status == RoleStatus::Live)
            .map(|(object, _)| object.clone())
            .collect();
        Ok(Acting::By {
            role: role.name.clone(),
            actors,
        })
    }

    /// The steps `step` gathers for a node owned by the replicated role
    /// `owner`, each given to every live instance of the role as its actor
    /// (ADR-027 RR-2). With `only`, only the steps of that kind are the
    /// role's; the others are kept as they are.
    pub(super) fn by_owner(
        &self,
        instance: &Instance,
        owner: Option<usize>,
        only: Option<StepKind>,
        gathered: &mut Gathered<M>,
        step: impl FnOnce(&mut Gathered<M>) -> Result<(), ExpansionStop>,
    ) -> Result<(), ExpansionStop> {
        let acting = self.acting(instance, owner)?;
        let from = gathered.successors.len();
        if matches!(&acting, Acting::By { actors, .. } if actors.is_empty()) && only.is_none() {
            return Ok(());
        }
        step(gathered)?;
        let Acting::By { role, actors } = acting else {
            return Ok(());
        };
        let fresh: Vec<_> = gathered.successors.drain(from..).collect();
        for (transition, next) in fresh {
            if only.is_some_and(|kind| transition.kind() != kind) {
                gathered.successors.push((transition, next));
                continue;
            }
            for object in &actors {
                gathered.successors.push((
                    transition.clone().acting(Actor {
                        role: role.clone(),
                        object: object.clone(),
                    }),
                    next.clone(),
                ));
            }
        }
        Ok(())
    }

    /// Whether a `spawn` of the replicated role `index` is enabled in
    /// `instance` (ST-14): fewer than the role's `max` instances are live,
    /// and some object of its population has not been spawned.
    pub(super) fn spawn_possible(&self, instance: &Instance, index: usize) -> bool {
        let Some(role) = self.program.replicated.get(index) else {
            return false;
        };
        let known = instance.roles.get(&index);
        let live = known.map_or(0, |known| {
            known
                .values()
                .filter(|status| **status == RoleStatus::Live)
                .count()
        });
        u64::try_from(live).is_ok_and(|live| live < role.max)
            && self
                .universes
                .get(index)
                .into_iter()
                .flatten()
                .any(|object| known.is_none_or(|known| !known.contains_key(object)))
    }

    /// ADR-027 PD-4: `NoRoleInstance` for a node owned by the replicated
    /// role `owner` when the role has no live instance in `instance` and no
    /// `spawn` is enabled.
    pub(super) fn no_role_instance(
        &self,
        instance: &Instance,
        owner: Option<usize>,
    ) -> Option<WaitCause> {
        let index = owner?;
        let role = self.program.replicated.get(index)?;
        let live = instance
            .roles
            .get(&index)
            .is_some_and(|known| known.values().any(|status| *status == RoleStatus::Live));
        (!live && !self.spawn_possible(instance, index)).then(|| WaitCause::NoRoleInstance {
            role: role.name.clone(),
        })
    }

    /// ST-14 and ST-15: every `spawn` and `retire` step of the protocol
    /// instance `ordinal`. An `until(p)` predicate that evaluates undecided
    /// pushes its finding and enables no `retire`.
    pub(super) fn role_steps(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        instance: &Instance,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        for (index, role) in self.program.replicated.iter().enumerate() {
            let known = instance.roles.get(&index);
            if self.spawn_possible(instance, index) {
                for object in self.universes.get(index).into_iter().flatten() {
                    if known.is_some_and(|known| known.contains_key(object)) {
                        continue;
                    }
                    let mut next = state.clone();
                    Self::instance_mut(&mut next, ordinal)?
                        .roles
                        .entry(index)
                        .or_default()
                        .insert(object.clone(), RoleStatus::Live);
                    self.normalize(&mut next, ordinal)?;
                    gathered.successors.push((
                        ProtocolTransition::role(
                            StepKind::Spawn,
                            ordinal,
                            role.name.clone(),
                            object.clone(),
                        ),
                        next,
                    ));
                }
            }
            let Lifetime::Until(predicate) = role.lifetime else {
                continue;
            };
            for (object, status) in known.into_iter().flatten() {
                if *status != RoleStatus::Live {
                    continue;
                }
                let mut frame = vec![Some(self.over.clone())];
                if self.program.activation_slot.is_some() {
                    frame.push(
                        instance
                            .trigger
                            .as_ref()
                            .and_then(|bound| bound.value.clone()),
                    );
                }
                frame.push(Some(Value::Reference(object.clone())));
                match self.evaluate(predicate, frame, &state.model)? {
                    Verdict::True => {}
                    Verdict::False => continue,
                    Verdict::Undecided(outcome) => {
                        gathered.findings.push(undetermined(&role.name, outcome));
                        continue;
                    }
                }
                let mut next = state.clone();
                Self::instance_mut(&mut next, ordinal)?
                    .roles
                    .entry(index)
                    .or_default()
                    .insert(object.clone(), RoleStatus::Retired);
                self.normalize(&mut next, ordinal)?;
                gathered.successors.push((
                    ProtocolTransition::role(
                        StepKind::Retire,
                        ordinal,
                        role.name.clone(),
                        object.clone(),
                    ),
                    next,
                ));
            }
        }
        Ok(())
    }

    /// ADR-027 RR-4: each `scope` role retires its live instances once no
    /// running thread of `instance` can reach a node the role owns.
    pub(super) fn retire_scoped(&self, instance: &mut Instance) {
        for (index, role) in self.program.replicated.iter().enumerate() {
            if !matches!(role.lifetime, Lifetime::Scope) {
                continue;
            }
            let reachable = instance.threads.values().any(|thread| {
                thread.disposition == Disposition::Running
                    && self.program.reaches_any(thread.rest, &role.owns)
            });
            if reachable {
                continue;
            }
            for status in instance.roles.entry(index).or_default().values_mut() {
                *status = RoleStatus::Retired;
            }
        }
    }

    /// ST-13: the `activate` steps of `state`, one per record of the trigger
    /// type that satisfies the activation guard, starting an instance with
    /// the smallest ordinal no live instance holds (AE-2). At the instance
    /// bound none is expanded (AE-4). Whether the state is instance-limited:
    /// at the bound, with some `activate` enabled.
    pub(super) fn activate_steps(
        &self,
        state: &ProtocolState<M>,
        gathered: &mut Gathered<M>,
    ) -> Result<bool, ExpansionStop> {
        let Some(activation) = &self.program.activation else {
            return Ok(false);
        };
        let at_bound = u64::try_from(state.instances.len())
            .map_or(true, |live| live >= self.max_live_instances);
        let ordinal = (0u64..)
            .find(|candidate| !state.instances.contains_key(candidate))
            .ok_or_else(|| ProtocolRefusal::fault("an-instance-ordinal-is-free"))?;
        let records = self
            .model
            .values_of(&activation.record_type, &state.model, &mut gathered.budget)?
            .ok_or_else(|| ProtocolRefusal::fault("an-activation-type-is-bounded"))?;
        let mut enabled = false;
        for record in records {
            if let Some(guard) = activation.guard {
                let frame = vec![Some(record.clone()), Some(self.over.clone())];
                match self.evaluate(guard, frame, &state.model)? {
                    Verdict::True => {}
                    Verdict::False => continue,
                    Verdict::Undecided(outcome) => {
                        gathered
                            .findings
                            .push(undetermined(&self.protocol.name, outcome));
                        continue;
                    }
                }
            }
            enabled = true;
            if at_bound {
                continue;
            }
            let enc = value_enc(&record);
            let trigger = Bound {
                value: Some(record),
                enc: enc.clone(),
            };
            match self.start(ordinal, Some(trigger), &state.model)? {
                Started::Instance(started) => {
                    let mut next = state.clone();
                    next.instances.insert(ordinal, *started);
                    self.normalize(&mut next, ordinal)?;
                    gathered
                        .successors
                        .push((ProtocolTransition::activate(enc), next));
                }
                Started::Undecided(finding) => gathered.findings.push(*finding),
            }
        }
        Ok(enabled && at_bound)
    }

    /// ADR-027 AE-4: whether `state` is instance-limited, with
    /// `max_live_instances` live instances and an `activate` step enabled.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the guard's evaluation stops.
    pub fn instance_limited(&self, state: &ProtocolState<M>) -> Result<bool, ExpansionStop> {
        let mut scratch = Gathered {
            budget: self.model.budget(),
            successors: Vec::new(),
            findings: Vec::new(),
            cut: false,
        };
        self.activate_steps(state, &mut scratch)
    }
}

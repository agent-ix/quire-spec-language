// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 §2.2 ST-1, ST-6, ST-7, ST-9 and ST-10: the steps of a protocol
//! state, each applied atomically and followed by FO-1's settling, and
//! PD-4's blocked threads of a state none of whose steps is enabled.
//!
//! After every step the instance it moved is normalised: a parallel
//! instance's dispositions follow its branch threads; a thread that is
//! neither `running` nor unread by its instance's join leaves the state; a
//! parallel instance whose join has settled and none of whose branch threads
//! is `running` leaves it (PS-3, PS-4); and a binder no running thread that
//! sees it can still reach a reader of leaves it (PS-5). The key is then a function of
//! what the protocol can still do (PS-8).

use std::collections::BTreeSet;

use qsl_semantics::check::{CheckedJoinPolicy, CheckedOutstanding, ProtocolNodeId};

use super::identity::{BlockedThread, ProtocolTransition, ThreadPath, WaitCause};
use super::memory::{GatedStep, MemoryModel};
use super::program::{Cursor, Rest};
use super::{
    undetermined, Bound, Disposition, Instance, Parallel, ParallelKey, ProtocolRefusal,
    ProtocolState, ProtocolSystem, Thread,
};
use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::model::encode::{reference_enc, value_enc};
use crate::simulation::model::{Budget, FirstApplication, ModelFinding, OperationRef, Verdict};

/// One successor of a protocol state.
type Successor<M> = (ProtocolTransition, ProtocolState<M>);

/// The JCS bytes of `value` under the identity limits.
fn jcs(value: &impl quire_canonical::Encode) -> Result<Vec<u8>, ProtocolRefusal> {
    crate::simulation::key::canonical_bytes(value, super::identity_limits())
        .map_err(|_| ProtocolRefusal::fault("a-protocol-identity-encodes"))
}

/// Whether `policy` holds over `completed` of the joined labels (QSpec
/// FR-171): `all`, every one; `any`, one; `quorum(n)`, `n`.
fn policy_holds(policy: &CheckedJoinPolicy, joined: usize, completed: usize) -> bool {
    match policy {
        CheckedJoinPolicy::All => completed == joined,
        CheckedJoinPolicy::Any => completed >= 1,
        CheckedJoinPolicy::Quorum(count) => {
            u64::try_from(completed).is_ok_and(|done| done >= *count)
        }
        // Refused before any state is built.
        CheckedJoinPolicy::Predicate => false,
    }
}

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// Every enabled step of `state` with its successor, in ascending JCS
    /// bytes of its identity (FR-215 "The system"), and the state's
    /// findings: FR-120's invariant findings of its model state, then each
    /// running thread's contract and protocol-expression findings, threads
    /// in ascending JCS bytes of their paths (ADR-027 TS-4).
    pub(super) fn expand(
        &self,
        state: &ProtocolState<M>,
    ) -> Result<Expansion<ProtocolTransition, ProtocolState<M>, ModelFinding>, ExpansionStop> {
        let mut findings = self.model.invariant_findings(&state.model)?;
        let mut successors = Vec::new();
        let mut budget = self.model.budget();
        let mut running = Vec::new();
        for (ordinal, instance) in &state.instances {
            for (path, thread) in &instance.threads {
                if thread.disposition == Disposition::Running {
                    running.push((jcs(path)?, *ordinal, instance, path, thread));
                }
            }
        }
        running.sort_by(|left, right| left.0.cmp(&right.0));
        for (_, ordinal, instance, path, thread) in running {
            match thread.rest {
                Rest::Fork(node) => {
                    if self.gate(state, path, GatedStep::Fork) {
                        successors.push(self.fork(state, ordinal, path, node)?);
                    }
                }
                Rest::Join(node) => {
                    if self.join_enabled(instance, path, node)?
                        && self.gate(state, path, GatedStep::Join)
                    {
                        successors.push(self.join(state, ordinal, path, node)?);
                    }
                }
                Rest::Node(node) => {
                    if self.gate(state, path, GatedStep::Attempt) {
                        self.attempt(
                            state,
                            ordinal,
                            path,
                            node,
                            &mut budget,
                            &mut successors,
                            &mut findings,
                        )?;
                    }
                }
                Rest::Finish => {
                    self.finish(
                        state,
                        ordinal,
                        path,
                        &mut budget,
                        &mut successors,
                        &mut findings,
                    )?;
                }
                Rest::End => {}
            }
        }
        for step in self.memory.internal_steps(&state.memory, &state.model) {
            let mut next = state.clone();
            next.model = step.model;
            next.memory = step.component;
            successors.push((ProtocolTransition::memory(step.identity), next));
        }
        let mut keyed = Vec::with_capacity(successors.len());
        for (transition, next) in successors {
            keyed.push((jcs(&transition)?, transition, next));
        }
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(Expansion {
            successors: keyed
                .into_iter()
                .map(|(_, transition, next)| (transition, next))
                .collect(),
            findings,
        })
    }

    fn gate(&self, state: &ProtocolState<M>, path: &ThreadPath, step: GatedStep) -> bool {
        self.memory.gate(&state.memory, path, step)
    }

    fn instance_mut(
        state: &mut ProtocolState<M>,
        ordinal: u64,
    ) -> Result<&mut Instance, ProtocolRefusal> {
        state
            .instances
            .get_mut(&ordinal)
            .ok_or_else(|| ProtocolRefusal::fault("a-stepped-instance-is-live"))
    }

    /// ST-6: `path` forks the parallel instance of `node` with the smallest
    /// ordinal no live sibling holds (PS-2), one `running` branch thread per
    /// branch, each settled from its body's start; the owner rests at the
    /// join.
    fn fork(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
    ) -> Result<Successor<M>, ProtocolRefusal> {
        let join = self
            .program
            .joins
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("a-fork-has-a-join"))?;
        let text = self.program.text(node)?.to_owned();
        let mut next = state.clone();
        let instance = Self::instance_mut(&mut next, ordinal)?;
        let free = (0u64..)
            .find(|candidate| {
                !instance.parallels.contains_key(&ParallelKey {
                    parent: path.clone(),
                    node,
                    ordinal: *candidate,
                })
            })
            .ok_or_else(|| ProtocolRefusal::fault("an-ordinal-is-free"))?;
        let mut dispositions = std::collections::BTreeMap::new();
        let mut branches = Vec::with_capacity(join.branches.len());
        for (label, body) in &join.branches {
            let rest = self.program.settle(Cursor::Enter(*body))?;
            let disposition = if rest == Rest::End {
                Disposition::Completed
            } else {
                Disposition::Running
            };
            let child = path.child(&text, label, free);
            let owner = ParallelKey {
                parent: path.clone(),
                node,
                ordinal: free,
            };
            instance.threads.insert(
                child.clone(),
                Thread {
                    rest,
                    disposition,
                    owner: Some((owner, label.clone())),
                },
            );
            dispositions.insert(label.clone(), disposition);
            branches.push(child);
        }
        instance.parallels.insert(
            ParallelKey {
                parent: path.clone(),
                node,
                ordinal: free,
            },
            Parallel {
                settled: false,
                dispositions,
            },
        );
        if let Some(owner) = instance.threads.get_mut(path) {
            owner.rest = Rest::Join(node);
        }
        next.memory = self.memory.split(&next.memory, path, &branches);
        self.normalize(&mut next, ordinal)?;
        Ok((ProtocolTransition::fork(path.clone(), text), next))
    }

    /// The parallel instance `path` waits at the join of `node` for: the one
    /// it forked and has not yet joined.
    fn joined_instance<'i>(
        instance: &'i Instance,
        path: &ThreadPath,
        node: ProtocolNodeId,
    ) -> Option<(&'i ParallelKey, &'i Parallel)> {
        instance
            .parallels
            .iter()
            .find(|(key, parallel)| key.parent == *path && key.node == node && !parallel.settled)
    }

    /// ST-7's enabledness: the join policy holds over the dispositions of
    /// the labels it awaits, a branch counting once `completed`.
    fn join_enabled(
        &self,
        instance: &Instance,
        path: &ThreadPath,
        node: ProtocolNodeId,
    ) -> Result<bool, ProtocolRefusal> {
        let join = self
            .program
            .joins
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("a-join-is-a-parallel"))?;
        let Some((_, parallel)) = Self::joined_instance(instance, path, node) else {
            return Err(ProtocolRefusal::fault("a-waiting-join-has-its-instance"));
        };
        let completed = join
            .joined
            .iter()
            .filter(|label| parallel.dispositions.get(*label) == Some(&Disposition::Completed))
            .count();
        Ok(policy_holds(&join.policy, join.joined.len(), completed))
    }

    /// ST-7: the instance is settled; under `outstanding cancel` every
    /// branch thread still `running` is `cancelled`, and so is every thread
    /// it forked, at any depth, whose parallel instances are settled with
    /// it; the owner settles past the join.
    ///
    /// QSpec FR-426's `join` row names "every branch thread still running"
    /// and is silent on the threads such a branch forked. A cancelled
    /// thread takes no further step, so nothing it started may either: an
    /// inner branch that kept stepping would write the model after its
    /// ancestor was cancelled, and the inner instance, whose owner leaves
    /// the state, would never be joined.
    fn join(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
    ) -> Result<Successor<M>, ProtocolRefusal> {
        let join = self
            .program
            .joins
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("a-join-is-a-parallel"))?;
        let text = self.program.text(node)?.to_owned();
        let mut next = state.clone();
        let instance = Self::instance_mut(&mut next, ordinal)?;
        let key = Self::joined_instance(instance, path, node)
            .map(|(key, _)| key.clone())
            .ok_or_else(|| ProtocolRefusal::fault("a-waiting-join-has-its-instance"))?;
        let branches: Vec<ThreadPath> = join
            .branches
            .iter()
            .map(|(label, _)| path.child(&text, label, key.ordinal))
            .collect();
        if join.outstanding == Some(CheckedOutstanding::Cancel) {
            let cancelled: Vec<&ThreadPath> = branches
                .iter()
                .filter(|branch| {
                    instance
                        .threads
                        .get(*branch)
                        .is_some_and(|thread| thread.disposition == Disposition::Running)
                })
                .collect();
            let within =
                |thread: &ThreadPath| cancelled.iter().any(|branch| branch.is_prefix_of(thread));
            for (thread_path, thread) in &mut instance.threads {
                if within(thread_path) && thread.disposition == Disposition::Running {
                    thread.disposition = Disposition::Cancelled;
                }
            }
            for (inner, parallel) in &mut instance.parallels {
                if within(&inner.parent) {
                    parallel.settled = true;
                }
            }
        }
        if let Some(parallel) = instance.parallels.get_mut(&key) {
            parallel.settled = true;
        }
        let rest = self.program.settle(Cursor::Exit(node))?;
        if let Some(owner) = instance.threads.get_mut(path) {
            owner.rest = rest;
            if rest == Rest::End {
                owner.disposition = Disposition::Completed;
            }
        }
        next.memory = self.memory.merge(&next.memory, path, &branches);
        self.normalize(&mut next, ordinal)?;
        Ok((ProtocolTransition::join(path.clone(), text), next))
    }

    /// The attempt `node`'s operation, as FR-120's application function
    /// takes it.
    fn operation(&self, node: ProtocolNodeId) -> Result<OperationRef<'_>, ProtocolRefusal> {
        let attempt = self
            .program
            .attempts
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("an-attempt-node-is-an-attempt"))?;
        Ok(OperationRef {
            declaring: attempt.declaring,
            name: &attempt.operation,
            receiver: attempt.receiver,
        })
    }

    /// ST-1: one step per FR-120 application of `node`'s operation (every
    /// receiver and argument vector) and post-state, over the clauses its
    /// `contracts` select and the thread's observation, whose binder
    /// constraint holds; the delta is written through the memory model.
    #[allow(
        clippy::too_many_arguments,
        reason = "one expansion's state, owner and accumulators"
    )]
    fn attempt(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
        budget: &mut Budget,
        successors: &mut Vec<Successor<M>>,
        findings: &mut Vec<ModelFinding>,
    ) -> Result<(), ExpansionStop> {
        let attempt = self
            .program
            .attempts
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("an-attempt-node-is-an-attempt"))?;
        let text = self.program.text(node)?.to_owned();
        let operation = self.operation(node)?;
        let observed = self.memory.observe(&state.memory, path, &state.model);
        let applications = self.model.apply_operation(
            &observed,
            &operation,
            &|clause| attempt.contracts.contains(&clause),
            budget,
        )?;
        findings.extend(applications.findings);
        let instance = state
            .instances
            .get(&ordinal)
            .ok_or_else(|| ProtocolRefusal::fault("a-stepped-instance-is-live"))?;
        for applied in applications.successors {
            let bound = attempt
                .binds
                .and(applied.result.as_ref())
                .map(|result| Bound {
                    value: result.clone(),
                    enc: value_enc(result),
                });
            if let Some(constraint) = attempt.constraint {
                let mut frame = self.frame(instance, path)?;
                if let (Some(slot), Some(bound)) = (self.program.binder_of.get(&node), &bound) {
                    if let Some(entry) = frame.get_mut(*slot) {
                        *entry = Some(bound.value.clone());
                    }
                }
                match self.evaluate(constraint, frame, &observed)? {
                    Verdict::True => {}
                    Verdict::False => continue,
                    Verdict::Undecided(outcome) => {
                        findings.push(undetermined(&text, outcome));
                        continue;
                    }
                }
            }
            let identity = ProtocolTransition::attempt(
                path.clone(),
                text.clone(),
                applied.transition,
                reference_enc(&applied.receiver),
                applied.arguments.iter().map(value_enc).collect(),
            );
            let mut next = state.clone();
            let (model, memory) =
                self.memory
                    .write(&state.memory, path, &state.model, applied.state);
            next.model = model;
            next.memory = memory;
            let rest = self.program.settle(Cursor::Exit(node))?;
            let target = Self::instance_mut(&mut next, ordinal)?;
            if let Some(bound) = bound {
                target.binders.insert((node, path.clone()), bound);
            }
            if let Some(thread) = target.threads.get_mut(path) {
                thread.rest = rest;
                if rest == Rest::End {
                    thread.disposition = Disposition::Completed;
                }
            }
            self.normalize(&mut next, ordinal)?;
            successors.push((identity, next));
        }
        Ok(())
    }

    /// Every thread of the instance but `path` has left `running` (ST-9).
    fn others_settled(instance: &Instance, path: &ThreadPath) -> bool {
        instance
            .threads
            .iter()
            .all(|(other, thread)| other == path || thread.disposition != Disposition::Running)
    }

    /// ST-9: once every other thread of the instance has left `running`,
    /// one step per record of the finish type that satisfies its
    /// constraint; the binder is bound and the instance finished, which
    /// removes it (AE-3).
    fn finish(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        budget: &mut Budget,
        successors: &mut Vec<Successor<M>>,
        findings: &mut Vec<ModelFinding>,
    ) -> Result<(), ExpansionStop> {
        let instance = state
            .instances
            .get(&ordinal)
            .ok_or_else(|| ProtocolRefusal::fault("a-stepped-instance-is-live"))?;
        if !Self::others_settled(instance, path) {
            return Ok(());
        }
        let finish = &self.program.finish;
        let (slot, record_type) = &finish.binder;
        let records = self
            .model
            .values_of(record_type, &state.model, budget)?
            .ok_or_else(|| ProtocolRefusal::fault("a-finish-type-is-bounded"))?;
        for record in records {
            if let Some(constraint) = finish.constraint {
                let mut frame = self.frame(instance, path)?;
                if let Some(entry) = frame.get_mut(*slot) {
                    *entry = Some(record.clone());
                }
                match self.evaluate(constraint, frame, &state.model)? {
                    Verdict::True => {}
                    Verdict::False => continue,
                    Verdict::Undecided(outcome) => {
                        findings.push(undetermined(&finish.text, outcome));
                        continue;
                    }
                }
            }
            let mut next = state.clone();
            next.instances.remove(&ordinal);
            successors.push((
                ProtocolTransition::finish(path.clone(), finish.text.clone(), value_enc(&record)),
                next,
            ));
        }
        Ok(())
    }

    /// PS-3, PS-4, PS-5 after a step of instance `ordinal`.
    fn normalize(&self, state: &mut ProtocolState<M>, ordinal: u64) -> Result<(), ProtocolRefusal> {
        let Some(instance) = state.instances.get_mut(&ordinal) else {
            return Ok(());
        };
        // Each branch thread's disposition, recorded on its instance.
        let owners: Vec<(ThreadPath, ParallelKey, String, Disposition)> = instance
            .threads
            .iter()
            .filter_map(|(path, thread)| {
                let (key, label) = thread.owner.as_ref()?;
                Some((path.clone(), key.clone(), label.clone(), thread.disposition))
            })
            .collect();
        let mut leaving = BTreeSet::new();
        for (path, key, label, disposition) in &owners {
            let Some(parallel) = instance.parallels.get_mut(key) else {
                return Err(ProtocolRefusal::fault("a-branch-thread-has-its-instance"));
            };
            parallel.dispositions.insert(label.clone(), *disposition);
            if parallel.settled && *disposition != Disposition::Running {
                leaving.insert(path.clone());
            }
        }
        for path in &leaving {
            instance.threads.remove(path);
        }
        instance.parallels.retain(|_, parallel| {
            !parallel.settled
                || parallel
                    .dispositions
                    .values()
                    .any(|disposition| *disposition == Disposition::Running)
        });
        // A binder stays while some running thread that sees it can still
        // reach a node that reads it.
        let running: Vec<(&ThreadPath, Rest)> = instance
            .threads
            .iter()
            .filter(|(_, thread)| thread.disposition == Disposition::Running)
            .map(|(path, thread)| (path, thread.rest))
            .collect();
        instance.binders.retain(|(node, bound_by), _| {
            self.program.binder_of.get(node).is_some_and(|slot| {
                running.iter().any(|(reader, rest)| {
                    super::sees(reader, bound_by) && self.program.reads_from(*rest, *slot)
                })
            })
        });
        Ok(())
    }

    /// ADR-027 PD-4: every waiting thread of `state` with its rest point and
    /// wait cause, in ascending JCS bytes of its path. A `running` thread
    /// with no enabled step waits; a completed or cancelled one does not.
    pub(super) fn blocked_threads(
        &self,
        state: &ProtocolState<M>,
    ) -> Result<Vec<BlockedThread>, ExpansionStop> {
        let expansion = self.expand(state)?;
        let stepping: BTreeSet<&ThreadPath> = expansion
            .successors
            .iter()
            .filter_map(|(transition, _)| transition.thread())
            .collect();
        let mut blocked = Vec::new();
        for instance in state.instances.values() {
            for (path, thread) in &instance.threads {
                if thread.disposition != Disposition::Running || stepping.contains(path) {
                    continue;
                }
                let Some(cause) = self.wait_cause(state, instance, path, thread.rest)? else {
                    continue;
                };
                blocked.push(BlockedThread {
                    path: path.clone(),
                    rest_point: self.rest_point(thread.rest)?,
                    cause,
                });
            }
        }
        let mut keyed = Vec::with_capacity(blocked.len());
        for entry in blocked {
            keyed.push((jcs(&entry.path)?, entry));
        }
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(keyed.into_iter().map(|(_, entry)| entry).collect())
    }

    /// The running threads of `instance` among `threads`, in ascending JCS
    /// bytes of their paths.
    fn running_of(
        instance: &Instance,
        threads: impl IntoIterator<Item = ThreadPath>,
    ) -> Result<Vec<ThreadPath>, ProtocolRefusal> {
        let mut keyed = Vec::new();
        for path in threads {
            if instance
                .threads
                .get(&path)
                .is_some_and(|thread| thread.disposition == Disposition::Running)
            {
                keyed.push((jcs(&path)?, path));
            }
        }
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(keyed.into_iter().map(|(_, path)| path).collect())
    }

    /// Why the running thread `path` resting at `rest`, which owns no
    /// enabled step, waits; `None` at the end of its body.
    fn wait_cause(
        &self,
        state: &ProtocolState<M>,
        instance: &Instance,
        path: &ThreadPath,
        rest: Rest,
    ) -> Result<Option<WaitCause>, ExpansionStop> {
        Ok(Some(match rest {
            Rest::Fork(_) => WaitCause::Gate,
            Rest::Join(node) => {
                if !self.join_enabled(instance, path, node)? {
                    let join = self
                        .program
                        .joins
                        .get(&node)
                        .ok_or_else(|| ProtocolRefusal::fault("a-join-is-a-parallel"))?;
                    let text = self.program.text(node)?;
                    let key = Self::joined_instance(instance, path, node)
                        .map(|(key, _)| key.ordinal)
                        .ok_or_else(|| ProtocolRefusal::fault("a-waiting-join-has-its-instance"))?;
                    let needed = join.joined.iter().map(|label| path.child(text, label, key));
                    WaitCause::JoinWaiting {
                        threads: Self::running_of(instance, needed)?,
                    }
                } else {
                    WaitCause::Gate
                }
            }
            Rest::Node(node) => {
                if !self.gate(state, path, GatedStep::Attempt) {
                    WaitCause::Gate
                } else {
                    self.attempt_cause(state, path, node)?
                }
            }
            Rest::Finish => {
                if Self::others_settled(instance, path) {
                    WaitCause::Constraint
                } else {
                    let others = instance
                        .threads
                        .keys()
                        .filter(|other| *other != path)
                        .cloned();
                    WaitCause::JoinWaiting {
                        threads: Self::running_of(instance, others)?,
                    }
                }
            }
            Rest::End => return Ok(None),
        }))
    }

    /// PD-4 for a waiting attempt: the first false selected precondition of
    /// its first application, in canonical order; else `no-post-state` when
    /// that application has no post-state, `constraint` when its binder
    /// constraint holds for none.
    fn attempt_cause(
        &self,
        state: &ProtocolState<M>,
        path: &ThreadPath,
        node: ProtocolNodeId,
    ) -> Result<WaitCause, ExpansionStop> {
        let attempt = self
            .program
            .attempts
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("an-attempt-node-is-an-attempt"))?;
        let operation = self.operation(node)?;
        let observed = self.memory.observe(&state.memory, path, &state.model);
        let select = |clause| attempt.contracts.contains(&clause);
        Ok(
            match self
                .model
                .first_application(&observed, &operation, &select)?
            {
                FirstApplication::Precondition(clause) => WaitCause::Precondition { clause },
                FirstApplication::None => WaitCause::NoPostState,
                FirstApplication::Enabled(transition) => {
                    let mut budget = self.model.budget();
                    let applications =
                        self.model
                            .apply_operation(&observed, &operation, &select, &mut budget)?;
                    if applications
                        .successors
                        .iter()
                        .any(|applied| applied.transition == transition)
                    {
                        WaitCause::Constraint
                    } else {
                        WaitCause::NoPostState
                    }
                }
            },
        )
    }
}

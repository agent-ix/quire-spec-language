// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 §2.2 ST-1, ST-2, ST-6 to ST-10: the thread steps of a protocol
//! state other than the channel steps (`channel`), each applied atomically
//! and followed by FO-1's settling, and PD-4's blocked threads of a state
//! none of whose steps is enabled.
//!
//! After every step the instance it moved is normalised: a parallel
//! instance's dispositions follow its branch threads; a thread that is
//! neither `running` nor unread by its instance's join leaves the state; a
//! parallel instance whose join has settled and none of whose branch threads
//! is `running` leaves it (PS-3, PS-4); and a binder no running thread that
//! reads it can still reach a reader of leaves it (PS-5); and a `repeat`
//! counter of a thread that is no longer running leaves it (PS-6). The key
//! is then a function of what the protocol can still do (PS-8).

use std::collections::BTreeSet;

use qsl_semantics::check::{
    AccessClass, CheckedBody, CheckedJoinPolicy, CheckedOutstanding, ControlStep, ProtocolNodeId,
};
use quire_exact::Value;

use super::access::InstanceAccess;
use super::identity::{BlockedThread, ProtocolTransition, StepKind, ThreadPath, WaitCause};
use super::memory::{Access, GatedStep, MemoryModel, SharedCell, Written};
use super::program::{Cursor, Join, Rest};
use super::settle::Settled;
use super::{
    undetermined, Bound, Disposition, Instance, Parallel, ParallelKey, ProtocolRefusal,
    ProtocolState, ProtocolSystem, Thread,
};
use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::model::encode::{reference_enc, value_enc};
use crate::simulation::model::{
    Applied, Budget, FirstApplication, ModelFinding, ModelSimState, OperationRef, Verdict,
};

/// One successor of a protocol state.
pub(super) type Successor<M> = (ProtocolTransition, ProtocolState<M>);

/// One protocol state's expansion.
pub(super) type ProtocolExpansion<M> =
    Expansion<ProtocolTransition, ProtocolState<M>, ModelFinding>;

/// What one expansion gathers: its candidate budget, its successors and its
/// findings.
pub(super) struct Gathered<M: MemoryModel> {
    pub(super) budget: Budget,
    pub(super) successors: Vec<Successor<M>>,
    pub(super) findings: Vec<ModelFinding>,
    /// Whether a memory bound left a store unexpanded (ADR-025 MB-3).
    pub(super) cut: bool,
}

/// The thread that reads a node's slots, and the node.
#[derive(Clone, Copy)]
pub(super) struct Reader<'r> {
    pub(super) instance: &'r Instance,
    pub(super) path: &'r ThreadPath,
    pub(super) node: ProtocolNodeId,
}

/// The JCS bytes of `value` under the identity limits.
pub(super) fn jcs(value: &impl quire_canonical::Encode) -> Result<Vec<u8>, ProtocolRefusal> {
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
    ///
    /// Beside the expansion, whether a memory bound left a store of the
    /// state unexpanded, which makes the state bound-limited (ADR-025 MB-3).
    pub(super) fn expand_bounded(
        &self,
        state: &ProtocolState<M>,
    ) -> Result<(ProtocolExpansion<M>, bool), ExpansionStop> {
        let mut gathered = Gathered {
            budget: self.model.budget(),
            successors: Vec::new(),
            findings: self.model.invariant_findings(&state.model)?,
            cut: false,
        };
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
                        self.fork(state, ordinal, path, node, &mut gathered)?;
                    }
                }
                Rest::Join(node) => {
                    if self.join_enabled(instance, path, node)?
                        && self.gate(state, path, GatedStep::Join)
                    {
                        self.join(state, ordinal, path, node, &mut gathered)?;
                    }
                }
                Rest::Node(node) => {
                    self.event_node(
                        state,
                        ordinal,
                        path,
                        node,
                        Cursor::Exit(node),
                        &mut gathered,
                    )?;
                }
                Rest::Await(node) => {
                    let awaited = self.program.awaited(node)?;
                    let text = self.program.text(node)?.to_owned();
                    let mut next = state.clone();
                    if self.place(
                        &mut next,
                        ordinal,
                        path,
                        Cursor::Enter(awaited.timeout),
                        &mut gathered.findings,
                    )? {
                        self.normalize(&mut next, ordinal)?;
                        gathered.successors.push((
                            ProtocolTransition::control(StepKind::Timeout, path.clone(), text),
                            next,
                        ));
                    }
                    self.event_node(
                        state,
                        ordinal,
                        path,
                        awaited.event,
                        Cursor::Enter(awaited.then),
                        &mut gathered,
                    )?;
                }
                Rest::Finish => self.finish(state, ordinal, path, &mut gathered)?,
                Rest::Check(_) | Rest::End => {}
            }
        }
        self.channel_steps(state, &mut gathered)?;
        for step in self.memory.internal_steps(&state.memory, &state.model) {
            let mut next = state.clone();
            next.model = step.model;
            next.memory = step.component;
            gathered
                .successors
                .push((ProtocolTransition::memory(step.identity), next));
        }
        let mut keyed = Vec::with_capacity(gathered.successors.len());
        for (transition, next) in gathered.successors {
            keyed.push((jcs(&transition)?, transition, next));
        }
        keyed.sort_by(|left, right| left.0.cmp(&right.0));
        Ok((
            Expansion {
                successors: keyed
                    .into_iter()
                    .map(|(_, transition, next)| (transition, next))
                    .collect(),
                findings: gathered.findings,
            },
            gathered.cut,
        ))
    }

    /// The steps of the thread `path` at the event node `node`, by its kind,
    /// each settling the thread from `after`.
    fn event_node(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
        after: Cursor,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        match self.program.control(node)?.step {
            ControlStep::Attempt => {
                if self.gate(state, path, GatedStep::Attempt) {
                    self.attempt(state, ordinal, path, node, after, gathered)?;
                }
            }
            ControlStep::Send => {
                if self.gate(state, path, GatedStep::Send) {
                    self.send(state, ordinal, path, node, after, gathered)?;
                }
            }
            ControlStep::Receive => {
                if self.gate(state, path, GatedStep::Receive) {
                    self.receive(state, ordinal, path, node, after, gathered)?;
                }
            }
            ControlStep::Event => self.event(state, ordinal, path, node, after, gathered)?,
            ControlStep::Sequence
            | ControlStep::Branch
            | ControlStep::Parallel
            | ControlStep::Choice
            | ControlStep::Case
            | ControlStep::Repeat
            | ControlStep::Await
            | ControlStep::Check
            | ControlStep::Unstepped(_) => {
                return Err(ProtocolRefusal::fault("a-thread-rests-at-an-event-node").into())
            }
        }
        Ok(())
    }

    /// Settle the thread `path` of instance `ordinal` of `next` from
    /// `cursor` (FO-1), setting its rest point and, at the end of a branch
    /// body, its disposition. `false`, with the finding pushed, when a guard
    /// is undecided: the step is not taken.
    pub(super) fn place(
        &self,
        next: &mut ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        cursor: Cursor,
        findings: &mut Vec<ModelFinding>,
    ) -> Result<bool, ExpansionStop> {
        let ProtocolState {
            model, instances, ..
        } = next;
        let instance = instances
            .get_mut(&ordinal)
            .ok_or_else(|| ProtocolRefusal::fault("a-stepped-instance-is-live"))?;
        match self.settle(instance, path, cursor, model)? {
            Settled::Undecided(finding) => {
                findings.push(*finding);
                Ok(false)
            }
            Settled::Rest(rest) => {
                if let Some(thread) = instance.threads.get_mut(path) {
                    thread.rest = rest;
                    if rest == Rest::End {
                        thread.disposition = Disposition::Completed;
                    }
                }
                Ok(true)
            }
        }
    }

    /// Bind the binder of `node` of the thread `path` to `bound` (PS-5). A
    /// binding `path` made before is immutable for the threads it forked
    /// that still read it: each such thread's branch below `path` keeps a
    /// copy, which it reads before `path`'s new binding.
    pub(super) fn bind(
        &self,
        instance: &mut Instance,
        node: ProtocolNodeId,
        path: &ThreadPath,
        bound: Bound,
    ) -> Result<(), ProtocolRefusal> {
        let key = (node, path.clone());
        if let (Some(slot), Some(old)) = (
            self.program.binder_of.get(&node).copied(),
            instance.binders.get(&key).cloned(),
        ) {
            let mut heirs = Vec::new();
            for (reader, thread) in &instance.threads {
                if thread.disposition != Disposition::Running
                    || reader == path
                    || !path.is_prefix_of(reader)
                    || !self.program.reads_from(thread.rest, slot)
                {
                    continue;
                }
                let reads_old = instance
                    .binding(node, reader)?
                    .is_some_and(|(by, _)| by == path);
                if let (true, Some(step)) = (reads_old, reader.steps.get(path.steps.len())) {
                    let mut heir = path.clone();
                    heir.steps.push(step.clone());
                    heirs.push(heir);
                }
            }
            for heir in heirs {
                instance
                    .binders
                    .entry((node, heir))
                    .or_insert_with(|| old.clone());
            }
        }
        instance.binders.insert(key, bound);
        Ok(())
    }

    /// Whether `constraint` holds for the binder of the reader's node bound
    /// to `record`, read by the reader's thread at `observed`; an undecided
    /// constraint
    /// pushes its finding and holds for no record.
    pub(super) fn admits(
        &self,
        reader: Reader<'_>,
        constraint: Option<&CheckedBody>,
        slot: usize,
        record: &Value,
        observed: &ModelSimState,
        findings: &mut Vec<ModelFinding>,
    ) -> Result<bool, ExpansionStop> {
        let Some(constraint) = constraint else {
            return Ok(true);
        };
        let Reader {
            instance,
            path,
            node,
        } = reader;
        let mut frame = self.frame(instance, path, node)?;
        if let Some(entry) = frame.get_mut(slot) {
            *entry = Some(record.clone());
        }
        Ok(match self.evaluate(constraint, frame, observed)? {
            Verdict::True => true,
            Verdict::False => false,
            Verdict::Undecided(outcome) => {
                findings.push(undetermined(self.program.text(node)?, outcome));
                false
            }
        })
    }

    /// ST-2: for a domain `event` or `commit` node, one step per record of
    /// its type that satisfies its constraint; the binder is bound and the
    /// model state is unchanged.
    fn event(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
        after: Cursor,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        let record = self
            .program
            .events
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("an-event-node-is-an-event"))?;
        let text = self.program.text(node)?.to_owned();
        let instance = Self::instance(state, ordinal)?;
        let values = self
            .model
            .values_of(&record.record_type, &state.model, &mut gathered.budget)?
            .ok_or_else(|| ProtocolRefusal::fault("an-event-type-is-bounded"))?;
        for value in values {
            if !self.admits(
                Reader {
                    instance,
                    path,
                    node,
                },
                record.constraint,
                record.slot,
                &value,
                &state.model,
                &mut gathered.findings,
            )? {
                continue;
            }
            let enc = value_enc(&value);
            let mut next = state.clone();
            self.bind(
                Self::instance_mut(&mut next, ordinal)?,
                node,
                path,
                Bound {
                    value,
                    enc: enc.clone(),
                },
            )?;
            if self.place(&mut next, ordinal, path, after, &mut gathered.findings)? {
                self.normalize(&mut next, ordinal)?;
                gathered.successors.push((
                    ProtocolTransition::record(StepKind::Event, path.clone(), text.clone(), enc),
                    next,
                ));
            }
        }
        Ok(())
    }

    fn gate(&self, state: &ProtocolState<M>, path: &ThreadPath, step: GatedStep) -> bool {
        self.memory.gate(&state.memory, path, step)
    }

    pub(super) fn instance_mut(
        state: &mut ProtocolState<M>,
        ordinal: u64,
    ) -> Result<&mut Instance, ProtocolRefusal> {
        state
            .instances
            .get_mut(&ordinal)
            .ok_or_else(|| ProtocolRefusal::fault("a-stepped-instance-is-live"))
    }

    pub(super) fn instance(
        state: &ProtocolState<M>,
        ordinal: u64,
    ) -> Result<&Instance, ProtocolRefusal> {
        state
            .instances
            .get(&ordinal)
            .ok_or_else(|| ProtocolRefusal::fault("a-stepped-instance-is-live"))
    }

    /// ST-6: `path` forks the parallel instance of `node` with the smallest
    /// ordinal no live sibling holds (PS-2), one `running` branch thread per
    /// branch, each settled from its body's start; the owner rests at the
    /// join. A reused ordinal reuses the paths of threads that have left the
    /// state, so what they bound and counted is cleared first.
    fn fork(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        let join = self
            .program
            .joins
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("a-fork-has-a-join"))?;
        let text = self.program.text(node)?.to_owned();
        let mut next = state.clone();
        let instance = Self::instance_mut(&mut next, ordinal)?;
        let free = Self::free_ordinal(instance, path, node, &text, join)?;
        let owner = ParallelKey {
            parent: path.clone(),
            node,
            ordinal: free,
        };
        let branches: Vec<(ThreadPath, &String, ProtocolNodeId)> = join
            .branches
            .iter()
            .map(|(label, body)| (path.child(&text, label, free), label, *body))
            .collect();
        let stale = |thread: &ThreadPath| {
            branches
                .iter()
                .any(|(child, _, _)| child.is_prefix_of(thread))
        };
        instance.binders.retain(|(_, thread), _| !stale(thread));
        instance.counters.retain(|(_, thread), _| !stale(thread));
        let mut dispositions = std::collections::BTreeMap::new();
        let mut children = Vec::with_capacity(branches.len());
        for (child, label, body) in &branches {
            let rest = match self.settle(instance, child, Cursor::Enter(*body), &state.model)? {
                Settled::Rest(rest) => rest,
                Settled::Undecided(finding) => {
                    gathered.findings.push(*finding);
                    return Ok(());
                }
            };
            let disposition = if rest == Rest::End {
                Disposition::Completed
            } else {
                Disposition::Running
            };
            instance.threads.insert(
                child.clone(),
                Thread {
                    rest,
                    disposition,
                    owner: Some((owner.clone(), (*label).clone())),
                },
            );
            dispositions.insert((*label).clone(), disposition);
            children.push(child.clone());
        }
        instance.parallels.insert(
            owner,
            Parallel {
                settled: false,
                dispositions,
            },
        );
        if let Some(thread) = instance.threads.get_mut(path) {
            thread.rest = Rest::Join(node);
        }
        // `new` resolved a model for every `parallel`, so a missing one is
        // a broken invariant, never a silent `sc`.
        let model = self
            .governing
            .get(&node)
            .copied()
            .ok_or_else(|| ProtocolRefusal::fault("a-forked-parallel-has-a-model"))?;
        next.memory = self.memory.split(&next.memory, path, &children, model);
        self.normalize(&mut next, ordinal)?;
        gathered
            .successors
            .push((ProtocolTransition::fork(path.clone(), text), next));
        Ok(())
    }

    /// The ordinal of the parallel instance `path`'s fork of `node`, whose
    /// static declaration path is `text`, creates in `instance` (PS-2): the
    /// smallest no live sibling holds.
    ///
    /// PS-2's "no live sibling holds" reads a sibling as live while its
    /// instance is, or while any thread its branch threads forked still
    /// runs: a reused path would otherwise become the nearest ancestor of
    /// that running descendant, and the fork's clearing of the reused paths
    /// would take bindings it still reads.
    pub(super) fn free_ordinal(
        instance: &Instance,
        path: &ThreadPath,
        node: ProtocolNodeId,
        text: &str,
        join: &Join,
    ) -> Result<u64, ProtocolRefusal> {
        (0u64..)
            .find(|candidate| {
                !instance.parallels.contains_key(&ParallelKey {
                    parent: path.clone(),
                    node,
                    ordinal: *candidate,
                }) && !join.branches.iter().any(|(label, _)| {
                    let branch = path.child(text, label, *candidate);
                    instance
                        .threads
                        .keys()
                        .any(|thread| branch.is_prefix_of(thread))
                })
            })
            .ok_or_else(|| ProtocolRefusal::fault("an-ordinal-is-free"))
    }

    /// The parallel instance `path` waits at the join of `node` for: the one
    /// it forked and has not yet joined.
    pub(super) fn joined_instance<'i>(
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
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
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
        next.memory = self.memory.merge(&next.memory, path, &branches);
        if self.place(
            &mut next,
            ordinal,
            path,
            Cursor::Exit(node),
            &mut gathered.findings,
        )? {
            self.normalize(&mut next, ordinal)?;
            gathered
                .successors
                .push((ProtocolTransition::join(path.clone(), text), next));
        }
        Ok(())
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
    fn attempt(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
        after: Cursor,
        gathered: &mut Gathered<M>,
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
            &mut gathered.budget,
        )?;
        gathered.findings.extend(applications.findings);
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
                let mut frame = self.frame(instance, path, node)?;
                if let (Some(slot), Some(bound)) = (self.program.binder_of.get(&node), &bound) {
                    if let Some(entry) = frame.get_mut(*slot) {
                        *entry = Some(bound.value.clone());
                    }
                }
                match self.evaluate(constraint, frame, &observed)? {
                    Verdict::True => {}
                    Verdict::False => continue,
                    Verdict::Undecided(outcome) => {
                        gathered.findings.push(undetermined(&text, outcome));
                        continue;
                    }
                }
            }
            let access = self.access(node, &applied, &observed)?;
            let identity = ProtocolTransition::attempt(
                path.clone(),
                text.clone(),
                applied.transition,
                reference_enc(&applied.receiver),
                applied.arguments.iter().map(value_enc).collect(),
            );
            let (model, memory) =
                match self
                    .memory
                    .write(&state.memory, path, &access, &state.model, applied.state)
                {
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
            if let Some(bound) = bound {
                self.bind(Self::instance_mut(&mut next, ordinal)?, node, path, bound)?;
            }
            if self.place(&mut next, ordinal, path, after, &mut gathered.findings)? {
                self.normalize(&mut next, ordinal)?;
                gathered.successors.push((identity, next));
            }
        }
        Ok(())
    }

    /// FR-220: what the application `applied` of the attempt `node` is, as
    /// EN-1 classified it; local for an attempt outside every weak
    /// `parallel`. A read-modify-write whose post-state leaves its location
    /// at the value `observed` gives it applies as a load (ADR-025 MA-5).
    fn access<'s>(
        &'s self,
        node: ProtocolNodeId,
        applied: &Applied,
        observed: &ModelSimState,
    ) -> Result<Access<'s>, ProtocolRefusal> {
        let Some(weak) = self.weak.of(node) else {
            return Ok(Access::local());
        };
        let Some(InstanceAccess::Access { class, location }) = weak.instance(&applied.transition)
        else {
            return Err(ProtocolRefusal::fault(
                "a-weak-attempt-application-is-one-classified-access",
            ));
        };
        let shared = match location {
            Some(location) => Some(SharedCell {
                location,
                cell: self
                    .weak
                    .cell(location)
                    .ok_or_else(|| ProtocolRefusal::fault("a-shared-location-has-a-cell"))?,
            }),
            None => None,
        };
        let unchanged = shared.is_some_and(|shared| {
            observed.cell(shared.cell).map(value_enc)
                == applied.state.cell(shared.cell).map(value_enc)
        });
        Ok(Access {
            class: if *class == AccessClass::Rmw && unchanged {
                AccessClass::Load
            } else {
                *class
            },
            ordering: weak.ordering,
            shared,
        })
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
        gathered: &mut Gathered<M>,
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
            .values_of(record_type, &state.model, &mut gathered.budget)?
            .ok_or_else(|| ProtocolRefusal::fault("a-finish-type-is-bounded"))?;
        for record in records {
            if !self.admits(
                Reader {
                    instance,
                    path,
                    node: finish.node,
                },
                finish.constraint,
                *slot,
                &record,
                &state.model,
                &mut gathered.findings,
            )? {
                continue;
            }
            let mut next = state.clone();
            next.instances.remove(&ordinal);
            gathered.successors.push((
                ProtocolTransition::record(
                    StepKind::Finish,
                    path.clone(),
                    finish.text.clone(),
                    value_enc(&record),
                ),
                next,
            ));
        }
        Ok(())
    }

    /// PS-3, PS-4, PS-5 after a step of instance `ordinal`.
    pub(super) fn normalize(
        &self,
        state: &mut ProtocolState<M>,
        ordinal: u64,
    ) -> Result<(), ProtocolRefusal> {
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
        // A binding stays while some running thread that can still reach a
        // node reading it would read that binding.
        let running: Vec<(&ThreadPath, Rest)> = instance
            .threads
            .iter()
            .filter(|(_, thread)| thread.disposition == Disposition::Running)
            .map(|(path, thread)| (path, thread.rest))
            .collect();
        let nodes: BTreeSet<ProtocolNodeId> =
            instance.binders.keys().map(|(node, _)| *node).collect();
        let mut live: BTreeSet<(ProtocolNodeId, ThreadPath)> = BTreeSet::new();
        for node in nodes {
            let Some(slot) = self.program.binder_of.get(&node).copied() else {
                continue;
            };
            for (reader, rest) in &running {
                if !self.program.reads_from(*rest, slot) {
                    continue;
                }
                for by in instance.read_by(node, reader) {
                    live.insert((node, by));
                }
            }
        }
        instance.binders.retain(|key, _| live.contains(key));
        let counting: BTreeSet<ThreadPath> =
            running.iter().map(|(path, _)| (*path).clone()).collect();
        instance
            .counters
            .retain(|(_, thread), _| counting.contains(thread));
        Ok(())
    }

    /// ADR-027 PD-4: every waiting thread of `state` with its rest point and
    /// wait cause, in ascending JCS bytes of its path. A `running` thread
    /// with no enabled step waits; a completed or cancelled one does not.
    pub(super) fn blocked_threads(
        &self,
        state: &ProtocolState<M>,
    ) -> Result<Vec<BlockedThread>, ExpansionStop> {
        let (expansion, _) = self.expand_bounded(state)?;
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
            Rest::Node(node) => match self.program.control(node)?.step {
                ControlStep::Attempt => {
                    if !self.gate(state, path, GatedStep::Attempt) {
                        WaitCause::Gate
                    } else {
                        self.attempt_cause(state, path, node)?
                    }
                }
                ControlStep::Send => {
                    if !self.gate(state, path, GatedStep::Send) {
                        WaitCause::Gate
                    } else {
                        self.send_cause(state, instance, path, node)?
                    }
                }
                ControlStep::Receive => {
                    if !self.gate(state, path, GatedStep::Receive) {
                        WaitCause::Gate
                    } else {
                        self.receive_cause(instance, node)?
                    }
                }
                ControlStep::Event => WaitCause::Constraint,
                ControlStep::Sequence
                | ControlStep::Branch
                | ControlStep::Parallel
                | ControlStep::Choice
                | ControlStep::Case
                | ControlStep::Repeat
                | ControlStep::Await
                | ControlStep::Check
                | ControlStep::Unstepped(_) => {
                    return Err(ProtocolRefusal::fault("a-thread-rests-at-an-event-node").into())
                }
            },
            Rest::Check(node) => WaitCause::CheckFalse {
                node: self.program.text(node)?.to_owned(),
            },
            // `timeout` is always enabled.
            Rest::Await(_) => return Ok(None),
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

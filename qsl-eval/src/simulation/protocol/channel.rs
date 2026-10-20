// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 ST-3 to ST-5, PS-7 and QSpec FR-170: the channel steps. A `send`
//! appends its entry to its channel's queue for the payload's FIFO key, as
//! the channel's capacity and overflow policy admit; a `receive` takes a
//! deliverable entry its `of` names, the head of a `fifo` queue or any entry
//! of an `unordered` one; and `duplicate` and `lose` copy or drop a queued
//! entry as the channel's delivery premise admits.
//!
//! A queue's capacity bounds each queue, one per FIFO key. A `send` at
//! capacity waits under `block`; under `reject` it leaves the queue
//! unchanged; under `loss` it drops the oldest (`head`) or newest (`tail`)
//! entry and appends, or drops its own (`new`). Its binder records its
//! outcome, `sent`, `rejected` or `lost`, in the `send-record` the step
//! binds, beside the payload its readers read.

use qsl_semantics::check::ProtocolNodeId;
use qsl_semantics::check::{ChannelOverflow, LossSelector};
use qsl_semantics::family::FamilyOutcome;
use quire_exact::{Meter, Outcome, Value};

use super::identity::{ProtocolTransition, StepKind, ThreadPath, WaitCause};
use super::memory::MemoryModel;
use super::program::{Channel, Cursor, Message};
use super::steps::{jcs, Gathered};
use super::{
    broken, undetermined, Bound, Entry, FifoKey, Instance, ProtocolRefusal, ProtocolState,
    ProtocolSystem, QueueKey,
};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::model::encode::{value_enc, Enc};
use crate::simulation::model::{verdict, ModelFinding, ModelSimState, Verdict};
use crate::value::evaluate_protocol_slot;

/// What a `send` did with its entry (QSpec FR-170).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SendOutcome {
    /// Appended.
    Sent,
    /// Refused at capacity under `overflow reject`.
    Rejected,
    /// Admitted at capacity under `overflow loss`, losing one entry.
    Lost,
}

impl SendOutcome {
    fn text(self) -> &'static str {
        match self {
            Self::Sent => "sent",
            Self::Rejected => "rejected",
            Self::Lost => "lost",
        }
    }
}

/// The record a `send` binds: its payload and its outcome.
fn send_record(payload: &Value, outcome: SendOutcome) -> Enc {
    Enc::tagged(
        "send-record",
        vec![
            ("payload", value_enc(payload)),
            ("outcome", Enc::text(outcome.text())),
        ],
    )
}

/// Insert `entry` into an `unordered` queue, kept in ascending JCS bytes.
fn insert_sorted(queue: &mut Vec<Entry>, entry: Entry) {
    let at = queue.partition_point(|queued| queued.bytes <= entry.bytes);
    queue.insert(at, entry);
}

impl<M: MemoryModel> ProtocolSystem<'_, M> {
    /// The `send` or `receive` `node`.
    fn message(&self, node: ProtocolNodeId) -> Result<&Message<'_>, ProtocolRefusal> {
        self.program
            .messages
            .get(&node)
            .ok_or_else(|| ProtocolRefusal::fault("a-message-node-is-a-message"))
    }

    /// The channel the `send` or `receive` `node` is written `via`.
    pub(super) fn channel_of(&self, node: ProtocolNodeId) -> Result<&Channel<'_>, ProtocolRefusal> {
        let message = self.message(node)?;
        self.program
            .channels
            .get(message.channel)
            .ok_or_else(|| ProtocolRefusal::fault("a-message-names-a-channel"))
    }

    /// The queue `payload` goes to on channel `channel`: its FIFO key's
    /// value for a `fifo` channel, none for an `unordered` one. `Err` holds
    /// the finding of a key the evaluator does not decide.
    fn queue_key(
        &self,
        channel: usize,
        payload: &Value,
        observed: &ModelSimState,
        node: ProtocolNodeId,
    ) -> Result<Result<QueueKey, ModelFinding>, ExpansionStop> {
        let key = self
            .program
            .channels
            .get(channel)
            .ok_or_else(|| ProtocolRefusal::fault("a-message-names-a-channel"))?
            .key;
        let Some(body) = key else {
            return Ok(Ok(QueueKey {
                channel,
                fifo: None,
            }));
        };
        let observation = self.model.observation_of(observed)?;
        let evaluation = evaluate_protocol_slot(
            self.package,
            body,
            vec![Some(payload.clone())],
            &observation.environment,
            &mut Meter::new(self.model.meter()),
        )
        .map_err(|_| broken())?;
        if let FamilyOutcome::Evaluated(Outcome::Completed(value)) = &evaluation.outcome {
            let enc = value_enc(value);
            return Ok(Ok(QueueKey {
                channel,
                fifo: Some(FifoKey {
                    bytes: jcs(&enc)?,
                    enc,
                }),
            }));
        }
        match verdict(&evaluation.outcome) {
            Verdict::Undecided(outcome) => Ok(Err(undetermined(self.program.text(node)?, outcome))),
            Verdict::True | Verdict::False => Err(broken()),
        }
    }

    /// ST-3: one step per payload of the channel's type that satisfies the
    /// constraint, unless its queue is at capacity under `block`.
    pub(super) fn send(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
        after: Cursor,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        let message = self.message(node)?;
        let channel = self.channel_of(node)?;
        let text = self.program.text(node)?.to_owned();
        let instance = Self::instance(state, ordinal)?;
        let payloads = self
            .model
            .values_of(
                &message.record.record_type,
                &state.model,
                &mut gathered.budget,
            )?
            .ok_or_else(|| ProtocolRefusal::fault("a-channel-type-is-bounded"))?;
        for payload in payloads {
            if !self.admits(
                instance,
                path,
                node,
                message.record.constraint,
                message.record.slot,
                &payload,
                &state.model,
                &mut gathered.findings,
            )? {
                continue;
            }
            let key = match self.queue_key(message.channel, &payload, &state.model, node)? {
                Ok(key) => key,
                Err(finding) => {
                    gathered.findings.push(finding);
                    continue;
                }
            };
            let enc = Enc::tagged(
                "queue-entry",
                vec![
                    ("send", Enc::text(text.clone())),
                    ("thread", path.enc()),
                    ("payload", value_enc(&payload)),
                ],
            );
            let entry = Entry {
                send: node,
                payload: payload.clone(),
                bytes: jcs(&enc)?,
                enc,
            };
            let mut queue = instance.queues.get(&key).cloned().unwrap_or_default();
            let fifo = key.fifo.is_some();
            let outcome = if queue.len() < channel.capacity {
                SendOutcome::Sent
            } else {
                match channel.overflow {
                    ChannelOverflow::Block => continue,
                    ChannelOverflow::Reject => SendOutcome::Rejected,
                    ChannelOverflow::Loss(selector) => {
                        // An empty queue at capacity 0 has no queued entry
                        // to drop, so the arriving one is lost.
                        let dropped = match selector {
                            LossSelector::Head if !queue.is_empty() => Some(0),
                            LossSelector::Tail => queue.len().checked_sub(1),
                            LossSelector::Head | LossSelector::New => None,
                        };
                        if let Some(at) = dropped {
                            queue.remove(at);
                            queue.push(entry.clone());
                        }
                        SendOutcome::Lost
                    }
                }
            };
            if outcome == SendOutcome::Sent {
                if fifo {
                    queue.push(entry);
                } else {
                    insert_sorted(&mut queue, entry);
                }
            }
            let record = send_record(&payload, outcome);
            let mut next = state.clone();
            let target = Self::instance_mut(&mut next, ordinal)?;
            store(target, key, queue);
            self.bind(
                target,
                node,
                path,
                Bound {
                    value: payload,
                    enc: record.clone(),
                },
            )?;
            if self.place(&mut next, ordinal, path, after, &mut gathered.findings)? {
                self.normalize(&mut next, ordinal)?;
                gathered.successors.push((
                    ProtocolTransition::record(StepKind::Send, path.clone(), text.clone(), record),
                    next,
                ));
            }
        }
        Ok(())
    }

    /// ST-4: one step per distinct deliverable entry from the `send` the
    /// receive's `of` names whose payload satisfies the constraint: the
    /// head of each `fifo` queue of the channel, any entry of an
    /// `unordered` one. The entry is removed and the binder bound to its
    /// payload.
    pub(super) fn receive(
        &self,
        state: &ProtocolState<M>,
        ordinal: u64,
        path: &ThreadPath,
        node: ProtocolNodeId,
        after: Cursor,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        let message = self.message(node)?;
        let text = self.program.text(node)?.to_owned();
        let instance = Self::instance(state, ordinal)?;
        for (at, entry) in Self::deliverable(instance, message) {
            if !self.admits(
                instance,
                path,
                node,
                message.record.constraint,
                message.record.slot,
                &entry.payload,
                &state.model,
                &mut gathered.findings,
            )? {
                continue;
            }
            let payload = entry.payload.clone();
            let enc = value_enc(&payload);
            let mut next = state.clone();
            let target = Self::instance_mut(&mut next, ordinal)?;
            let mut queue = target.queues.get(&at.0).cloned().unwrap_or_default();
            if at.1 < queue.len() {
                queue.remove(at.1);
            }
            store(target, at.0, queue);
            self.bind(
                target,
                node,
                path,
                Bound {
                    value: payload,
                    enc: enc.clone(),
                },
            )?;
            if self.place(&mut next, ordinal, path, after, &mut gathered.findings)? {
                self.normalize(&mut next, ordinal)?;
                gathered.successors.push((
                    ProtocolTransition::record(StepKind::Receive, path.clone(), text.clone(), enc),
                    next,
                ));
            }
        }
        Ok(())
    }

    /// The distinct deliverable entries for `message` in `instance`, each
    /// with its queue and position.
    fn deliverable<'i>(
        instance: &'i Instance,
        message: &Message<'_>,
    ) -> Vec<((QueueKey, usize), &'i Entry)> {
        let mut found: Vec<((QueueKey, usize), &Entry)> = Vec::new();
        for (key, queue) in &instance.queues {
            if key.channel != message.channel {
                continue;
            }
            let candidates: Vec<(usize, &Entry)> = if key.fifo.is_some() {
                queue.first().map(|head| (0, head)).into_iter().collect()
            } else {
                queue.iter().enumerate().collect()
            };
            for (position, entry) in candidates {
                let distinct = !found
                    .iter()
                    .any(|((other, _), seen)| other == key && seen.bytes == entry.bytes);
                if Some(entry.send) == message.from && distinct {
                    found.push(((key.clone(), position), entry));
                }
            }
        }
        found
    }

    /// PD-4 for a waiting `send`: `full-queue` when some payload its
    /// constraint admits waits only on its queue's capacity, else
    /// `constraint`.
    pub(super) fn send_cause(
        &self,
        state: &ProtocolState<M>,
        instance: &Instance,
        path: &ThreadPath,
        node: ProtocolNodeId,
    ) -> Result<WaitCause, ExpansionStop> {
        let message = self.message(node)?;
        let channel = self.channel_of(node)?;
        let mut budget = self.model.budget();
        let mut findings = Vec::new();
        let payloads = self
            .model
            .values_of(&message.record.record_type, &state.model, &mut budget)?
            .ok_or_else(|| ProtocolRefusal::fault("a-channel-type-is-bounded"))?;
        for payload in payloads {
            if self.admits(
                instance,
                path,
                node,
                message.record.constraint,
                message.record.slot,
                &payload,
                &state.model,
                &mut findings,
            )? {
                return Ok(WaitCause::FullQueue {
                    channel: channel.name.clone(),
                });
            }
        }
        Ok(WaitCause::Constraint)
    }

    /// ST-5: every `duplicate` and `lose` its channels' delivery premises
    /// admit, over every queue of every live instance.
    pub(super) fn channel_steps(
        &self,
        state: &ProtocolState<M>,
        gathered: &mut Gathered<M>,
    ) -> Result<(), ExpansionStop> {
        for (ordinal, instance) in &state.instances {
            for (key, queue) in &instance.queues {
                let channel = self
                    .program
                    .channels
                    .get(key.channel)
                    .ok_or_else(|| ProtocolRefusal::fault("a-queue-names-a-channel"))?;
                let identity = Enc::tagged(
                    "channel",
                    vec![
                        ("name", Enc::text(channel.name.clone())),
                        ("instance", Enc::text(ordinal.to_string())),
                    ],
                );
                let mut results: Vec<(StepKind, Vec<Vec<u8>>)> = Vec::new();
                for (position, entry) in queue.iter().enumerate() {
                    let mut variants = Vec::new();
                    if channel.duplicates() && queue.len() < channel.capacity {
                        let mut copied = queue.clone();
                        if key.fifo.is_some() {
                            copied.push(entry.clone());
                        } else {
                            insert_sorted(&mut copied, entry.clone());
                        }
                        variants.push((StepKind::Duplicate, copied));
                    }
                    if channel.loses() {
                        let mut lost = queue.clone();
                        lost.remove(position);
                        variants.push((StepKind::Lose, lost));
                    }
                    for (kind, changed) in variants {
                        let bytes: Vec<Vec<u8>> =
                            changed.iter().map(|queued| queued.bytes.clone()).collect();
                        if results
                            .iter()
                            .any(|(seen, result)| *seen == kind && *result == bytes)
                        {
                            continue;
                        }
                        results.push((kind, bytes));
                        let mut next = state.clone();
                        store(
                            Self::instance_mut(&mut next, *ordinal)?,
                            key.clone(),
                            changed,
                        );
                        gathered.successors.push((
                            ProtocolTransition::channel(kind, identity.clone(), entry.enc.clone()),
                            next,
                        ));
                    }
                }
            }
        }
        Ok(())
    }
}

/// Set the queue `key` of `instance` to `queue`; an empty queue leaves the
/// state (PS-8).
fn store(instance: &mut Instance, key: QueueKey, queue: Vec<Entry>) {
    if queue.is_empty() {
        instance.queues.remove(&key);
    } else {
        instance.queues.insert(key, queue);
    }
}

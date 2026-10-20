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
//! entry and appends, or drops its own (`new`). Its binder is bound to the
//! sent payload, and the step carries its send record `{channel, payload,
//! outcome, displaced}` (ST-3, PX-1): `outcome` is `accepted` when its entry
//! was appended, `rejected` or `lost` when it was not, and `displaced` the
//! queued entry a `head` or `tail` selector removed for it.

use qsl_semantics::check::ProtocolNodeId;
use qsl_semantics::check::{ChannelOverflow, LossSelector};
use qsl_semantics::family::FamilyOutcome;
use quire_exact::{Meter, Outcome, Value};

use super::identity::{
    ProtocolTransition, QueueEvent, Removal, SendOutcome, SendRecord, StepKind, ThreadPath,
    WaitCause,
};
use super::memory::MemoryModel;
use super::program::{Channel, Cursor, Message};
use super::steps::{jcs, Gathered, Reader};
use super::{
    broken, undetermined, Bound, Entry, FifoKey, Instance, ProtocolRefusal, ProtocolState,
    ProtocolSystem, QueueKey,
};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::model::encode::{value_enc, Enc};
use crate::simulation::model::{verdict, ModelFinding, ModelSimState, Verdict};
use crate::value::evaluate_protocol_slot;

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
                Reader {
                    instance,
                    path,
                    node,
                },
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
            let mut displaced = None;
            let outcome = if queue.len() < channel.capacity {
                if fifo {
                    queue.push(entry);
                } else {
                    insert_sorted(&mut queue, entry);
                }
                SendOutcome::Accepted
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
                            displaced = Some(queue.remove(at));
                            queue.push(entry);
                            SendOutcome::Accepted
                        } else {
                            SendOutcome::Lost
                        }
                    }
                }
            };
            let record = SendRecord::new(
                channel.name.clone(),
                value_enc(&payload),
                outcome,
                displaced.as_ref().map(|entry| entry.enc.clone()),
            );
            let enc = value_enc(&payload);
            let mut next = state.clone();
            let target = Self::instance_mut(&mut next, ordinal)?;
            store(target, key, queue);
            Self::bind(
                target,
                node,
                path,
                Bound {
                    value: payload,
                    enc: enc.clone(),
                },
            );
            let mut transition =
                ProtocolTransition::record(StepKind::Send, path.clone(), text.clone(), enc)
                    .carrying(Some(record));
            let dropped = match (outcome, &displaced) {
                (SendOutcome::Lost, _) => Some(text.clone()),
                (_, Some(entry)) => Some(self.program.text(entry.send)?.to_owned()),
                (SendOutcome::Accepted | SendOutcome::Rejected, None) => None,
            };
            if let Some(send) = dropped {
                transition = transition.removing(QueueEvent {
                    channel: channel.name.clone(),
                    instance: ordinal,
                    send,
                    removal: Removal::Lost,
                });
            }
            if self.place(&mut next, ordinal, path, after, &mut gathered.findings)? {
                self.normalize(&mut next, ordinal)?;
                gathered.successors.push((transition, next));
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
        let channel = self.channel_of(node)?.name.clone();
        let text = self.program.text(node)?.to_owned();
        let instance = Self::instance(state, ordinal)?;
        for (at, entry) in Self::deliverable(instance, message) {
            if !self.admits(
                Reader {
                    instance,
                    path,
                    node,
                },
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
            Self::bind(
                target,
                node,
                path,
                Bound {
                    value: payload,
                    enc: enc.clone(),
                },
            );
            let transition =
                ProtocolTransition::record(StepKind::Receive, path.clone(), text.clone(), enc)
                    .removing(QueueEvent {
                        channel: channel.clone(),
                        instance: ordinal,
                        send: self.program.text(entry.send)?.to_owned(),
                        removal: Removal::Received,
                    });
            if self.place(&mut next, ordinal, path, after, &mut gathered.findings)? {
                self.normalize(&mut next, ordinal)?;
                gathered.successors.push((transition, next));
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

    /// PD-4 for a waiting `receive` of instance `ordinal`, over the steps
    /// `prefix` took to reach the state: `constraint` when an entry it could
    /// take fails its constraint; `head-blocked` when its send's entries wait
    /// behind another send's head of a `fifo` queue; `displaced` when none of
    /// its send's entries is queued and the last step of `prefix` that took
    /// one off the channel was a loss; else `empty-queue`.
    pub(super) fn receive_cause(
        &self,
        instance: &Instance,
        ordinal: u64,
        node: ProtocolNodeId,
        prefix: &[&ProtocolTransition],
    ) -> Result<WaitCause, ProtocolRefusal> {
        let message = self.message(node)?;
        let channel = self.channel_of(node)?.name.clone();
        if !Self::deliverable(instance, message).is_empty() {
            return Ok(WaitCause::Constraint);
        }
        let queued = instance
            .queues
            .iter()
            .filter(|(key, _)| key.channel == message.channel)
            .flat_map(|(_, queue)| queue.iter())
            .any(|entry| Some(entry.send) == message.from);
        if queued {
            return Ok(WaitCause::HeadBlocked { channel });
        }
        let send = match message.from {
            Some(from) => Some(self.program.text(from)?),
            None => None,
        };
        let last = prefix.iter().rev().find_map(|step| {
            step.removed().filter(|removed| {
                removed.channel == channel
                    && removed.instance == ordinal
                    && Some(removed.send.as_str()) == send
            })
        });
        Ok(match last.map(|removed| removed.removal) {
            Some(Removal::Lost) => WaitCause::Displaced { channel },
            Some(Removal::Received) | None => WaitCause::EmptyQueue { channel },
        })
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
                Reader {
                    instance,
                    path,
                    node,
                },
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
                        let mut transition = ProtocolTransition::channel(
                            kind,
                            channel.name.clone(),
                            *ordinal,
                            entry.enc.clone(),
                        );
                        if kind == StepKind::Lose {
                            transition = transition.removing(QueueEvent {
                                channel: channel.name.clone(),
                                instance: *ordinal,
                                send: self.program.text(entry.send)?.to_owned(),
                                removal: Removal::Lost,
                            });
                        }
                        gathered.successors.push((transition, next));
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

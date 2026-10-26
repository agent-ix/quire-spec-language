// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-227 "For `kind: Race`" (ADR-025 DR-6; QSpec FR-438 "Race replay"):
//! happens-before over a replayed prefix by vector clocks, computed from the
//! replayed steps alone and independent of the engine's race summary.
//!
//! Each thread holds three clocks that play the part of RA-1's views
//! `cur`, `acq` and `rel(ℓ)`, and each message of a location's modification
//! order holds the clock its writer gave it. Synchronisation is DR-2's, over
//! RA-2 to RA-6:
//!
//! - a load joins the clock of the message it reads into `acq`, and into
//!   `cur` when it is `acquire` or stronger;
//! - a store gives its message `cur` and sets `rel(ℓ)` to it when it is
//!   `release` or stronger, else gives it `rel(ℓ)`; a read-modify-write also
//!   joins the clock of the message it read, and decides its race only after
//!   its own acquire join;
//! - a fence joins `acq` into `cur` when it is `acquire` or stronger, sets
//!   `rel(ℓ)` of every location, touched or not, to `cur` when it is
//!   `release` or stronger, and a
//!   `seq_cst` one joins the global fence clock into `cur` and takes it back;
//! - `fork` copies a thread's clocks to each branch and `join` joins the
//!   branches' into the parent;
//! - a `send` queues `cur` and its `receive` joins it into `cur` and `acq`;
//!   a `duplicate` copies a queued entry with its clock, and a `lose`, or a
//!   `send` that displaces one at capacity, drops it.
//!
//! Under `tso` there is no global fence clock (DR-3), so a `seq_cst` fence is
//! an acquire and a release only. There are no messages either: a store waits in its thread's buffer
//! with the clock it was given, `flush` moves it to memory, and a load joins
//! the clock of the newest buffered store of its thread, else of memory.
//!
//! A message or slot is found by the depth its step names, counted from the
//! mo-last message, which garbage collection (RA-7) never changes.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::simulation::protocol::{
    AccessRecord, ProtocolTransition, SendOutcome, StepKind, ThreadPath,
};
use qsl_eval::simulation::Location;
use qsl_semantics::check::AccessOrdering;

use crate::model_check::counterexample::Race;

/// A vector clock: for each thread, the last of its events that happens
/// before the holder.
type Clock = BTreeMap<ThreadPath, u64>;

fn join(into: &mut Clock, from: &Clock) {
    for (thread, time) in from {
        let held = into.entry(thread.clone()).or_insert(0);
        *held = (*held).max(*time);
    }
}

/// A thread's three clocks.
#[derive(Clone, Debug, Default)]
struct Clocks {
    cur: Clock,
    acq: Clock,
    /// `rel(ℓ)` for each location a release access has set since the last
    /// release fence; every other location's is `fence`.
    rel: BTreeMap<Location, Clock>,
    /// `cur` at the thread's last release fence, which RA-5 assigns to
    /// `rel(ℓ)` of every location, whether or not it has been touched.
    fence: Clock,
    /// Events the thread has made.
    time: u64,
}

impl Clocks {
    /// The thread's next event, which its `cur` clock then covers.
    fn tick(&mut self, thread: &ThreadPath) -> u64 {
        self.time = self.time.saturating_add(1);
        self.cur.insert(thread.clone(), self.time);
        self.time
    }

    /// `rel(ℓ)`.
    fn released(&self, location: &Location) -> Clock {
        self.rel.get(location).unwrap_or(&self.fence).clone()
    }

    /// These clocks joined with `other`'s.
    fn join(&mut self, other: &Self) {
        let locations: Vec<Location> = self.rel.keys().chain(other.rel.keys()).cloned().collect();
        for location in locations {
            let mut joined = self.released(&location);
            join(&mut joined, &other.released(&location));
            self.rel.insert(location, joined);
        }
        join(&mut self.fence, &other.fence);
        join(&mut self.cur, &other.cur);
        join(&mut self.acq, &other.acq);
    }
}

/// One shared-location access of the replayed prefix, as the check reads it.
struct Event {
    thread: ThreadPath,
    location: Location,
    writes: bool,
    atomic: bool,
    /// The thread's event number of the access.
    time: u64,
    /// The thread's `cur` clock at the access.
    clock: Clock,
}

/// A queue: its channel, its instance and its FIFO key, `None` for an
/// `unordered` channel (ADR-027 PS-7).
type QueueName = (String, u64, Option<Vec<u8>>);

/// A queued entry: its canonical bytes and the clock its sender carried.
type Queued = (Vec<u8>, Clock);

/// The vector-clock state of a replayed prefix.
#[derive(Default)]
struct Replay {
    /// The prefix runs under `tso`, which has no counterpart of `S` (DR-3).
    tso: bool,
    threads: BTreeMap<ThreadPath, Clocks>,
    /// The threads that hold views: the parent and the branches of every
    /// `parallel` forked and not yet joined (RA-6).
    live: BTreeSet<ThreadPath>,
    /// Each location's messages in modification order, oldest first: the
    /// clock their writers gave them. The initial message holds none.
    messages: BTreeMap<Location, Vec<Clock>>,
    /// Each `tso` thread's store buffer, oldest first: the location and the
    /// clock the store was given.
    buffers: BTreeMap<ThreadPath, Vec<(Location, Clock)>>,
    /// Each location's memory value under `tso`: the clock of the store that
    /// wrote it. The initial value holds none.
    memory: BTreeMap<Location, Clock>,
    /// The global `seq_cst` fence clock.
    fences: Clock,
    /// Each channel's queued entries, oldest first: the entry's canonical
    /// bytes and the clock the sender carried.
    queues: BTreeMap<QueueName, Vec<Queued>>,
}

/// Whether the access loads or stores with acquire or release strength
/// (RA-2, RA-3).
fn strength(access: &AccessRecord) -> (bool, bool) {
    let acquires = matches!(
        access.ordering,
        Some(AccessOrdering::Acquire | AccessOrdering::AcqRel | AccessOrdering::SeqCst)
    );
    let releases = matches!(
        access.ordering,
        Some(AccessOrdering::Release | AccessOrdering::AcqRel | AccessOrdering::SeqCst)
    );
    (acquires, releases)
}

/// The position of the `before`-th entry of `queue` with the bytes `entry`.
fn nth_equal(queue: &[Queued], entry: &[u8], before: usize) -> Option<usize> {
    queue
        .iter()
        .enumerate()
        .filter(|(_, (held, _))| held == entry)
        .nth(before)
        .map(|(at, _)| at)
}

/// Add `entry` to `queue` as the engine does: at the end of a `fifo` queue,
/// after the entries of no greater bytes in an `unordered` one.
fn insert(queue: &mut Vec<Queued>, entry: Queued, fifo: bool) {
    let at = if fifo {
        queue.len()
    } else {
        queue.partition_point(|(held, _)| *held <= entry.0)
    };
    queue.insert(at, entry);
}

impl Replay {
    fn thread(&mut self, path: &ThreadPath) -> &mut Clocks {
        self.threads.entry(path.clone()).or_default()
    }

    /// The `before`-th queued entry of `queue` with the canonical bytes
    /// `entry`; entries of equal bytes are taken in queue order.
    fn take(&mut self, queue: &QueueName, entry: &[u8], before: usize) -> Option<Clock> {
        let queued = self.queues.get_mut(queue)?;
        let at = nth_equal(queued, entry, before)?;
        Some(queued.remove(at).1)
    }

    /// The step at one position, and the access event it makes.
    fn step(&mut self, step: &ProtocolTransition) -> Option<Event> {
        match step.kind() {
            StepKind::Duplicate | StepKind::Lose => {
                let (channel, instance) = step.channel_queue()?;
                let flow = step.queue_flow()?;
                let queue = (channel.to_owned(), instance, flow.key.clone());
                let queued = self.queues.get_mut(&queue)?;
                let at = nth_equal(queued, &flow.entry, flow.before)?;
                if step.kind() == StepKind::Lose {
                    queued.remove(at);
                } else {
                    // The engine keeps the original and adds one copy: at the
                    // end of a `fifo` queue, after the equal entries of an
                    // `unordered` one.
                    let copy = (flow.entry.clone(), queued[at].1.clone());
                    insert(queued, copy, flow.key.is_some());
                }
                return None;
            }
            _ => {}
        }
        if let Some(flush) = step.memory_step() {
            // TSO-4: `flush(b)` writes the oldest buffered store to memory.
            let buffer = self.buffers.get_mut(&flush.thread)?;
            if buffer.is_empty() {
                return None;
            }
            let (location, clock) = buffer.remove(0);
            self.memory.insert(location, clock);
            return None;
        }
        let thread = step.thread()?;
        match step.kind() {
            StepKind::Attempt => self.attempt(step, thread),
            StepKind::Fence => {
                self.fence(thread, step.fence_ordering()?);
                None
            }
            StepKind::Fork => {
                let parent = self.thread(thread);
                parent.tick(thread);
                let copy = parent.clone();
                self.live.insert(thread.clone());
                for child in step.fork_children() {
                    self.threads.insert(child.clone(), copy.clone());
                    self.live.insert(child.clone());
                }
                None
            }
            StepKind::Join => {
                let below: Vec<ThreadPath> = self
                    .live
                    .iter()
                    .filter(|path| *path != thread && thread.is_prefix_of(path))
                    .cloned()
                    .collect();
                let mut joined = self.thread(thread).clone();
                for path in &below {
                    self.live.remove(path);
                    if let Some(clocks) = self.threads.remove(path) {
                        joined.join(&clocks);
                    }
                }
                joined.tick(thread);
                self.threads.insert(thread.clone(), joined);
                if self.live.len() == 1 && self.live.contains(thread) {
                    // An outermost `join`: the next `parallel` starts from
                    // one initial message per location (RA-1, RA-7).
                    self.messages.clear();
                    self.memory.clear();
                    self.buffers.clear();
                    self.live.clear();
                }
                None
            }
            StepKind::Send => {
                let record = step.send_record()?;
                let flow = step.queue_flow()?;
                let queue = (
                    record.channel().to_owned(),
                    thread.instance,
                    flow.key.clone(),
                );
                let displacing = flow.displaced.is_some();
                if let Some((displaced, before)) = &flow.displaced {
                    self.take(&queue, displaced, *before);
                }
                if record.outcome() != SendOutcome::Accepted {
                    return None;
                }
                let clocks = self.thread(thread);
                clocks.tick(thread);
                let carried = clocks.cur.clone();
                let queued = self.queues.entry(queue).or_default();
                let entry = (flow.entry.clone(), carried);
                if displacing {
                    // A send that displaces an entry is appended whatever the
                    // channel's order.
                    queued.push(entry);
                } else {
                    insert(queued, entry, flow.key.is_some());
                }
                None
            }
            StepKind::Receive => {
                let (channel, instance, _) = step.received()?;
                let flow = step.queue_flow()?;
                let carried = self.take(
                    &(channel.to_owned(), instance, flow.key.clone()),
                    &flow.entry,
                    flow.before,
                )?;
                let clocks = self.thread(thread);
                clocks.tick(thread);
                join(&mut clocks.cur, &carried);
                join(&mut clocks.acq, &carried);
                None
            }
            _ => None,
        }
    }

    /// RA-5, DR-2.
    fn fence(&mut self, thread: &ThreadPath, ordering: AccessOrdering) {
        let acquires = matches!(
            ordering,
            AccessOrdering::Acquire | AccessOrdering::AcqRel | AccessOrdering::SeqCst
        );
        let releases = matches!(
            ordering,
            AccessOrdering::Release | AccessOrdering::AcqRel | AccessOrdering::SeqCst
        );
        let tso = self.tso;
        let clocks = self.thread(thread);
        clocks.tick(thread);
        if acquires {
            let acquired = clocks.acq.clone();
            join(&mut clocks.cur, &acquired);
        }
        if releases {
            // `rel(ℓ)` of every location, touched or not.
            clocks.fence = clocks.cur.clone();
            clocks.rel.clear();
        }
        if ordering == AccessOrdering::SeqCst && !tso {
            let mut current = clocks.cur.clone();
            join(&mut current, &self.fences);
            self.fences.clone_from(&current);
            self.thread(thread).cur = current;
        }
    }

    /// RA-2 to RA-4, DR-2: an attempt's shared access.
    fn attempt(&mut self, step: &ProtocolTransition, thread: &ThreadPath) -> Option<Event> {
        let access = step.access()?;
        let Some(choice) = step.memory_choice() else {
            return self.buffered_attempt(access, thread);
        };
        let (acquires, releases) = strength(access);
        let location = &access.location;
        let messages = self
            .messages
            .entry(location.clone())
            .or_insert_with(|| vec![Clock::new()]);
        let len = messages.len();
        let read = match choice.read_depth {
            Some(depth) => {
                let at = len
                    .checked_sub(1)?
                    .checked_sub(usize::try_from(depth).ok()?)?;
                Some((at, messages.get(at)?.clone()))
            }
            None => None,
        };
        if access.reads && read.is_none() {
            return None;
        }
        let slot = match (access.writes, read.as_ref(), choice.slot_depth) {
            (false, _, _) => None,
            (true, Some((at, _)), _) => Some(at.checked_add(1)?),
            (true, None, Some(depth)) => Some(len.checked_sub(usize::try_from(depth).ok()?)?),
            (true, None, None) => return None,
        };
        let clocks = self.threads.entry(thread.clone()).or_default();
        let time = clocks.tick(thread);
        if let Some((_, carried)) = &read {
            join(&mut clocks.acq, carried);
            if acquires {
                join(&mut clocks.cur, carried);
            }
        }
        let event = Event {
            thread: thread.clone(),
            location: location.clone(),
            writes: access.writes,
            atomic: access.atomic(),
            time,
            clock: clocks.cur.clone(),
        };
        if let Some(slot) = slot {
            let mut given = if releases {
                clocks.rel.insert(location.clone(), clocks.cur.clone());
                clocks.cur.clone()
            } else {
                clocks.released(location)
            };
            if let Some((_, carried)) = &read {
                join(&mut given, carried);
            }
            let messages = self.messages.get_mut(location)?;
            if slot > messages.len() {
                return None;
            }
            messages.insert(slot, given);
        }
        Some(event)
    }
}

impl Replay {
    /// An access under `tso`, which names no message (TSO-2, TSO-3, TSO-5):
    /// a load reads its thread's newest buffered store of the location, else
    /// memory; a store joins the buffer and a locked access (a
    /// read-modify-write, or a `seq_cst` store) writes memory. The clock a
    /// store is given follows RA-3, and a read joins as in RA-2.
    fn buffered_attempt(&mut self, access: &AccessRecord, thread: &ThreadPath) -> Option<Event> {
        let (acquires, releases) = strength(access);
        let location = &access.location;
        let rmw = access.reads && access.writes;
        let locked = rmw || (access.writes && access.ordering == Some(AccessOrdering::SeqCst));
        let source = if access.reads {
            let buffered = if rmw {
                None
            } else {
                self.buffers
                    .get(thread)
                    .and_then(|buffer| buffer.iter().rev().find(|(held, _)| held == location))
                    .map(|(_, clock)| clock.clone())
            };
            Some(buffered.unwrap_or_else(|| self.memory.get(location).cloned().unwrap_or_default()))
        } else {
            None
        };
        let clocks = self.threads.entry(thread.clone()).or_default();
        let time = clocks.tick(thread);
        if let Some(carried) = &source {
            join(&mut clocks.acq, carried);
            if acquires {
                join(&mut clocks.cur, carried);
            }
        }
        let event = Event {
            thread: thread.clone(),
            location: location.clone(),
            writes: access.writes,
            atomic: access.atomic(),
            time,
            clock: clocks.cur.clone(),
        };
        if access.writes {
            let mut given = if releases {
                clocks.rel.insert(location.clone(), clocks.cur.clone());
                clocks.cur.clone()
            } else {
                clocks.released(location)
            };
            if let (true, Some(carried)) = (rmw, &source) {
                join(&mut given, carried);
            }
            if locked {
                self.memory.insert(location.clone(), given);
            } else {
                self.buffers
                    .entry(thread.clone())
                    .or_default()
                    .push((location.clone(), given));
            }
        }
        Some(event)
    }
}

/// DR-6: whether the accesses at the positions `race` names are by different
/// threads, touch its location, include a write and a non-atomic access, and
/// are unordered by happens-before over `steps`, the replayed prefix.
pub(super) fn reproduces(steps: &[ProtocolTransition], race: &Race, tso: bool) -> bool {
    let mut replay = Replay {
        tso,
        ..Replay::default()
    };
    let events: Vec<Option<Event>> = steps.iter().map(|step| replay.step(step)).collect();
    let event = |position: u64| -> Option<&Event> {
        let at = usize::try_from(position).ok()?.checked_sub(1)?;
        events.get(at)?.as_ref()
    };
    let (Some(earlier), Some(later)) = (event(race.earlier), event(race.later)) else {
        return false;
    };
    race.earlier < race.later
        && earlier.location == race.location
        && later.location == race.location
        && earlier.thread != later.thread
        && (earlier.writes || later.writes)
        && !(earlier.atomic && later.atomic)
        && later
            .clock
            .get(&earlier.thread)
            .is_none_or(|seen| *seen < earlier.time)
}

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
//!   every `rel(ℓ)` to `cur` when it is `release` or stronger, and a
//!   `seq_cst` one joins the global fence clock into `cur` and takes it back;
//! - `fork` copies a thread's clocks to each branch and `join` joins the
//!   branches' into the parent;
//! - a `send` queues `cur` and its `receive` joins it into `cur` and `acq`.
//!
//! A message or slot is found by the depth its step names, counted from the
//! mo-last message, which garbage collection (RA-7) never changes.

use std::collections::BTreeMap;

use qsl_eval::simulation::protocol::{
    AccessRecord, ProtocolTransition, SendOutcome, StepKind, ThreadPath,
};
use qsl_eval::simulation::Location;

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
    rel: BTreeMap<Location, Clock>,
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

/// The vector-clock state of a replayed prefix.
#[derive(Default)]
struct Replay {
    threads: BTreeMap<ThreadPath, Clocks>,
    /// Each location's messages in modification order, oldest first: the
    /// clock their writers gave them. The initial message holds none.
    messages: BTreeMap<Location, Vec<Clock>>,
    /// The global `seq_cst` fence clock.
    fences: Clock,
    /// Each channel's queued entries: the send node and the clock the
    /// sender carried, oldest first.
    queues: BTreeMap<(String, u64), Vec<(String, Clock)>>,
}

/// Whether the access loads or stores with acquire or release strength
/// (RA-2, RA-3).
fn strength(access: &AccessRecord) -> (bool, bool) {
    use qsl_semantics::check::AccessOrdering::{AcqRel, Acquire, Release, SeqCst};
    let acquires = matches!(access.ordering, Some(Acquire | AcqRel | SeqCst));
    let releases = matches!(access.ordering, Some(Release | AcqRel | SeqCst));
    (acquires, releases)
}

impl Replay {
    fn thread(&mut self, path: &ThreadPath) -> &mut Clocks {
        self.threads.entry(path.clone()).or_default()
    }

    /// The step at one position, and the access event it makes.
    fn step(&mut self, step: &ProtocolTransition) -> Option<Event> {
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
                for child in step.fork_children() {
                    self.threads.insert(child.clone(), copy.clone());
                }
                None
            }
            StepKind::Join => {
                let parent = self.thread(thread).clone();
                let below: Vec<ThreadPath> = self
                    .threads
                    .keys()
                    .filter(|path| *path != thread && thread.is_prefix_of(path))
                    .cloned()
                    .collect();
                let mut joined = parent;
                for path in &below {
                    if let Some(clocks) = self.threads.remove(path) {
                        join(&mut joined.cur, &clocks.cur);
                        join(&mut joined.acq, &clocks.acq);
                        for (location, clock) in clocks.rel {
                            join(joined.rel.entry(location).or_default(), &clock);
                        }
                    }
                }
                joined.tick(thread);
                self.threads.insert(thread.clone(), joined);
                if self.threads.len() == 1 {
                    // An outermost `join`: the next `parallel` starts from
                    // one initial message per location (RA-1, RA-7).
                    self.messages.clear();
                }
                None
            }
            StepKind::Send => {
                let accepted = step
                    .send_record()
                    .is_some_and(|record| record.outcome() == SendOutcome::Accepted);
                let (Some(node), true) = (step.node(), accepted) else {
                    return None;
                };
                let record = step.send_record()?;
                let clocks = self.thread(thread);
                clocks.tick(thread);
                let carried = clocks.cur.clone();
                self.queues
                    .entry((record.channel().to_owned(), thread.instance))
                    .or_default()
                    .push((node.to_owned(), carried));
                None
            }
            StepKind::Receive => {
                let (channel, instance, send) = step.received()?;
                let queue = self.queues.get_mut(&(channel.to_owned(), instance))?;
                let at = queue.iter().position(|(node, _)| node == send)?;
                let (_, carried) = queue.remove(at);
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
    fn fence(&mut self, thread: &ThreadPath, ordering: &str) {
        let acquires = matches!(ordering, "acquire" | "acq_rel" | "seq_cst");
        let releases = matches!(ordering, "release" | "acq_rel" | "seq_cst");
        let locations: Vec<Location> = self.messages.keys().cloned().collect();
        let clocks = self.thread(thread);
        clocks.tick(thread);
        if acquires {
            let acquired = clocks.acq.clone();
            join(&mut clocks.cur, &acquired);
        }
        if releases {
            let current = clocks.cur.clone();
            for location in locations {
                clocks.rel.insert(location, current.clone());
            }
        }
        if ordering == "seq_cst" {
            let mut current = clocks.cur.clone();
            join(&mut current, &self.fences);
            self.fences.clone_from(&current);
            self.thread(thread).cur = current;
        }
    }

    /// RA-2 to RA-4, DR-2: an attempt's shared access.
    fn attempt(&mut self, step: &ProtocolTransition, thread: &ThreadPath) -> Option<Event> {
        let access = step.access()?;
        let choice = step.memory_choice()?;
        let (acquires, releases) = strength(access);
        let location = &access.location;
        let messages = self
            .messages
            .entry(location.clone())
            .or_insert_with(|| vec![Clock::new()]);
        let len = messages.len();
        let read = match choice.read_depth {
            Some(depth) => {
                let at = len.checked_sub(1)?.checked_sub(usize::try_from(depth).ok()?)?;
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
                clocks.rel.get(location).cloned().unwrap_or_default()
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

/// DR-6: whether the accesses at the positions `race` names are by different
/// threads, touch its location, include a write and a non-atomic access, and
/// are unordered by happens-before over `steps`, the replayed prefix.
pub(super) fn reproduces(steps: &[ProtocolTransition], race: &Race) -> bool {
    let mut replay = Replay::default();
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

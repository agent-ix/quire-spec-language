// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-224 "Race summary" (ADR-025 DR-2, DR-3, DR-5; QSpec FR-438): the data
//! race a step completes, and the parts of the race summary a memory model
//! shares.
//!
//! For each shared location `ℓ` with a non-atomic access the memory component
//! holds a [`RaceSummary`]: the thread of `ℓ`'s last write and whether it was
//! atomic, and for each thread whether its last read of `ℓ` was atomic. Each
//! happens-before carrier holds [`RaceBools`] for `ℓ`: whether `ℓ`'s last
//! write happens before it, and, for each thread `c`, whether `c`'s last read
//! of `ℓ` happens before it. A memory model decides a step's race with
//! [`RaceSummary::conflicts`] over the accessing thread's `cur` carrier from
//! its pre-state, then records the access with [`RaceSummary::record`] and
//! the carriers' updates ([`set_write`], [`set_read`], [`clear_write`],
//! [`clear_read`]), and joins carriers with [`join_maps`]. `ra` holds the
//! carriers in its views ([`super::Ra`]); a memory model with other carriers
//! reads the same rules over them.

use std::collections::{BTreeMap, BTreeSet};

use qsl_semantics::check::AccessOrdering;

use super::identity::ThreadPath;
use crate::simulation::footprint::Location;
use crate::simulation::model::encode::{map_enc, Enc};

/// One access of a thread to a shared location, as the race summary reads it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessRecord {
    /// The accessing thread.
    pub thread: ThreadPath,
    /// The shared location.
    pub location: Location,
    /// Whether the access reads the location: a load or the read part of a
    /// read-modify-write.
    pub reads: bool,
    /// Whether the access writes the location: a store or the write part of
    /// a read-modify-write.
    pub writes: bool,
    /// The attempt's `ordering`; `None` is a non-atomic access (ADR-025
    /// MA-2).
    pub ordering: Option<AccessOrdering>,
}

impl AccessRecord {
    /// Whether the access is atomic: it names an `ordering`.
    pub fn atomic(&self) -> bool {
        self.ordering.is_some()
    }

    /// Whether this access is one a [`Conflict`] on its location names: by
    /// the conflict's thread, and a write or a read as the conflict says.
    pub fn is_named_by(&self, race: &DataRace) -> bool {
        self.location == race.location
            && race.earlier.iter().any(|earlier| earlier.is_named_by(self))
    }
}

/// Which earlier access of a thread a race names.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EarlierKind {
    /// The thread's last write.
    Write,
    /// The thread's last read.
    Read,
}

/// One earlier access that does not happen before the step that completes a
/// race: the thread and whether it is the thread's last write or last read.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Conflict {
    /// The earlier access's thread.
    pub thread: ThreadPath,
    /// Whether it is that thread's last write or its last read.
    pub kind: EarlierKind,
}

impl Conflict {
    fn is_named_by(&self, access: &AccessRecord) -> bool {
        access.thread == self.thread
            && match self.kind {
                EarlierKind::Write => access.writes,
                EarlierKind::Read => access.reads,
            }
    }
}

/// The data race a step completes (ADR-025 DR-4): the location, and every
/// last conflicting access, in ascending order, that does not happen before
/// the step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DataRace {
    /// The shared location both accesses touch.
    pub location: Location,
    /// The earlier conflicting accesses the summary names.
    pub earlier: Vec<Conflict>,
}

/// A happens-before carrier's Booleans for one location (ADR-025 DR-3):
/// whether the location's last write happens before the carrier, and the
/// threads whose last read of it does. A thread absent from `reads` is
/// `false`.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RaceBools {
    write: bool,
    reads: BTreeSet<ThreadPath>,
}

impl RaceBools {
    /// These Booleans or-ed with `other`'s (a join of carriers).
    pub(crate) fn join(&mut self, other: &Self) {
        self.write |= other.write;
        self.reads.extend(other.reads.iter().cloned());
    }

    /// Whether every Boolean is false.
    fn is_clear(&self) -> bool {
        !self.write && self.reads.is_empty()
    }

    /// `{"write":<Boolean>,"reads":{<thread>:<Boolean>}}`, the reads over
    /// the threads `summary` holds a last read of.
    pub(crate) fn enc(&self, summary: &RaceSummary) -> Enc {
        Enc::Object(vec![
            ("write", Enc::Bool(self.write)),
            (
                "reads",
                map_enc(
                    summary
                        .last_reads
                        .keys()
                        .map(|thread| (thread.enc(), Enc::Bool(self.reads.contains(thread))))
                        .collect(),
                ),
            ),
        ])
    }
}

/// One carrier's Booleans for every location with a race summary; a location
/// absent from the map has every Boolean false.
pub(crate) type RaceMap = BTreeMap<Location, RaceBools>;

/// `into` or-ed with `from`: where carriers join.
pub(crate) fn join_maps(into: &mut RaceMap, from: &RaceMap) {
    for (location, bools) in from {
        into.entry(location.clone()).or_default().join(bools);
    }
}

fn update(map: &mut RaceMap, location: &Location, change: impl FnOnce(&mut RaceBools)) {
    let bools = map.entry(location.clone()).or_default();
    change(bools);
    if bools.is_clear() {
        map.remove(location);
    }
}

/// A write to `location`: its write Boolean is true in the writer's own
/// carrier `map`.
pub(crate) fn set_write(map: &mut RaceMap, location: &Location) {
    update(map, location, |bools| bools.write = true);
}

/// A write to `location`: its write Boolean is false in every other carrier.
pub(crate) fn clear_write(map: &mut RaceMap, location: &Location) {
    update(map, location, |bools| bools.write = false);
}

/// A read of `location` by `thread`: its read Boolean is true in the
/// reader's own carrier `map`.
pub(crate) fn set_read(map: &mut RaceMap, location: &Location, thread: &ThreadPath) {
    update(map, location, |bools| {
        bools.reads.insert(thread.clone());
    });
}

/// A read of `location` by `thread`: its read Boolean is false in every other
/// carrier.
pub(crate) fn clear_read(map: &mut RaceMap, location: &Location, thread: &ThreadPath) {
    update(map, location, |bools| {
        bools.reads.remove(thread);
    });
}

/// The race summary of one location (ADR-025 DR-3).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct RaceSummary {
    /// The thread of the last write and whether it was atomic.
    last_write: Option<(ThreadPath, bool)>,
    /// For each thread that has read the location, whether its last read was
    /// atomic.
    last_reads: BTreeMap<ThreadPath, bool>,
}

impl RaceSummary {
    /// The earlier accesses `access` races with, given the Booleans of the
    /// accessing thread's `cur` carrier before the step (ADR-025 DR-3,
    /// FastTrack): the last write when it is by another thread, not
    /// ordered before `carrier` and not atomic together with `access`; and
    /// for a write each other thread's last read likewise.
    pub(crate) fn conflicts(&self, access: &AccessRecord, carrier: &RaceBools) -> Vec<Conflict> {
        let atomic = access.atomic();
        let mut found = Vec::new();
        if let Some((writer, writer_atomic)) = &self.last_write {
            if *writer != access.thread && !(atomic && *writer_atomic) && !carrier.write {
                found.push(Conflict {
                    thread: writer.clone(),
                    kind: EarlierKind::Write,
                });
            }
        }
        if access.writes {
            for (reader, reader_atomic) in &self.last_reads {
                if *reader != access.thread
                    && !(atomic && *reader_atomic)
                    && !carrier.reads.contains(reader)
                {
                    found.push(Conflict {
                        thread: reader.clone(),
                        kind: EarlierKind::Read,
                    });
                }
            }
        }
        found.sort();
        found
    }

    /// This summary after `access`.
    pub(crate) fn record(&mut self, access: &AccessRecord) {
        if access.writes {
            self.last_write = Some((access.thread.clone(), access.atomic()));
        }
        if access.reads {
            self.last_reads
                .insert(access.thread.clone(), access.atomic());
        }
    }

    /// `{"last_write":{"thread":<path>,"atomic":<Boolean>}|null,
    /// "last_reads":{<thread>:<Boolean>}}`.
    pub(crate) fn enc(&self) -> Enc {
        Enc::Object(vec![
            (
                "last_write",
                match &self.last_write {
                    Some((thread, atomic)) => Enc::Object(vec![
                        ("thread", thread.enc()),
                        ("atomic", Enc::Bool(*atomic)),
                    ]),
                    None => Enc::Null,
                },
            ),
            (
                "last_reads",
                map_enc(
                    self.last_reads
                        .iter()
                        .map(|(thread, atomic)| (thread.enc(), Enc::Bool(*atomic)))
                        .collect(),
                ),
            ),
        ])
    }
}

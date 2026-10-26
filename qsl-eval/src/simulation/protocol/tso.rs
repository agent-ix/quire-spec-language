// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-221 and FR-229 (ADR-025 TSO-1 to TSO-8, MS-1, MS-2, MS-4, MB-3;
//! QSpec FR-436): [`Tso`], the x86-TSO memory model through the seam.
//!
//! The component is one FIFO store buffer per thread of an active `tso`
//! `parallel`: a sequence of (location, value) pairs in program order. A
//! thread with no buffer, outside every `tso` `parallel`, applies in place,
//! so a subject whose `parallel` controls all resolve to `sc` explores
//! exactly as under [`super::Sc`].
//!
//! - A load reads the newest buffered value of its location, else memory
//!   (the observation, SE-1 b).
//! - A store appends to its thread's buffer and leaves memory unchanged;
//!   a store that would exceed `max_store_buffer` is cut (MB-3).
//! - A read-modify-write, and a `seq_cst` store, is a locked access:
//!   enabled only when its thread's buffer is empty, it writes memory.
//! - `flush(b)` writes the oldest entry of `b`'s buffer to memory.
//! - `fork` and `send` need the thread's buffer empty; `join` needs every
//!   branch's buffer empty; branches start with empty buffers, and a join
//!   removes them.
//! - Atoms read memory: a buffered store is invisible to them.
//!
//! Every ordering but `seq_cst` on a store emits a plain `MOV` or nothing
//! (TSO-8), so it changes no behaviour here.

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::MemoryModelName;
use qsl_semantics::check::{AccessClass, AccessOrdering};
use quire_canonical::{Encode, Error, Sink, Writer};
use quire_exact::Value;

use super::access::location_enc;
use super::fairness::ProtocolFairness;
use super::footprint::ProtocolLocation;
use super::identity::ThreadPath;
use super::memory::{
    buffer_location, Access, CarriedKey, Chosen, FootprintStep, GatedStep, MemoryBranch,
    MemoryModel, MemoryStep, MemoryStepId, ReadChoice, SharedCell, Written,
};
use super::ra::{acquires, releases, CarriedView};
use super::race::{
    clear_read, clear_write, join_maps, set_read, set_write, AccessRecord, DataRace, RaceMap,
    RaceSummary,
};
use crate::simulation::footprint::{Footprint, Location};
use crate::simulation::model::encode::{map_enc, value_enc, Enc};
use crate::simulation::model::{Cell, ModelSimState};

/// The published default of a memory bound (ADR-025 MB-2).
pub const DEFAULT_MEMORY_BOUND: u64 = 4;

/// The name of `tso`'s memory step (ADR-025 TSO-5).
const FLUSH: &str = "flush";

/// FR-221: the x86-TSO memory model, with its store buffer bound
/// (FR-225).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Tso {
    max_store_buffer: u64,
}

impl Tso {
    /// `Tso` with at most `max_store_buffer` entries per buffer.
    pub fn new(max_store_buffer: u64) -> Self {
        Self { max_store_buffer }
    }

    /// The bound it runs under.
    pub fn max_store_buffer(&self) -> u64 {
        self.max_store_buffer
    }

    /// DR-3: the data race `access` completes, and its effect on the race
    /// Booleans in `next`, the component `write_one` left. The race is decided
    /// from the pre-state `component`, then the Booleans are updated, then
    /// the rule of RA-2 to RA-4 applies over the store a load reads or a
    /// store gives its Booleans to.
    fn track(
        &self,
        component: &TsoComponent,
        next: &mut TsoComponent,
        thread: &ThreadPath,
        access: &Access<'_>,
    ) -> Option<DataRace> {
        let shared = access.shared?;
        if !component.race.threads.contains_key(thread) {
            return None;
        }
        let (reads, writes) = match access.class {
            AccessClass::Load => (true, false),
            AccessClass::Store => (false, true),
            AccessClass::Rmw => (true, true),
            AccessClass::Local => return None,
        };
        let record = AccessRecord {
            thread: thread.clone(),
            location: shared.location.clone(),
            reads,
            writes,
            ordering: access.ordering,
        };
        let acquire = acquires(access.ordering);
        let rmw = access.class == AccessClass::Rmw;
        let race = component.race_of(
            &record,
            if rmw && acquire {
                component.race.memory.get(shared.location)
            } else {
                None
            },
        );
        next.note(&record);
        let source = match access.class {
            AccessClass::Load => Some(next.read_source(thread, shared.location)),
            AccessClass::Rmw => next.race.memory.get(shared.location).cloned(),
            AccessClass::Store | AccessClass::Local => None,
        };
        if let Some(source) = &source {
            next.race.read_from(thread, source, acquire);
        }
        if reads && !writes {
            return race;
        }
        let view = next.race.published(
            thread,
            shared.location,
            releases(access.ordering),
            source.as_ref().filter(|_| rmw),
        );
        let direct = locked(access.class, access.ordering) || !next.buffers.contains_key(thread);
        if direct {
            next.race.memory.insert(shared.location.clone(), view);
        } else if let Some(entry) = next
            .buffers
            .get_mut(thread)
            .and_then(|buffer| buffer.last_mut())
        {
            entry.race = view;
        }
        race
    }

    /// TSO-2 and TSO-5: where `thread`'s attempt `access` writes its delta.
    fn write_one(
        &self,
        component: &TsoComponent,
        thread: &ThreadPath,
        access: &Access<'_>,
        pre: &ModelSimState,
        post: ModelSimState,
    ) -> Written<TsoComponent> {
        let Some(buffer) = component.buffers.get(thread) else {
            return Written::Applied(post, component.clone());
        };
        if locked(access.class, access.ordering) {
            if !buffer.is_empty() {
                return Written::Disabled;
            }
            return Written::Applied(post, component.clone());
        }
        match (access.class, access.shared) {
            (AccessClass::Store, Some(shared)) => {
                if u64::try_from(buffer.len()).map_or(true, |held| held >= self.max_store_buffer) {
                    return Written::Cut;
                }
                let value = post.cell(shared.cell).cloned();
                let mut model = unbuffered(pre, post, buffer);
                model.set_cell(shared.cell, pre.cell(shared.cell).cloned());
                let mut next = component.clone();
                if let Some(owned) = next.buffers.get_mut(thread) {
                    owned.push(Buffered {
                        location: shared.location.clone(),
                        cell: shared.cell.clone(),
                        enc: value.as_ref().map_or(Enc::Null, value_enc),
                        value,
                        location_enc: location_enc(shared.location, Some(shared.cell)),
                        race: RaceMap::new(),
                    });
                }
                Written::Applied(model, next)
            }
            _ => Written::Applied(unbuffered(pre, post, buffer), component.clone()),
        }
    }
}

impl Default for Tso {
    /// The published default bound, 4.
    fn default() -> Self {
        Self::new(DEFAULT_MEMORY_BOUND)
    }
}

/// One buffered store: its location, the cell it names, and its value.
#[derive(Clone, Debug)]
pub struct Buffered {
    /// The location.
    pub location: Location,
    /// The cell of the model state it names.
    pub cell: Cell,
    /// The stored value; `None` leaves the field absent.
    pub value: Option<Value>,
    enc: Enc,
    location_enc: Enc,
    /// The race Booleans the store was given (ADR-025 DR-3); empty when the
    /// subject has no non-atomic access.
    race: RaceMap,
}

impl PartialEq for Buffered {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location && self.enc == other.enc && self.race == other.race
    }
}

impl Eq for Buffered {}

/// `Tso`'s component (TSO-1): each live thread of an active `tso`
/// `parallel` and its buffer, oldest entry first.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TsoComponent {
    buffers: BTreeMap<ThreadPath, Vec<Buffered>>,
    /// The race summary of ADR-025 DR-3; empty when the subject has no
    /// non-atomic access.
    race: TsoRace,
}

impl TsoComponent {
    /// The buffer of `thread`; `None` for a thread outside every `tso`
    /// `parallel`.
    pub fn buffer(&self, thread: &ThreadPath) -> Option<&[Buffered]> {
        self.buffers.get(thread).map(Vec::as_slice)
    }

    /// Every thread with a buffer, in path order.
    pub fn threads(&self) -> impl Iterator<Item = &ThreadPath> {
        self.buffers.keys()
    }

    fn enc(&self) -> Enc {
        let racing = !self.race.summaries.is_empty();
        let mut entries: Vec<(Enc, Enc)> = self
            .buffers
            .iter()
            .map(|(thread, buffer)| {
                (
                    Enc::tagged("store-buffer", vec![("thread", thread.enc())]),
                    Enc::tagged(
                        "sequence",
                        vec![(
                            "elements",
                            Enc::Array(
                                buffer
                                    .iter()
                                    .map(|entry| {
                                        let mut members = vec![
                                            ("location", entry.location_enc.clone()),
                                            ("value", entry.enc.clone()),
                                        ];
                                        if racing {
                                            members.push(("race", self.race.map_enc(&entry.race)));
                                        }
                                        Enc::Object(members)
                                    })
                                    .collect(),
                            ),
                        )],
                    ),
                )
            })
            .collect();
        entries.extend(self.race.entries());
        map_enc(entries)
    }

    /// DR-3: `f` over the race Booleans of every happens-before carrier the
    /// component holds: each thread's current, acquire and release
    /// summaries, each buffered store and each memory value.
    fn each_race_map(&mut self, mut f: impl FnMut(&mut RaceMap)) {
        for carriers in self.race.threads.values_mut() {
            f(&mut carriers.cur);
            f(&mut carriers.acq);
            for map in carriers.rel.values_mut() {
                f(map);
            }
        }
        for map in self.race.memory.values_mut() {
            f(map);
        }
        for buffer in self.buffers.values_mut() {
            for entry in buffer {
                f(&mut entry.race);
            }
        }
    }

    /// DR-3 and ADR-025 DR-4: the data race `access` completes, decided from
    /// its pre-state over the accessing thread's current summary, joined
    /// with `read` for an acquire read-modify-write. `None` for a location
    /// with no race summary.
    fn race_of(&self, access: &AccessRecord, read: Option<&RaceMap>) -> Option<DataRace> {
        let summary = self.race.summaries.get(&access.location)?;
        let mut carrier = self
            .race
            .threads
            .get(&access.thread)
            .and_then(|carriers| carriers.cur.get(&access.location))
            .cloned()
            .unwrap_or_default();
        if let Some(bools) = read.and_then(|map| map.get(&access.location)) {
            carrier.join(bools);
        }
        let earlier = summary.conflicts(access, &carrier);
        (!earlier.is_empty()).then(|| DataRace {
            location: access.location.clone(),
            earlier,
        })
    }

    /// DR-3: the summary and every carrier after `access`: a write sets the
    /// location's write Boolean true in the writer's current and acquire
    /// summaries and false in every other carrier, and a read its thread's
    /// read Boolean likewise.
    fn note(&mut self, access: &AccessRecord) {
        let Some(summary) = self.race.summaries.get_mut(&access.location) else {
            return;
        };
        summary.record(access);
        let (location, thread) = (&access.location, &access.thread);
        if access.writes {
            self.each_race_map(|map| clear_write(map, location));
        }
        if access.reads {
            self.each_race_map(|map| clear_read(map, location, thread));
        }
        if let Some(carriers) = self.race.threads.get_mut(thread) {
            for map in [&mut carriers.cur, &mut carriers.acq] {
                if access.writes {
                    set_write(map, location);
                }
                if access.reads {
                    set_read(map, location, thread);
                }
            }
        }
    }

    /// The Booleans of the store a load of `location` by `thread` reads
    /// from: its newest buffered store of the location, else memory.
    fn read_source(&self, thread: &ThreadPath, location: &Location) -> RaceMap {
        self.buffers
            .get(thread)
            .and_then(|buffer| {
                buffer
                    .iter()
                    .rev()
                    .find(|entry| entry.location == *location)
            })
            .map(|entry| entry.race.clone())
            .or_else(|| self.race.memory.get(location).cloned())
            .unwrap_or_default()
    }
}

/// A thread's three summaries (ADR-025 DR-3), which play the part of RA-1's
/// views of the same names for the race Booleans.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct Carriers {
    cur: RaceMap,
    acq: RaceMap,
    rel: BTreeMap<Location, RaceMap>,
}

impl Carriers {
    /// These summaries joined with `other`'s.
    fn join(&mut self, other: &Self) {
        join_maps(&mut self.cur, &other.cur);
        join_maps(&mut self.acq, &other.acq);
        for (location, map) in &other.rel {
            join_maps(self.rel.entry(location.clone()).or_default(), map);
        }
    }
}

/// `tso`'s race summary (ADR-025 DR-3, QSpec FR-438): the summary of each
/// location with a non-atomic access, each live thread's summaries and each
/// memory value's Booleans; buffered stores and queued `send`s carry theirs.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct TsoRace {
    summaries: BTreeMap<Location, RaceSummary>,
    /// The encoding of each shared location, for the state key.
    locations: BTreeMap<Location, Enc>,
    threads: BTreeMap<ThreadPath, Carriers>,
    memory: BTreeMap<Location, RaceMap>,
}

impl TsoRace {
    /// Entering an outermost `tso` `parallel` with a non-atomic access,
    /// nothing is written: every carrier holds the write Boolean of each
    /// location true (QSpec FR-438).
    fn enter(&mut self, parent: &ThreadPath, cells: &[SharedCell<'_>]) {
        if !cells.iter().any(|cell| cell.non_atomic) {
            return;
        }
        for cell in cells {
            self.locations.insert(
                cell.location.clone(),
                location_enc(cell.location, Some(cell.cell)),
            );
            if cell.non_atomic {
                self.summaries
                    .insert(cell.location.clone(), RaceSummary::default());
            }
        }
        let mut tip = RaceMap::new();
        for location in self.summaries.keys() {
            set_write(&mut tip, location);
        }
        for location in self.locations.keys() {
            self.memory.insert(location.clone(), tip.clone());
        }
        let rel = self
            .locations
            .keys()
            .map(|location| (location.clone(), tip.clone()))
            .collect();
        self.threads.insert(
            parent.clone(),
            Carriers {
                cur: tip.clone(),
                acq: tip,
                rel,
            },
        );
    }

    /// A carrier's Booleans for every location with a race summary.
    fn map_enc(&self, race: &RaceMap) -> Enc {
        map_enc(
            self.summaries
                .iter()
                .map(|(location, summary)| {
                    let bools = race.get(location).cloned().unwrap_or_default();
                    (self.location_enc(location), bools.enc(summary))
                })
                .collect(),
        )
    }

    fn location_enc(&self, location: &Location) -> Enc {
        self.locations.get(location).cloned().unwrap_or(Enc::Null)
    }

    /// The state-key entries of the summaries, thread summaries and memory
    /// values.
    fn entries(&self) -> Vec<(Enc, Enc)> {
        let mut entries = Vec::new();
        for (location, summary) in &self.summaries {
            entries.push((
                Enc::tagged(
                    "race-summary",
                    vec![("location", self.location_enc(location))],
                ),
                summary.enc(),
            ));
        }
        for (thread, carriers) in &self.threads {
            entries.push((
                Enc::tagged("race-summaries", vec![("thread", thread.enc())]),
                Enc::Object(vec![
                    ("cur", self.map_enc(&carriers.cur)),
                    ("acq", self.map_enc(&carriers.acq)),
                    (
                        "rel",
                        map_enc(
                            carriers
                                .rel
                                .iter()
                                .map(|(location, map)| {
                                    (self.location_enc(location), self.map_enc(map))
                                })
                                .collect(),
                        ),
                    ),
                ]),
            ));
        }
        for (location, map) in &self.memory {
            entries.push((
                Enc::tagged(
                    "race-memory",
                    vec![("location", self.location_enc(location))],
                ),
                self.map_enc(map),
            ));
        }
        entries
    }

    /// RA-2 for the race Booleans: `thread` reads from a store whose
    /// Booleans are `source`.
    fn read_from(&mut self, thread: &ThreadPath, source: &RaceMap, acquire: bool) {
        if let Some(carriers) = self.threads.get_mut(thread) {
            join_maps(&mut carriers.acq, source);
            if acquire {
                join_maps(&mut carriers.cur, source);
            }
        }
    }

    /// RA-3, RA-4 for the race Booleans: the Booleans a store of `location`
    /// by `thread` is given, joined with those of the store an
    /// read-modify-write reads.
    fn published(
        &mut self,
        thread: &ThreadPath,
        location: &Location,
        release: bool,
        read: Option<&RaceMap>,
    ) -> RaceMap {
        let mut view = match self.threads.get_mut(thread) {
            Some(carriers) if release => {
                carriers.rel.insert(location.clone(), carriers.cur.clone());
                carriers.cur.clone()
            }
            Some(carriers) => carriers.rel.get(location).cloned().unwrap_or_default(),
            None => RaceMap::new(),
        };
        if let Some(read) = read {
            join_maps(&mut view, read);
        }
        view
    }
}

/// QSpec FR-436: the state key's `memory` member maps each
/// `{"type":"store-buffer","thread":<path>}` to its sequence.
impl Encode for TsoComponent {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.enc().encode_into(writer)
    }
}

/// Whether an access is a locked one (TSO-5): a read-modify-write, or a
/// `seq_cst` store.
fn locked(class: AccessClass, ordering: Option<AccessOrdering>) -> bool {
    class == AccessClass::Rmw
        || (class == AccessClass::Store && ordering == Some(AccessOrdering::SeqCst))
}

/// `model` with each of `buffer`'s entries, oldest first, written to its
/// cell, so the newest entry of each location wins.
fn overlay(model: &ModelSimState, buffer: &[Buffered]) -> ModelSimState {
    let mut seen = model.clone();
    for entry in buffer {
        seen.set_cell(&entry.cell, entry.value.clone());
    }
    seen
}

/// `post` with each location of `buffer` back at its memory value in
/// `pre`: what the observation overlaid is not written.
fn unbuffered(pre: &ModelSimState, mut post: ModelSimState, buffer: &[Buffered]) -> ModelSimState {
    for entry in buffer {
        post.set_cell(&entry.cell, pre.cell(&entry.cell).cloned());
    }
    post
}

impl MemoryModel for Tso {
    type Component = TsoComponent;

    fn models(&self) -> &'static [MemoryModelName] {
        &[MemoryModelName::Sc, MemoryModelName::Tso]
    }

    fn initial(&self) -> TsoComponent {
        TsoComponent::default()
    }

    /// TSO-3: memory with the thread's own buffered stores.
    fn observe<'s>(
        &self,
        component: &TsoComponent,
        thread: &ThreadPath,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState> {
        match component.buffers.get(thread) {
            Some(buffer) if !buffer.is_empty() => Cow::Owned(overlay(model, buffer)),
            _ => Cow::Borrowed(model),
        }
    }

    /// TSO-2 and TSO-5.
    fn write(
        &self,
        component: &TsoComponent,
        thread: &ThreadPath,
        access: &Access<'_>,
        _read: Option<ReadChoice>,
        pre: &ModelSimState,
        post: ModelSimState,
    ) -> Vec<Chosen<TsoComponent>> {
        match self.write_one(component, thread, access, pre, post) {
            Written::Applied(model, mut next) => {
                let race = self.track(component, &mut next, thread, access);
                vec![Chosen {
                    choice: None,
                    written: Written::Applied(model, next),
                    race,
                }]
            }
            other => Chosen::only(other),
        }
    }

    /// RA-5 for the race Booleans: an acquire fence joins the acquire
    /// summary into the current one, a release fence sets every release
    /// summary to it. `tso` has no counterpart of `S`.
    fn fence(
        &self,
        component: &TsoComponent,
        thread: &ThreadPath,
        ordering: AccessOrdering,
    ) -> Option<TsoComponent> {
        let mut next = component.clone();
        let TsoRace {
            locations, threads, ..
        } = &mut next.race;
        if let Some(carriers) = threads.get_mut(thread) {
            if acquires(Some(ordering)) {
                let acquired = carriers.acq.clone();
                join_maps(&mut carriers.cur, &acquired);
            }
            if releases(Some(ordering)) {
                for location in locations.keys() {
                    carriers.rel.insert(location.clone(), carriers.cur.clone());
                }
            }
        }
        Some(next)
    }

    /// RA-6 for the race Booleans: the sender's current summary.
    fn send(&self, component: &TsoComponent, thread: &ThreadPath) -> Option<CarriedView> {
        component
            .race
            .threads
            .get(thread)
            .map(|carriers| CarriedView::of_race(carriers.cur.clone()))
    }

    /// RA-6 for the race Booleans: the carried summary joined into the
    /// receiver's current and acquire summaries.
    fn receive(
        &self,
        component: &TsoComponent,
        thread: &ThreadPath,
        carried: &CarriedView,
    ) -> TsoComponent {
        let mut next = component.clone();
        if let Some(carriers) = next.race.threads.get_mut(thread) {
            join_maps(&mut carriers.cur, carried.race());
            join_maps(&mut carriers.acq, carried.race());
        }
        next
    }

    /// DR-3: a summary a queued `send` carries is a happens-before carrier
    /// too.
    fn carried_after(&self, carried: &mut CarriedView, access: &AccessRecord) {
        if access.writes {
            clear_write(carried.race_mut(), &access.location);
        }
        if access.reads {
            clear_read(carried.race_mut(), &access.location, &access.thread);
        }
    }

    fn carried_race_key(
        &self,
        component: &TsoComponent,
        carried: &CarriedView,
    ) -> Option<CarriedKey> {
        (!component.race.summaries.is_empty())
            .then(|| CarriedKey(component.race.map_enc(carried.race())))
    }

    fn carried_key(&self, component: &TsoComponent, carried: &CarriedView) -> CarriedKey {
        CarriedKey(component.race.map_enc(carried.race()))
    }

    /// TSO-4: `flush(b)` for each non-empty buffer.
    fn internal_steps(
        &self,
        component: &TsoComponent,
        model: &ModelSimState,
    ) -> Vec<MemoryStep<TsoComponent>> {
        component
            .buffers
            .iter()
            .filter_map(|(thread, buffer)| {
                let oldest = buffer.first()?;
                let mut written = model.clone();
                written.set_cell(&oldest.cell, oldest.value.clone());
                let mut next = component.clone();
                if let Some(owned) = next.buffers.get_mut(thread) {
                    owned.remove(0);
                }
                // DR-3: the memory value carries the Booleans its store was
                // given.
                if next.race.threads.contains_key(thread) {
                    next.race
                        .memory
                        .insert(oldest.location.clone(), oldest.race.clone());
                }
                Some(MemoryStep {
                    identity: MemoryStepId {
                        model: MemoryModelName::Tso,
                        step: FLUSH,
                        thread: thread.clone(),
                    },
                    component: next,
                    model: written,
                })
            })
            .collect()
    }

    /// TSO-7: `fork` and `send` need the thread's buffer empty, `join`
    /// every buffer below the thread.
    fn gate(&self, component: &TsoComponent, thread: &ThreadPath, step: GatedStep) -> bool {
        let empty = |path: &ThreadPath| component.buffers.get(path).is_none_or(Vec::is_empty);
        match step {
            GatedStep::Fork | GatedStep::Send => empty(thread),
            GatedStep::Join => component
                .buffers
                .iter()
                .filter(|(path, _)| *path != thread && thread.is_prefix_of(path))
                .all(|(_, buffer)| buffer.is_empty()),
            // TSO-6: a `seq_cst` fence is `MFENCE`, enabled once the
            // thread's buffer has drained; any other fence emits nothing.
            GatedStep::Fence(AccessOrdering::SeqCst) => empty(thread),
            GatedStep::Receive | GatedStep::Attempt | GatedStep::Fence(_) => true,
        }
    }

    /// TSO-7: every branch of a `tso` `parallel` starts with an empty
    /// buffer.
    fn split(
        &self,
        component: &TsoComponent,
        parent: &ThreadPath,
        branches: &[ThreadPath],
        model: MemoryModelName,
        shared: &[SharedCell<'_>],
        _state: &ModelSimState,
    ) -> TsoComponent {
        if model != MemoryModelName::Tso {
            return component.clone();
        }
        let mut next = component.clone();
        for branch in branches {
            next.buffers.insert(branch.clone(), Vec::new());
        }
        // DR-3: entering an outermost `tso` `parallel` starts the summaries;
        // each branch starts with its parent's.
        if !next.race.threads.contains_key(parent) {
            next.race.enter(parent, shared);
        }
        if let Some(carriers) = next.race.threads.get(parent).cloned() {
            for branch in branches {
                next.race.threads.insert(branch.clone(), carriers.clone());
            }
        }
        next
    }

    /// TSO-7: a join removes its branches' buffers, which its gate found
    /// empty.
    fn merge(
        &self,
        component: &TsoComponent,
        parent: &ThreadPath,
        branches: &[ThreadPath],
    ) -> TsoComponent {
        let mut next = component.clone();
        next.buffers
            .retain(|path, _| !branches.iter().any(|branch| branch.is_prefix_of(path)));
        // DR-3: the parent's summaries become the join of its branches';
        // after an outermost join nothing is live.
        let mut joined: Option<Carriers> = None;
        for branch in branches {
            let Some(carriers) = next.race.threads.remove(branch) else {
                continue;
            };
            match &mut joined {
                Some(all) => all.join(&carriers),
                None => joined = Some(carriers),
            }
        }
        next.race
            .threads
            .retain(|path, _| !branches.iter().any(|branch| branch.is_prefix_of(path)));
        if let Some(carriers) = joined {
            next.race.threads.insert(parent.clone(), carriers);
        }
        if next.race.threads.keys().all(|path| path == parent) {
            next.race = TsoRace::default();
        }
        next
    }

    /// MS-2: memory; buffered stores are not visible to atoms.
    fn atom_values<'s>(
        &self,
        _component: &TsoComponent,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState> {
        Cow::Borrowed(model)
    }

    /// MF-1: `fair weak each flush(b)` for each thread `b` of each `tso`
    /// `parallel`, one member per branch whose threads it covers.
    fn fairness(&self, branches: &[MemoryBranch]) -> Vec<ProtocolFairness> {
        branches
            .iter()
            .filter(|branch| branch.model == MemoryModelName::Tso)
            .map(|branch| {
                ProtocolFairness::memory_step(
                    MemoryModelName::Tso,
                    FLUSH,
                    branch.parallel.clone(),
                    branch.branch.clone(),
                )
            })
            .collect()
    }

    /// MR-1 under `tso`, for a thread with a buffer:
    ///
    /// - every attempt reads `buf(b)`, which its observation overlays and
    ///   which enables a locked access and a store under the bound; a plain
    ///   store writes `buf(b)` in place of its shared location's `mem(ℓ)`;
    ///   a load and a locked access keep their model reads and writes of
    ///   `mem(ℓ)`;
    /// - `fork`, `send` and a `seq_cst` fence are enabled by `buf(b)`
    ///   (TSO-6);
    /// - `join` is enabled by, and removes, the buffer of every thread
    ///   below it;
    /// - `flush(b)` reads and writes `buf(b)` and writes `mem(ℓ)` of every
    ///   shared location.
    ///
    /// A thread with no buffer applies in place, so its steps keep their
    /// `sc` footprints.
    fn footprint(
        &self,
        component: &TsoComponent,
        thread: &ThreadPath,
        step: FootprintStep<'_>,
        _shared: &BTreeSet<Location>,
        footprint: &mut Footprint,
    ) -> bool {
        let buffered = component.buffers.contains_key(thread);
        let own = buffer_location(thread);
        // MR-1: an access to a location with a race summary reads and writes
        // `race(ℓ)`. Every other step changes a carrier's Booleans of `ℓ` by
        // a copy or an or, which commutes with a write's clear, so only the
        // accesses to `ℓ` and the `flush` of its stores meet it.
        let race = |footprint: &mut Footprint, location: &Location| {
            if component.race.summaries.contains_key(location) {
                let race = Location::Protocol(ProtocolLocation::Race(Box::new(location.clone())));
                footprint.reads.insert(race.clone());
                footprint.enabling.insert(race.clone());
                footprint.writes.insert(race);
            }
        };
        match step {
            FootprintStep::Attempt {
                class,
                ordering,
                shared,
            } => {
                if let Some(location) =
                    shared.filter(|_| component.race.threads.contains_key(thread))
                {
                    race(footprint, location);
                }
                if buffered {
                    footprint.reads.insert(own.clone());
                    footprint.enabling.insert(own.clone());
                    if let (AccessClass::Store, false, Some(shared)) =
                        (class, locked(class, ordering), shared)
                    {
                        footprint.writes.remove(shared);
                        footprint.writes.insert(own);
                    }
                }
                true
            }
            FootprintStep::Gated(
                GatedStep::Fork | GatedStep::Send | GatedStep::Fence(AccessOrdering::SeqCst),
            ) => {
                if buffered {
                    footprint.reads.insert(own.clone());
                    footprint.enabling.insert(own);
                }
                true
            }
            FootprintStep::Gated(GatedStep::Join) => {
                for below in component
                    .buffers
                    .keys()
                    .filter(|path| *path != thread && thread.is_prefix_of(path))
                {
                    let location = buffer_location(below);
                    footprint.reads.insert(location.clone());
                    footprint.enabling.insert(location.clone());
                    footprint.writes.insert(location);
                }
                true
            }
            FootprintStep::Gated(GatedStep::Receive | GatedStep::Attempt | GatedStep::Fence(_)) => {
                true
            }
            // `flush` is the one internal step `tso` takes (TSO-4), so its
            // model and thread identify it.
            FootprintStep::Memory { identity, stored } => {
                if identity.model != MemoryModelName::Tso || identity.thread != *thread || !buffered
                {
                    return false;
                }
                footprint.reads.insert(own.clone());
                footprint.enabling.insert(own.clone());
                footprint.writes.insert(own);
                footprint.writes.extend(stored.iter().cloned());
                for location in stored.iter() {
                    race(footprint, location);
                }
                true
            }
        }
    }
}

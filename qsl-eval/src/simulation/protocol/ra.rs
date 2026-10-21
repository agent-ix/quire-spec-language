// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-222, FR-223 and FR-229 (ADR-025 RA-1 to RA-8, PSC-1 to PSC-6; QSpec
//! FR-437): [`Ra`], the RC11 release-acquire memory model through the seam,
//! with the partial SC order of the `seq_cst` accesses and fences.
//!
//! The component is, for each shared location, its messages in modification
//! order (mo), each with a value, a message view and an attached mark; each
//! thread's current, acquire and release views; the global `seq_cst` fence
//! view `S`; and the SC event graph `P` with its summaries `F(ℓ)`. A thread
//! outside every `ra` `parallel` has no views and applies in place.
//!
//! - A load reads any message at or after its thread's current view; a store
//!   inserts a message at any slot after it, but not directly before an
//!   attached message; a read-modify-write reads a message with no attached
//!   successor and inserts directly after it, attached. Each choice is a
//!   separate transition, named by its depth from the mo-last message.
//! - A fence moves views by its ordering; a `seq_cst` one also joins `S`.
//! - A `seq_cst` access or fence is an SC event: it is enabled with a
//!   message or slot choice only when the graph with its edges stays
//!   acyclic (PSC-3).
//! - `fork` copies the parent's views to each branch; `join` sets the
//!   parent's views to the join of its branches'; a `send` carries the
//!   sender's current view and its `receive` joins it into the receiver's.
//! - After every step, a message earlier in mo than every thread's current
//!   view is removed with the SC events on it (RA-7, PSC-4). After an outermost
//!   `join` the component is empty: each location's model-state cell holds
//!   its mo-last message's value throughout, so atoms read the model state.
//!
//! Messages and events carry ids that only this module reads; the state key
//! names them by depth and rank, so equal futures share one key.
//!
//! The partial SC order built here is `po`, `hb|loc`, `mo` and `rb` between
//! SC accesses, and `po` and `hb` between fences. RC11's `po|≠loc ; hb ;
//! po|≠loc` clause and its fence clause through `eco` are not built: PSC-2's
//! frontier does not decide them, and a `seq_cst` fence's happens-before
//! reach is carried by `S` (RA-5); STD-239. A collected SC event leaves its
//! predecessors to its thread's later events (STD-246), and a thread's older
//! fence leaves when it fences again, which `S` makes sound while fences
//! take only fence-to-fence edges (STD-247). Garbage collection pins a message
//! by the running threads' current views alone, and a store or local access
//! observes its thread's current view (STD-240). The typed form of a view's
//! frontier and of the SC event graph in the key is this module's (STD-241).

use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::MemoryModelName;
use qsl_semantics::check::{AccessClass, AccessOrdering};
use quire_canonical::{Encode, Error, Limits, Sink, Writer};
use quire_exact::Value;
use quire_semantic_value::fairness::FairnessConstraint;

use super::access::location_enc;
use super::footprint::ProtocolLocation;
use super::identity::ThreadPath;
use super::memory::{
    Access, CarriedKey, Chosen, FootprintStep, GatedStep, MemoryChoice, MemoryModel, MemoryStep,
    Read, ReadChoice, SharedCell, Written,
};
use super::tso::DEFAULT_MEMORY_BOUND;
use crate::simulation::footprint::{Footprint, Location};
use crate::simulation::model::encode::{map_enc, value_enc, Enc};
use crate::simulation::model::{Cell, ModelSimState};

/// A message's or an SC event's id: stable inside one state lineage, never
/// part of a key.
type Id = u64;

/// FR-222: the RC11 release-acquire memory model, with its message bound
/// (FR-225).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Ra {
    max_messages: u64,
}

impl Ra {
    /// `Ra` with at most `max_messages` live messages per location.
    pub fn new(max_messages: u64) -> Self {
        Self { max_messages }
    }

    /// The bound it runs under.
    pub fn max_messages(&self) -> u64 {
        self.max_messages
    }
}

impl Default for Ra {
    /// The published default bound, 4 (ADR-025 MB-2).
    fn default() -> Self {
        Self::new(DEFAULT_MEMORY_BOUND)
    }
}

/// One message of a location (RA-1).
#[derive(Clone, Debug)]
struct Message {
    id: Id,
    value: Option<Value>,
    enc: Enc,
    view: View,
    /// An RMW wrote it directly after its predecessor.
    attached: bool,
}

/// A view (RA-1): one message per shared location, with PSC-2's frontier:
/// for each thread, the latest SC event of that thread that happens before
/// the view.
#[derive(Clone, Debug, Default)]
struct View {
    at: BTreeMap<Location, Id>,
    frontier: BTreeMap<ThreadPath, Id>,
}

/// A thread's three views (RA-1).
#[derive(Clone, Debug)]
struct Views {
    cur: View,
    acq: View,
    rel: BTreeMap<Location, View>,
}

/// One shared location: the cell it names in the model state and its
/// messages, oldest first.
#[derive(Clone, Debug)]
struct Shared {
    cell: Cell,
    messages: Vec<Message>,
}

/// What an SC event is (PSC-1).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum EventKind {
    Read,
    Write,
    Rmw,
    Fence,
}

impl EventKind {
    fn text(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Rmw => "rmw",
            Self::Fence => "fence",
        }
    }
}

/// A live SC event: a `seq_cst` access or fence, with its thread, kind,
/// location and the messages it read and wrote (PSC-1).
#[derive(Clone, Debug)]
struct ScEvent {
    thread: ThreadPath,
    kind: EventKind,
    location: Option<Location>,
    read: Option<Id>,
    write: Option<Id>,
}

/// The view a `send` carries with its entry (RA-6).
#[derive(Clone, Debug)]
pub struct CarriedView(View);

/// `Ra`'s component (RA-1, PSC-1).
#[derive(Clone, Debug, Default)]
pub struct RaComponent {
    shared: BTreeMap<Location, Shared>,
    threads: BTreeMap<ThreadPath, Views>,
    /// `S`, the global `seq_cst` fence view.
    sc: View,
    events: BTreeMap<Id, ScEvent>,
    /// `P`: `(e, x)` when `e` precedes `x` in `psc`, transitively closed.
    order: BTreeSet<(Id, Id)>,
    /// `F(ℓ)`: the live SC events that precede every SC write to `ℓ` still
    /// to come.
    fronts: BTreeMap<Location, BTreeSet<Id>>,
    /// The live SC events that precede every later SC event of a thread,
    /// whatever its location: what a removed event of that thread owed its
    /// thread's later events (PSC-4; STD-246).
    owed: BTreeMap<ThreadPath, BTreeSet<Id>>,
    next: Id,
}

impl PartialEq for RaComponent {
    fn eq(&self, other: &Self) -> bool {
        self.enc() == other.enc()
    }
}

impl Eq for RaComponent {}

/// QSpec FR-437: the state key's `memory` member maps each
/// `{"type":"messages","location":<location>}` to its sequence, each
/// `{"type":"views","thread":<path>}` to its three views, `{"type":
/// "sc-view"}` to `S` and `{"type":"sc-graph"}` to the SC event graph.
impl Encode for RaComponent {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.enc().encode_into(writer)
    }
}

/// `values` as an array in ascending canonical bytes; one that does not
/// encode makes the array unencodable, so the key refuses.
fn sorted(values: Vec<Enc>) -> Enc {
    let limits = Limits::new(u64::MAX);
    let mut keyed = Vec::with_capacity(values.len());
    for value in values {
        match quire_canonical::to_vec(&value, limits) {
            Ok(bytes) => keyed.push((bytes, value)),
            Err(_) => return Enc::Unencodable("an SC event or edge"),
        }
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    Enc::Array(keyed.into_iter().map(|(_, value)| value).collect())
}

/// The depth of position `position` in a sequence of `len` messages.
fn depth(len: usize, position: usize) -> u64 {
    u64::try_from(len.saturating_sub(position + 1)).unwrap_or(u64::MAX)
}

/// Whether the access loads with acquire strength (RA-2).
fn acquires(ordering: Option<AccessOrdering>) -> bool {
    matches!(
        ordering,
        Some(AccessOrdering::Acquire | AccessOrdering::AcqRel | AccessOrdering::SeqCst)
    )
}

/// Whether the access stores with release strength (RA-3).
fn releases(ordering: Option<AccessOrdering>) -> bool {
    matches!(
        ordering,
        Some(AccessOrdering::Release | AccessOrdering::AcqRel | AccessOrdering::SeqCst)
    )
}

/// The position of message `id` in `location`'s sequence.
fn position(shared: &BTreeMap<Location, Shared>, location: &Location, id: Id) -> Option<usize> {
    shared
        .get(location)?
        .messages
        .iter()
        .position(|message| message.id == id)
}

/// `into` joined with `from`: for each location the later message, for each
/// thread the later SC event. A message no longer live is skipped.
fn join(shared: &BTreeMap<Location, Shared>, into: &mut View, from: &View) {
    for (location, id) in &from.at {
        let Some(theirs) = position(shared, location, *id) else {
            continue;
        };
        let ahead = into
            .at
            .get(location)
            .and_then(|mine| position(shared, location, *mine))
            .is_none_or(|mine| theirs > mine);
        if ahead {
            into.at.insert(location.clone(), *id);
        }
    }
    for (thread, id) in &from.frontier {
        let latest = into.frontier.entry(thread.clone()).or_insert(*id);
        if *latest < *id {
            *latest = *id;
        }
    }
}

impl Views {
    /// These views joined with `other`'s.
    fn join(&mut self, shared: &BTreeMap<Location, Shared>, other: &Self) {
        join(shared, &mut self.cur, &other.cur);
        join(shared, &mut self.acq, &other.acq);
        for (location, view) in &other.rel {
            join(shared, self.rel.entry(location.clone()).or_default(), view);
        }
    }
}

impl RaComponent {
    /// Whether no weak `parallel` is active: nothing is live.
    pub fn is_empty(&self) -> bool {
        self.shared.is_empty()
    }

    fn held(&self, cell: &Cell) -> Option<(&Location, &Shared)> {
        self.shared.iter().find(|(_, shared)| shared.cell == *cell)
    }

    /// The values of `cell`'s messages in mo, oldest first.
    pub fn values(&self, cell: &Cell) -> Vec<Option<&Value>> {
        self.held(cell)
            .map(|(_, shared)| {
                shared
                    .messages
                    .iter()
                    .map(|message| message.value.as_ref())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The position in `cell`'s sequence of the message `thread`'s current
    /// view points at.
    pub fn current(&self, thread: &ThreadPath, cell: &Cell) -> Option<usize> {
        let (location, _) = self.held(cell)?;
        let id = self.threads.get(thread)?.cur.at.get(location)?;
        self.position(location, *id)
    }

    /// The position in `target`'s sequence that the view of message `at` of
    /// `cell` points at.
    pub fn message_view(&self, cell: &Cell, at: usize, target: &Cell) -> Option<usize> {
        let (_, shared) = self.held(cell)?;
        let (location, _) = self.held(target)?;
        let id = shared.messages.get(at)?.view.at.get(location)?;
        self.position(location, *id)
    }

    /// The position in `cell`'s sequence each running thread's current view
    /// points at.
    pub fn currents(&self, cell: &Cell) -> Vec<usize> {
        let Some((location, _)) = self.held(cell) else {
            return Vec::new();
        };
        self.running()
            .filter_map(|views| self.position(location, *views.cur.at.get(location)?))
            .collect()
    }

    /// The number of live messages over every location.
    pub fn live_messages(&self) -> usize {
        self.shared
            .values()
            .map(|shared| shared.messages.len())
            .sum()
    }

    /// The number of live threads.
    pub fn live_threads(&self) -> usize {
        self.threads.len()
    }

    /// The number of live SC events in `P` (PSC-1).
    pub fn live_events(&self) -> usize {
        self.events.len()
    }

    /// Whether every live SC event is on a live message, and every edge of
    /// `P` joins live events (PSC-4).
    pub fn events_are_live(&self) -> bool {
        let live = |location: &Option<Location>, message: Option<Id>| match (location, message) {
            (Some(location), Some(id)) => self.position(location, id).is_some(),
            (_, None) => true,
            (None, Some(_)) => false,
        };
        self.events
            .values()
            .all(|event| live(&event.location, event.read) && live(&event.location, event.write))
            && self
                .order
                .iter()
                .all(|(a, b)| self.events.contains_key(a) && self.events.contains_key(b))
    }

    fn position(&self, location: &Location, id: Id) -> Option<usize> {
        position(&self.shared, location, id)
    }

    /// Whether position of `earlier` is known and before `later`'s at
    /// `location`.
    fn before(&self, location: &Location, earlier: Option<Id>, later: Option<Id>) -> bool {
        match (
            earlier.and_then(|id| self.position(location, id)),
            later.and_then(|id| self.position(location, id)),
        ) {
            (Some(earlier), Some(later)) => earlier < later,
            _ => false,
        }
    }

    /// The view pointing at every location's mo-last message.
    fn tip(&self) -> View {
        View {
            at: self
                .shared
                .iter()
                .filter_map(|(location, shared)| {
                    Some((location.clone(), shared.messages.last()?.id))
                })
                .collect(),
            frontier: BTreeMap::new(),
        }
    }

    fn fresh(&mut self) -> Id {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }

    /// RA-1: entering an outermost `ra` `parallel`, each location holds one
    /// message, its current value, with every view pointing at it.
    fn enter(&mut self, parent: &ThreadPath, cells: &[SharedCell<'_>], state: &ModelSimState) {
        for cell in cells {
            if self.shared.contains_key(cell.location) {
                continue;
            }
            let id = self.fresh();
            let value = state.cell(cell.cell).cloned();
            self.shared.insert(
                cell.location.clone(),
                Shared {
                    cell: cell.cell.clone(),
                    messages: vec![Message {
                        id,
                        enc: value.as_ref().map_or(Enc::Null, value_enc),
                        value,
                        view: View::default(),
                        attached: false,
                    }],
                },
            );
        }
        let tip = self.tip();
        for shared in self.shared.values_mut() {
            for message in &mut shared.messages {
                if message.view.at.is_empty() {
                    message.view = tip.clone();
                }
            }
        }
        let rel = tip
            .at
            .keys()
            .map(|location| (location.clone(), tip.clone()))
            .collect();
        self.threads.insert(
            parent.clone(),
            Views {
                cur: tip.clone(),
                acq: tip.clone(),
                rel,
            },
        );
        if self.sc.at.is_empty() {
            self.sc = tip;
        }
    }

    /// The component with nothing live, keeping the id counter so an id
    /// never names two things.
    fn clear(&mut self) {
        *self = Self {
            next: self.next,
            ..Self::default()
        };
    }

    /// The model state `thread` sees: each shared cell holding the message
    /// its current view points at (FR-229 "State and observation").
    fn probe<'s>(&self, thread: &ThreadPath, model: &'s ModelSimState) -> Cow<'s, ModelSimState> {
        let Some(views) = self.threads.get(thread) else {
            return Cow::Borrowed(model);
        };
        let mut seen = model.clone();
        for (location, shared) in &self.shared {
            let seen_value = views
                .cur
                .at
                .get(location)
                .and_then(|id| shared.messages.iter().find(|message| message.id == *id));
            if let Some(message) = seen_value {
                seen.set_cell(&shared.cell, message.value.clone());
            }
        }
        Cow::Owned(seen)
    }

    /// `post` with every shared cell but `except` back at its mo-last value
    /// in `pre`: what an observation overlaid is not written.
    fn restored(
        &self,
        pre: &ModelSimState,
        mut post: ModelSimState,
        except: Option<&Cell>,
    ) -> ModelSimState {
        for shared in self.shared.values() {
            if Some(&shared.cell) != except {
                post.set_cell(&shared.cell, pre.cell(&shared.cell).cloned());
            }
        }
        post
    }

    /// RA-2: `thread` reads message `id` of `location`.
    fn read_from(&mut self, thread: &ThreadPath, location: &Location, id: Id, acquire: bool) {
        let Some(view) = self
            .shared
            .get(location)
            .and_then(|shared| shared.messages.iter().find(|message| message.id == id))
            .map(|message| message.view.clone())
        else {
            return;
        };
        let Some(views) = self.threads.get_mut(thread) else {
            return;
        };
        views.cur.at.insert(location.clone(), id);
        join(&self.shared, &mut views.acq, &view);
        if acquire {
            join(&self.shared, &mut views.cur, &view);
        }
    }

    /// RA-3: insert a message at `slot` of `location`'s sequence.
    fn insert(
        &mut self,
        location: &Location,
        slot: usize,
        value: Option<Value>,
        attached: bool,
    ) -> Id {
        let id = self.fresh();
        if let Some(shared) = self.shared.get_mut(location) {
            shared.messages.insert(
                slot,
                Message {
                    id,
                    enc: value.as_ref().map_or(Enc::Null, value_enc),
                    value,
                    view: View::default(),
                    attached,
                },
            );
        }
        id
    }

    /// RA-3, RA-4: `thread` has written message `id` of `location`: its
    /// current view moves to it and the message gets its view, joined with
    /// the view of the message it read for an RMW.
    fn publish(
        &mut self,
        thread: &ThreadPath,
        location: &Location,
        id: Id,
        release: bool,
        read: Option<&View>,
    ) {
        let Some(views) = self.threads.get_mut(thread) else {
            return;
        };
        views.cur.at.insert(location.clone(), id);
        let mut view = if release {
            views.rel.insert(location.clone(), views.cur.clone());
            views.cur.clone()
        } else {
            let mut view = views.rel.get(location).cloned().unwrap_or_default();
            let written = View {
                at: BTreeMap::from([(location.clone(), id)]),
                frontier: BTreeMap::new(),
            };
            join(&self.shared, &mut view, &written);
            view
        };
        if let Some(read) = read {
            join(&self.shared, &mut view, read);
        }
        if let Some(message) = self
            .shared
            .get_mut(location)
            .and_then(|shared| shared.messages.iter_mut().find(|message| message.id == id))
        {
            message.view = view;
        }
    }

    /// PSC-3: `thread` executes an SC access or fence. `false` when its
    /// edges would close a cycle in `P`, so the choice is pruned; else the
    /// event joins `P`, the thread's frontier names it, and the events it
    /// supersedes leave.
    fn sc_event(
        &mut self,
        thread: &ThreadPath,
        kind: EventKind,
        location: Option<&Location>,
        read: Option<Id>,
        write: Option<Id>,
    ) -> bool {
        let Some(views) = self.threads.get(thread) else {
            return true;
        };
        let mut before = BTreeSet::new();
        let mut after = BTreeSet::new();
        for (id, event) in &self.events {
            let program_order = event.thread == *thread;
            let happens_before = views
                .cur
                .frontier
                .get(&event.thread)
                .is_some_and(|latest| id <= latest);
            let Some(at) = location else {
                if program_order || (happens_before && event.kind == EventKind::Fence) {
                    before.insert(*id);
                }
                continue;
            };
            if program_order || (happens_before && event.location.as_ref() == Some(at)) {
                before.insert(*id);
            }
            if event.location.as_ref() != Some(at) {
                continue;
            }
            if write.is_some()
                && (self.before(at, event.write, write) || self.before(at, event.read, write))
            {
                before.insert(*id);
            }
            if event.write.is_some()
                && (self.before(at, write, event.write) || self.before(at, read, event.write))
            {
                after.insert(*id);
            }
        }
        if write.is_some() {
            if let Some(summary) = location.and_then(|at| self.fronts.get(at)) {
                before.extend(summary.iter().copied());
            }
        }
        if let Some(owed) = self.owed.get(thread) {
            before.extend(owed.iter().copied());
        }
        let cycle = before.iter().any(|e| {
            after
                .iter()
                .any(|s| e == s || self.order.contains(&(*s, *e)))
        });
        if cycle {
            return false;
        }
        let id = self.fresh();
        let ancestors: BTreeSet<Id> = std::iter::once(id)
            .chain(before.iter().copied())
            .chain(
                self.order
                    .iter()
                    .filter(|(_, b)| before.contains(b))
                    .map(|(a, _)| *a),
            )
            .collect();
        let descendants: BTreeSet<Id> = std::iter::once(id)
            .chain(after.iter().copied())
            .chain(
                self.order
                    .iter()
                    .filter(|(a, _)| after.contains(a))
                    .map(|(_, b)| *b),
            )
            .collect();
        for a in &ancestors {
            for d in &descendants {
                if a != d {
                    self.order.insert((*a, *d));
                }
            }
        }
        // A fence another fence of its thread follows leaves; `S` carried its
        // frontier into every later fence, which takes its edge from the newer
        // one (STD-247).
        let superseded: BTreeSet<Id> = self
            .events
            .iter()
            .filter(|(_, old)| {
                old.thread == *thread
                    && old.kind == kind
                    && old.location.as_ref() == location
                    && match kind {
                        EventKind::Fence => true,
                        EventKind::Read => old.read == read,
                        EventKind::Write | EventKind::Rmw => false,
                    }
            })
            .map(|(old, _)| *old)
            .collect();
        self.events.insert(
            id,
            ScEvent {
                thread: thread.clone(),
                kind,
                location: location.cloned(),
                read,
                write,
            },
        );
        if let Some(views) = self.threads.get_mut(thread) {
            views.cur.frontier.insert(thread.clone(), id);
        }
        self.drop_events(&superseded, false);
        true
    }

    /// PSC-4: the events `ids` leave `P`, which keeps its transitive
    /// closure. With `summarise`, a removed read or write's predecessors
    /// join `F(ℓ)`, as every later write to `ℓ` is mo-after or fr-after it,
    /// and its thread's `owed` set, as every later event of the thread is
    /// after it in `po`;
    /// an event of some `F(ℓ)` is replaced there by its predecessors.
    fn drop_events(&mut self, ids: &BTreeSet<Id>, summarise: bool) {
        for id in ids {
            let Some(event) = self.events.get(id) else {
                continue;
            };
            let predecessors: BTreeSet<Id> = self
                .order
                .iter()
                .filter(|(a, b)| b == id && !ids.contains(a))
                .map(|(a, _)| *a)
                .collect();
            if summarise {
                if let Some(location) = &event.location {
                    self.fronts
                        .entry(location.clone())
                        .or_default()
                        .extend(predecessors.iter().copied());
                }
                self.owed
                    .entry(event.thread.clone())
                    .or_default()
                    .extend(predecessors.iter().copied());
            }
            for summary in self.fronts.values_mut().chain(self.owed.values_mut()) {
                if summary.remove(id) {
                    summary.extend(predecessors.iter().copied());
                }
            }
        }
        for id in ids {
            self.events.remove(id);
        }
        self.order
            .retain(|(a, b)| !ids.contains(a) && !ids.contains(b));
        self.fronts.retain(|_, summary| !summary.is_empty());
        self.owed.retain(|_, summary| !summary.is_empty());
    }

    /// The views of the threads that can still step: a thread waiting at a
    /// `join` takes its branches' joined views, which are above its own.
    fn running(&self) -> impl Iterator<Item = &Views> {
        self.threads
            .iter()
            .filter(|(path, _)| {
                !self
                    .threads
                    .keys()
                    .any(|other| other != *path && path.is_prefix_of(other))
            })
            .map(|(_, views)| views)
    }

    /// RA-7: the first position of each location some running thread's
    /// current view points at. A message before it cannot be read, stored after or
    /// reached by joining a view: a pointer into it, in any other view or
    /// message view, is below every current view and joins as nothing.
    fn kept_from(&self) -> BTreeMap<Location, usize> {
        self.shared
            .iter()
            .map(|(location, shared)| {
                let first = self
                    .running()
                    .map(|views| {
                        views
                            .cur
                            .at
                            .get(location)
                            .and_then(|id| self.position(location, *id))
                            .unwrap_or(0)
                    })
                    .min()
                    .unwrap_or_else(|| shared.messages.len().saturating_sub(1));
                (location.clone(), first)
            })
            .collect()
    }

    /// RA-7: remove every message earlier than every current view, with the
    /// SC events on it (PSC-4).
    fn collect(&mut self) {
        if self.threads.is_empty() {
            return;
        }
        let from = self.kept_from();
        let mut removed = BTreeSet::new();
        for (location, shared) in &mut self.shared {
            let first = from.get(location).copied().unwrap_or(0);
            removed.extend(shared.messages.drain(..first).map(|message| message.id));
            if let Some(oldest) = shared.messages.first_mut() {
                oldest.attached = false;
            }
        }
        if removed.is_empty() {
            return;
        }
        let events: BTreeSet<Id> = self
            .events
            .iter()
            .filter(|(_, event)| {
                event.read.is_some_and(|id| removed.contains(&id))
                    || event.write.is_some_and(|id| removed.contains(&id))
            })
            .map(|(id, _)| *id)
            .collect();
        self.drop_events(&events, true);
    }

    /// RA-5: `thread`'s fence of `ordering`; `None` when its SC event would
    /// close a cycle in `P`.
    fn fenced(&self, thread: &ThreadPath, ordering: AccessOrdering) -> Option<Self> {
        let mut next = self.clone();
        let Some(mut views) = next.threads.remove(thread) else {
            return Some(next);
        };
        if acquires(Some(ordering)) {
            let acquired = views.acq.clone();
            join(&next.shared, &mut views.cur, &acquired);
        }
        if releases(Some(ordering)) {
            for view in views.rel.values_mut() {
                *view = views.cur.clone();
            }
        }
        next.threads.insert(thread.clone(), views);
        if ordering == AccessOrdering::SeqCst {
            let fence_view = next.sc.clone();
            if let Some(views) = next.threads.get_mut(thread) {
                join(&next.shared, &mut views.cur, &fence_view);
            }
            if !next.sc_event(thread, EventKind::Fence, None, None, None) {
                return None;
            }
            if let Some(views) = next.threads.get(thread) {
                next.sc = views.cur.clone();
            }
        }
        next.collect();
        Some(next)
    }

    fn location_enc(&self, location: &Location) -> Enc {
        location_enc(
            location,
            self.shared.get(location).map(|shared| &shared.cell),
        )
    }

    /// A view as a map from location to message depth, and its frontier as
    /// the number of live events of each thread it covers.
    fn view_enc(&self, view: &View) -> Enc {
        let at = view
            .at
            .iter()
            .filter_map(|(location, id)| {
                let len = self.shared.get(location)?.messages.len();
                let at = self.position(location, *id)?;
                Some((
                    self.location_enc(location),
                    Enc::text(depth(len, at).to_string()),
                ))
            })
            .collect();
        let frontier = view
            .frontier
            .iter()
            .filter_map(|(thread, latest)| {
                let covered = self
                    .events
                    .iter()
                    .filter(|(id, event)| event.thread == *thread && *id <= latest)
                    .count();
                (covered > 0).then(|| (thread.enc(), Enc::text(covered.to_string())))
            })
            .collect();
        Enc::tagged(
            "view",
            vec![("at", map_enc(at)), ("frontier", map_enc(frontier))],
        )
    }

    /// An SC event named by its thread, kind, location and the depths of
    /// the messages it read and wrote (PSC-5).
    fn event_enc(&self, id: Id) -> Enc {
        let Some(event) = self.events.get(&id) else {
            return Enc::Null;
        };
        let named = |message: Option<Id>| {
            let location = event.location.as_ref()?;
            let len = self.shared.get(location)?.messages.len();
            let at = self.position(location, message?)?;
            Some(Enc::text(depth(len, at).to_string()))
        };
        let mut members = vec![
            ("thread", event.thread.enc()),
            ("kind", Enc::text(event.kind.text())),
        ];
        if let Some(location) = &event.location {
            members.push(("location", self.location_enc(location)));
        }
        if let Some(read) = named(event.read) {
            members.push(("read", read));
        }
        if let Some(write) = named(event.write) {
            members.push(("write", write));
        }
        Enc::Object(members)
    }

    fn graph_enc(&self) -> Enc {
        Enc::Object(vec![
            (
                "events",
                sorted(self.events.keys().map(|id| self.event_enc(*id)).collect()),
            ),
            (
                "edges",
                sorted(
                    self.order
                        .iter()
                        .map(|(a, b)| Enc::Array(vec![self.event_enc(*a), self.event_enc(*b)]))
                        .collect(),
                ),
            ),
            (
                "owed",
                map_enc(
                    self.owed
                        .iter()
                        .map(|(thread, events)| {
                            (
                                thread.enc(),
                                sorted(events.iter().map(|id| self.event_enc(*id)).collect()),
                            )
                        })
                        .collect(),
                ),
            ),
            (
                "summaries",
                map_enc(
                    self.fronts
                        .iter()
                        .map(|(location, events)| {
                            (
                                self.location_enc(location),
                                sorted(events.iter().map(|id| self.event_enc(*id)).collect()),
                            )
                        })
                        .collect(),
                ),
            ),
        ])
    }

    fn enc(&self) -> Enc {
        if self.shared.is_empty() {
            return map_enc(Vec::new());
        }
        let mut entries = Vec::new();
        for (location, shared) in &self.shared {
            entries.push((
                Enc::tagged("messages", vec![("location", self.location_enc(location))]),
                Enc::tagged(
                    "sequence",
                    vec![(
                        "elements",
                        Enc::Array(
                            shared
                                .messages
                                .iter()
                                .map(|message| {
                                    Enc::Object(vec![
                                        ("value", message.enc.clone()),
                                        ("view", self.view_enc(&message.view)),
                                        ("attached", Enc::Bool(message.attached)),
                                    ])
                                })
                                .collect(),
                        ),
                    )],
                ),
            ));
        }
        for (thread, views) in &self.threads {
            entries.push((
                Enc::tagged("views", vec![("thread", thread.enc())]),
                Enc::Object(vec![
                    ("cur", self.view_enc(&views.cur)),
                    ("acq", self.view_enc(&views.acq)),
                    (
                        "rel",
                        map_enc(
                            views
                                .rel
                                .iter()
                                .map(|(location, view)| {
                                    (self.location_enc(location), self.view_enc(view))
                                })
                                .collect(),
                        ),
                    ),
                ]),
            ));
        }
        entries.push((Enc::tagged("sc-view", Vec::new()), self.view_enc(&self.sc)));
        entries.push((Enc::tagged("sc-graph", Vec::new()), self.graph_enc()));
        map_enc(entries)
    }

    /// The position of the message `depth` from `location`'s mo-last.
    fn depth_position(&self, location: &Location, depth: u64) -> Option<usize> {
        let len = self.shared.get(location)?.messages.len();
        len.checked_sub(1)?
            .checked_sub(usize::try_from(depth).ok()?)
    }
}

/// An access of one thread to one shared location, from the model state
/// `pre` to `post` (SE-1 c).
struct Delta<'a> {
    thread: &'a ThreadPath,
    ordering: Option<AccessOrdering>,
    shared: SharedCell<'a>,
    pre: &'a ModelSimState,
    post: &'a ModelSimState,
}

impl Ra {
    /// Whether `location` holds more live messages than the bound allows
    /// once `component` has collected (MB-2, MB-3).
    fn exceeds(&self, component: &RaComponent, location: &Location) -> bool {
        let live = component
            .shared
            .get(location)
            .map_or(0, |held| held.messages.len());
        u64::try_from(live).map_or(true, |live| live > self.max_messages)
    }

    /// RA-2: a load by `delta`'s thread reading the message at `read`.
    fn load(
        &self,
        component: &RaComponent,
        delta: &Delta<'_>,
        read: ReadChoice,
    ) -> Vec<Chosen<RaComponent>> {
        let Delta {
            thread,
            ordering,
            shared,
            pre,
            post,
        } = delta;
        let location = shared.location;
        let Some(at) = component.depth_position(location, read.depth) else {
            return Vec::new();
        };
        let Some(id) = component
            .shared
            .get(location)
            .and_then(|held| held.messages.get(at))
            .map(|message| message.id)
        else {
            return Vec::new();
        };
        let mut next = component.clone();
        next.read_from(thread, location, id, acquires(*ordering));
        if *ordering == Some(AccessOrdering::SeqCst)
            && !next.sc_event(thread, EventKind::Read, Some(location), Some(id), None)
        {
            return Vec::new();
        }
        next.collect();
        vec![Chosen {
            choice: Some(MemoryChoice {
                read_depth: Some(read.depth),
                slot_depth: None,
            }),
            written: Written::Applied(
                component.restored(pre, ModelSimState::clone(post), None),
                next,
            ),
        }]
    }

    /// RA-3: a store by `delta`'s thread, at each slot it may take.
    fn store(&self, component: &RaComponent, delta: &Delta<'_>) -> Vec<Chosen<RaComponent>> {
        let Delta {
            thread,
            ordering,
            shared,
            pre,
            post,
        } = delta;
        let location = shared.location;
        let (Some(held), Some(views)) = (
            component.shared.get(location),
            component.threads.get(thread),
        ) else {
            return Vec::new();
        };
        let len = held.messages.len();
        let first = views
            .cur
            .at
            .get(location)
            .and_then(|id| component.position(location, *id))
            .map_or(0, |at| at + 1);
        let value = post.cell(shared.cell).cloned();
        let mut chosen = Vec::new();
        let mut cut = false;
        for slot in first..=len {
            if held
                .messages
                .get(slot)
                .is_some_and(|message| message.attached)
            {
                continue;
            }
            let mut next = component.clone();
            let id = next.insert(location, slot, value.clone(), false);
            if *ordering == Some(AccessOrdering::SeqCst)
                && !next.sc_event(thread, EventKind::Write, Some(location), None, Some(id))
            {
                continue;
            }
            next.publish(thread, location, id, releases(*ordering), None);
            next.collect();
            if self.exceeds(&next, location) {
                cut = true;
                continue;
            }
            let kept = (slot == len).then_some(shared.cell);
            chosen.push(Chosen {
                choice: Some(MemoryChoice {
                    read_depth: None,
                    slot_depth: Some(depth_of_slot(len, slot)),
                }),
                written: Written::Applied(
                    component.restored(pre, ModelSimState::clone(post), kept),
                    next,
                ),
            });
        }
        if cut {
            chosen.push(Chosen {
                choice: None,
                written: Written::Cut,
            });
        }
        chosen
    }

    /// RA-4: a read-modify-write by `delta`'s thread reading the message at
    /// `read` and inserting directly after it.
    fn read_modify_write(
        &self,
        component: &RaComponent,
        delta: &Delta<'_>,
        read: ReadChoice,
    ) -> Vec<Chosen<RaComponent>> {
        let Delta {
            thread,
            ordering,
            shared,
            pre,
            post,
        } = delta;
        let location = shared.location;
        let Some(held) = component.shared.get(location) else {
            return Vec::new();
        };
        let len = held.messages.len();
        let Some(at) = component.depth_position(location, read.depth) else {
            return Vec::new();
        };
        if held
            .messages
            .get(at + 1)
            .is_some_and(|message| message.attached)
        {
            return Vec::new();
        }
        let Some(read_message) = held.messages.get(at) else {
            return Vec::new();
        };
        let (id, read_view) = (read_message.id, read_message.view.clone());
        let mut next = component.clone();
        next.read_from(thread, location, id, acquires(*ordering));
        let slot = at + 1;
        let written = next.insert(location, slot, post.cell(shared.cell).cloned(), true);
        if *ordering == Some(AccessOrdering::SeqCst)
            && !next.sc_event(
                thread,
                EventKind::Rmw,
                Some(location),
                Some(id),
                Some(written),
            )
        {
            return Vec::new();
        }
        next.publish(
            thread,
            location,
            written,
            releases(*ordering),
            Some(&read_view),
        );
        next.collect();
        if self.exceeds(&next, location) {
            return Chosen::only(Written::Cut);
        }
        let kept = (slot == len).then_some(shared.cell);
        vec![Chosen {
            choice: Some(MemoryChoice {
                read_depth: Some(read.depth),
                slot_depth: Some(depth_of_slot(len, slot)),
            }),
            written: Written::Applied(
                component.restored(pre, ModelSimState::clone(post), kept),
                next,
            ),
        }]
    }
}

/// The depth of the slot at `slot` in a sequence of `len` messages: the
/// messages after it.
fn depth_of_slot(len: usize, slot: usize) -> u64 {
    u64::try_from(len.saturating_sub(slot)).unwrap_or(u64::MAX)
}

impl MemoryModel for Ra {
    type Component = RaComponent;

    fn models(&self) -> &'static [MemoryModelName] {
        &[MemoryModelName::Sc, MemoryModelName::Ra]
    }

    fn initial(&self) -> RaComponent {
        RaComponent::default()
    }

    /// FR-229: the model state with each shared cell holding the message
    /// the thread's current view points at.
    fn observe<'s>(
        &self,
        component: &RaComponent,
        thread: &ThreadPath,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState> {
        component.probe(thread, model)
    }

    /// RA-2: one observation per message at or after the thread's current
    /// view.
    fn reads<'s>(
        &self,
        component: &RaComponent,
        thread: &ThreadPath,
        model: &'s ModelSimState,
        shared: &SharedCell<'_>,
    ) -> Vec<Read<'s>> {
        let (Some(views), Some(held)) = (
            component.threads.get(thread),
            component.shared.get(shared.location),
        ) else {
            return Vec::new();
        };
        let first = views
            .cur
            .at
            .get(shared.location)
            .and_then(|id| component.position(shared.location, *id))
            .unwrap_or(0);
        let base = component.probe(thread, model).into_owned();
        let len = held.messages.len();
        held.messages
            .iter()
            .enumerate()
            .skip(first)
            .map(|(at, message)| {
                let mut observed = base.clone();
                observed.set_cell(&held.cell, message.value.clone());
                Read {
                    choice: ReadChoice {
                        depth: depth(len, at),
                    },
                    observed: Cow::Owned(observed),
                }
            })
            .collect()
    }

    /// RA-2 to RA-4.
    fn write(
        &self,
        component: &RaComponent,
        thread: &ThreadPath,
        access: &Access<'_>,
        read: Option<ReadChoice>,
        pre: &ModelSimState,
        post: ModelSimState,
    ) -> Vec<Chosen<RaComponent>> {
        if !component.threads.contains_key(thread) {
            return Chosen::only(Written::Applied(post, component.clone()));
        }
        let Some(shared) = access.shared else {
            return Chosen::only(Written::Applied(
                component.restored(pre, post, None),
                component.clone(),
            ));
        };
        let delta = Delta {
            thread,
            ordering: access.ordering,
            shared,
            pre,
            post: &post,
        };
        match (access.class, read) {
            (AccessClass::Load, Some(read)) => self.load(component, &delta, read),
            (AccessClass::Store, _) => self.store(component, &delta),
            (AccessClass::Rmw, Some(read)) => self.read_modify_write(component, &delta, read),
            (AccessClass::Load | AccessClass::Rmw, None) => Vec::new(),
            (AccessClass::Local, _) => Chosen::only(Written::Applied(
                component.restored(pre, post.clone(), None),
                component.clone(),
            )),
        }
    }

    /// RA-5.
    fn fence(
        &self,
        component: &RaComponent,
        thread: &ThreadPath,
        ordering: AccessOrdering,
    ) -> Option<RaComponent> {
        component.fenced(thread, ordering)
    }

    /// RA-6: the sender's current view.
    fn send(&self, component: &RaComponent, thread: &ThreadPath) -> Option<CarriedView> {
        component
            .threads
            .get(thread)
            .map(|views| CarriedView(views.cur.clone()))
    }

    /// RA-6: the carried view joined into the receiver's current and
    /// acquire views.
    fn receive(
        &self,
        component: &RaComponent,
        thread: &ThreadPath,
        carried: &CarriedView,
    ) -> RaComponent {
        let mut next = component.clone();
        if let Some(views) = next.threads.get_mut(thread) {
            join(&next.shared, &mut views.cur, &carried.0);
            join(&next.shared, &mut views.acq, &carried.0);
            next.collect();
        }
        next
    }

    fn carried_key(&self, component: &RaComponent, carried: &CarriedView) -> CarriedKey {
        CarriedKey(component.view_enc(&carried.0))
    }

    /// `ra` has no memory step.
    fn internal_steps(
        &self,
        _component: &RaComponent,
        _model: &ModelSimState,
    ) -> Vec<MemoryStep<RaComponent>> {
        Vec::new()
    }

    /// `ra` has no gate (RA-6).
    fn gate(&self, _component: &RaComponent, _thread: &ThreadPath, _step: GatedStep) -> bool {
        true
    }

    /// RA-6: each branch starts with its parent's views; entering an
    /// outermost `ra` `parallel` starts the component (RA-1).
    fn split(
        &self,
        component: &RaComponent,
        parent: &ThreadPath,
        branches: &[ThreadPath],
        model: MemoryModelName,
        shared: &[SharedCell<'_>],
        state: &ModelSimState,
    ) -> RaComponent {
        if model != MemoryModelName::Ra {
            return component.clone();
        }
        let mut next = component.clone();
        if !next.threads.contains_key(parent) {
            next.enter(parent, shared, state);
        }
        if let Some(views) = next.threads.get(parent).cloned() {
            for branch in branches {
                next.threads.insert(branch.clone(), views.clone());
            }
        }
        next
    }

    /// RA-6, RA-7: the parent's views become the join of its branches';
    /// after an outermost join nothing is live.
    fn merge(
        &self,
        component: &RaComponent,
        parent: &ThreadPath,
        branches: &[ThreadPath],
    ) -> RaComponent {
        let mut next = component.clone();
        let mut joined: Option<Views> = None;
        for branch in branches {
            let Some(views) = next.threads.remove(branch) else {
                continue;
            };
            match &mut joined {
                Some(all) => all.join(&next.shared, &views),
                None => joined = Some(views),
            }
        }
        next.threads
            .retain(|path, _| !branches.iter().any(|branch| branch.is_prefix_of(path)));
        next.owed
            .retain(|path, _| !branches.iter().any(|branch| branch.is_prefix_of(path)));
        if let Some(views) = joined {
            next.threads.insert(parent.clone(), views);
        }
        if next.threads.keys().all(|path| path == parent) {
            next.clear();
        } else {
            next.collect();
        }
        next
    }

    /// MS-2: each shared location's mo-last message, which the model state
    /// holds.
    fn atom_values<'s>(
        &self,
        _component: &RaComponent,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState> {
        Cow::Borrowed(model)
    }

    /// MF-2's visibility fairness is FR-228's; no infinite-trace clause over
    /// a protocol subject is decided yet (FR-215), so none is added here.
    fn fairness(&self) -> Vec<FairnessConstraint> {
        Vec::new()
    }

    /// MR-1 under `ra`, for a thread with views:
    ///
    /// - a load of `ℓ` reads `msgs(ℓ)` and reads and writes `view(b)`; a
    ///   store or RMW reads and writes both;
    /// - a fence reads and writes `view(b)`, and a `seq_cst` fence also `S`;
    /// - every `seq_cst` access and fence reads and writes the SC event
    ///   graph, and while an SC event is live so does every step that moves
    ///   views, which can collect it;
    /// - `fork` reads and writes `view(b)`, and entering an outermost
    ///   `parallel` writes `msgs(ℓ)`, `S` and the graph; `join` reads and
    ///   writes the views of the threads below `b`, and an outermost one
    ///   writes `msgs(ℓ)`, `S` and the graph;
    /// - `send` reads `view(b)` and `receive` reads and writes it.
    ///
    /// Garbage collection (RA-7) removes only what no view can read, in a
    /// way every order of two steps reaches, so it adds no message location;
    /// the SC events on a collected message leave the graph, which is why a
    /// view-moving step meets the graph while one is live.
    fn footprint(
        &self,
        component: &RaComponent,
        thread: &ThreadPath,
        step: FootprintStep<'_>,
        shared: &BTreeSet<Location>,
        footprint: &mut Footprint,
    ) -> bool {
        let viewed = component.threads.contains_key(thread);
        // A step that moves a view can collect messages, and with them SC
        // events, so while any is live it meets the graph (SR-12255
        // FND-005).
        let collects = !component.events.is_empty();
        let view = Location::Protocol(ProtocolLocation::View(thread.clone()));
        let graph = Location::Protocol(ProtocolLocation::ScGraph);
        let fence_view = Location::Protocol(ProtocolLocation::ScView);
        let messages = |location: &Location| {
            Location::Protocol(ProtocolLocation::Messages(Box::new(location.clone())))
        };
        let touch = |footprint: &mut Footprint, location: Location| {
            footprint.reads.insert(location.clone());
            footprint.enabling.insert(location.clone());
            footprint.writes.insert(location);
        };
        match step {
            FootprintStep::Attempt {
                class,
                ordering,
                shared: location,
            } => {
                if viewed {
                    if let Some(location) = location {
                        touch(footprint, view);
                        match class {
                            AccessClass::Store | AccessClass::Rmw => {
                                touch(footprint, messages(location));
                            }
                            AccessClass::Load | AccessClass::Local => {
                                footprint.reads.insert(messages(location));
                                footprint.enabling.insert(messages(location));
                            }
                        }
                        if ordering == Some(AccessOrdering::SeqCst) || collects {
                            touch(footprint, graph.clone());
                        }
                    }
                }
                true
            }
            FootprintStep::Gated(GatedStep::Fork) => {
                if viewed {
                    touch(footprint, view);
                } else {
                    for location in shared {
                        footprint.writes.insert(messages(location));
                    }
                    footprint.writes.insert(fence_view);
                    footprint.writes.insert(graph);
                    footprint.writes.insert(view);
                }
                true
            }
            FootprintStep::Gated(GatedStep::Join) => {
                for below in component
                    .threads
                    .keys()
                    .filter(|path| *path != thread && thread.is_prefix_of(path))
                {
                    touch(
                        footprint,
                        Location::Protocol(ProtocolLocation::View(below.clone())),
                    );
                }
                if viewed {
                    touch(footprint, view);
                    let outermost = !component
                        .threads
                        .keys()
                        .any(|path| path != thread && path.is_prefix_of(thread));
                    if outermost {
                        for location in shared {
                            footprint.writes.insert(messages(location));
                        }
                        footprint.writes.insert(fence_view);
                    }
                    if outermost || collects {
                        footprint.writes.insert(graph);
                    }
                }
                true
            }
            FootprintStep::Gated(GatedStep::Send) => {
                if viewed {
                    footprint.reads.insert(view);
                }
                true
            }
            FootprintStep::Gated(GatedStep::Receive) => {
                if viewed {
                    touch(footprint, view);
                    if collects {
                        touch(footprint, graph);
                    }
                }
                true
            }
            FootprintStep::Gated(GatedStep::Fence(ordering)) => {
                if viewed {
                    touch(footprint, view);
                    if ordering == AccessOrdering::SeqCst {
                        touch(footprint, fence_view);
                    }
                    if ordering == AccessOrdering::SeqCst || collects {
                        touch(footprint, graph);
                    }
                }
                true
            }
            FootprintStep::Gated(GatedStep::Attempt) => true,
            FootprintStep::Memory { .. } => false,
        }
    }
}

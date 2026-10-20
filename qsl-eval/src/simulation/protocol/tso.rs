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
use std::collections::BTreeMap;

use qsl_foundation::MemoryModelName;
use qsl_semantics::check::{AccessClass, AccessOrdering};
use quire_canonical::{Encode, Error, Sink, Writer};
use quire_exact::Value;
use quire_semantic_value::fairness::FairnessConstraint;

use super::access::location_enc;
use super::identity::ThreadPath;
use super::memory::{Access, GatedStep, MemoryModel, MemoryStep, MemoryStepId, Written};
use crate::simulation::footprint::Location;
use crate::simulation::model::encode::{map_enc, value_enc, Enc};
use crate::simulation::model::{Cell, ModelSimState};

/// The published default of a memory bound (ADR-025 MB-2).
pub const DEFAULT_MEMORY_BOUND: u64 = 4;

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
}

impl PartialEq for Buffered {
    fn eq(&self, other: &Self) -> bool {
        self.location == other.location && self.enc == other.enc
    }
}

impl Eq for Buffered {}

/// `Tso`'s component (TSO-1): each live thread of an active `tso`
/// `parallel` and its buffer, oldest entry first.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct TsoComponent {
    buffers: BTreeMap<ThreadPath, Vec<Buffered>>,
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
        map_enc(
            self.buffers
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
                                            Enc::Object(vec![
                                                ("location", entry.location_enc.clone()),
                                                ("value", entry.enc.clone()),
                                            ])
                                        })
                                        .collect(),
                                ),
                            )],
                        ),
                    )
                })
                .collect(),
        )
    }
}

/// QSpec FR-436: the state key's `memory` member maps each
/// `{"type":"store-buffer","thread":<path>}` to its sequence.
impl Encode for TsoComponent {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.enc().encode_into(writer)
    }
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
        pre: &ModelSimState,
        post: ModelSimState,
    ) -> Written<TsoComponent> {
        let Some(buffer) = component.buffers.get(thread) else {
            return Written::Applied(post, component.clone());
        };
        let locked = access.class == AccessClass::Rmw
            || (access.class == AccessClass::Store
                && access.ordering == Some(AccessOrdering::SeqCst));
        if locked {
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
                    });
                }
                Written::Applied(model, next)
            }
            _ => Written::Applied(unbuffered(pre, post, buffer), component.clone()),
        }
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
                Some(MemoryStep {
                    identity: MemoryStepId {
                        model: MemoryModelName::Tso,
                        step: "flush",
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
            GatedStep::Receive | GatedStep::Attempt => true,
        }
    }

    /// TSO-7: every branch of a `tso` `parallel` starts with an empty
    /// buffer.
    fn split(
        &self,
        component: &TsoComponent,
        _parent: &ThreadPath,
        branches: &[ThreadPath],
        model: MemoryModelName,
    ) -> TsoComponent {
        if model != MemoryModelName::Tso {
            return component.clone();
        }
        let mut next = component.clone();
        for branch in branches {
            next.buffers.insert(branch.clone(), Vec::new());
        }
        next
    }

    /// TSO-7: a join removes its branches' buffers, which its gate found
    /// empty.
    fn merge(
        &self,
        component: &TsoComponent,
        _parent: &ThreadPath,
        branches: &[ThreadPath],
    ) -> TsoComponent {
        let mut next = component.clone();
        next.buffers
            .retain(|path, _| !branches.iter().any(|branch| branch.is_prefix_of(path)));
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

    /// MF-1's flush fairness is FR-228's; no infinite-trace clause over a
    /// protocol subject is decided yet (FR-215), so none is added here.
    fn fairness(&self) -> Vec<FairnessConstraint> {
        Vec::new()
    }
}

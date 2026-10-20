// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 §2.5 SE-1 to SE-3: the one seam through which the protocol system
//! reaches memory, and [`Sc`], the sequentially consistent model that is the
//! seam's identity (SE-2).

use std::borrow::Cow;

use qsl_foundation::MemoryModelName;
use qsl_semantics::check::AccessClass;
use qsl_semantics::check::AccessOrdering;
use quire_canonical::{Encode, Error, Sink, Writer};
use quire_semantic_value::fairness::FairnessConstraint;

use std::collections::BTreeSet;

use super::footprint::ProtocolLocation;
use super::identity::ThreadPath;
use crate::simulation::footprint::{Footprint, Location};
use crate::simulation::model::encode::Enc;
use crate::simulation::model::{Cell, ModelSimState};

/// The step of a thread a memory model's gate is read for (SE-1 e).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GatedStep {
    /// ST-6.
    Fork,
    /// ST-7.
    Join,
    /// ST-3.
    Send,
    /// ST-4.
    Receive,
    /// ST-1.
    Attempt,
}

/// One internal step's typed canonical identity (SE-1 d, QSpec FR-436):
/// `{"type":"memory-step","model":<model>,"step":<step>,"thread":<path>}`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemoryStepId {
    /// The memory model that owns the step.
    pub model: MemoryModelName,
    /// The step's name, such as `flush`.
    pub step: &'static str,
    /// The thread the step is about.
    pub thread: ThreadPath,
}

impl MemoryStepId {
    pub(crate) fn enc(&self) -> Enc {
        Enc::tagged(
            "memory-step",
            vec![
                ("model", Enc::text(self.model.label())),
                ("step", Enc::text(self.step)),
                ("thread", self.thread.enc()),
            ],
        )
    }
}

/// One internal step of a memory model (SE-1 d): its identity and its
/// effect.
#[derive(Clone, Debug)]
pub struct MemoryStep<C> {
    /// The step's identity.
    pub identity: MemoryStepId,
    /// The memory component after the step.
    pub component: C,
    /// The model state after the step.
    pub model: ModelSimState,
}

/// The one shared location of an access (ADR-025 MA-2) and the cell of the
/// model state it names.
#[derive(Clone, Copy, Debug)]
pub struct SharedCell<'a> {
    /// The location.
    pub location: &'a Location,
    /// Its cell.
    pub cell: &'a Cell,
}

/// What an attempt's application is, as EN-1 classifies it (FR-220): its
/// class over the shared locations of its weak `parallel`, its ordering,
/// and its one shared location. An attempt outside every weak `parallel`
/// is local.
#[derive(Clone, Copy, Debug)]
pub struct Access<'a> {
    /// The access class.
    pub class: AccessClass,
    /// The attempt's `ordering`.
    pub ordering: Option<AccessOrdering>,
    /// The shared location of a load, store or read-modify-write.
    pub shared: Option<SharedCell<'a>>,
}

impl Access<'_> {
    /// A local access, which applies in place under every model.
    pub fn local() -> Self {
        Self {
            class: AccessClass::Local,
            ordering: None,
            shared: None,
        }
    }
}

/// A step as a memory model's footprint reads it (ADR-025 MR-1).
#[derive(Clone, Copy, Debug)]
pub enum FootprintStep<'a> {
    /// An attempt, as EN-1 classified its application: its class, its
    /// ordering and its one shared location.
    Attempt {
        /// The access class.
        class: AccessClass,
        /// The attempt's `ordering`.
        ordering: Option<AccessOrdering>,
        /// The shared location of a load, store or read-modify-write.
        shared: Option<&'a Location>,
    },
    /// A step the model gates (SE-1 e).
    Gated(GatedStep),
    /// One of the model's own internal steps, and every shared location of
    /// the subject, which its thread may store.
    Memory {
        /// The step's identity.
        identity: &'a MemoryStepId,
        /// The shared locations.
        stored: &'a BTreeSet<Location>,
    },
}

/// `buf(thread)`: `thread`'s part of the memory component (ADR-025 MR-1).
pub(crate) fn buffer_location(thread: &ThreadPath) -> Location {
    Location::Protocol(ProtocolLocation::Buffer(thread.clone()))
}

/// Where an attempt's delta went (SE-1 c).
#[derive(Clone, Debug)]
pub enum Written<C> {
    /// The model state and component after the step.
    Applied(ModelSimState, C),
    /// The model's gate disables this application (SE-1 e), such as a
    /// locked access whose thread's buffer is not empty.
    Disabled,
    /// A store a memory bound does not expand at this state (ADR-025 MB-3).
    Cut,
}

/// ADR-027 SE-1: the memory model a `parallel` runs under, through its
/// eight members.
pub trait MemoryModel {
    /// (a) The memory state, a member of the state key.
    type Component: Clone + Eq + std::fmt::Debug + Encode;

    /// The resolved models (FR-219) this memory model implements: a
    /// protocol subject whose `parallel` resolves to another is refused.
    fn models(&self) -> &'static [MemoryModelName] {
        &[MemoryModelName::Sc]
    }

    /// (a) The component of an initial state.
    fn initial(&self) -> Self::Component;

    /// (b) The observation of the model state `model` that `thread`'s
    /// attempt evaluates its contracts over.
    fn observe<'s>(
        &self,
        component: &Self::Component,
        thread: &ThreadPath,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState>;

    /// (c) and (e) Where `thread`'s attempt `access` from `pre` to `post`
    /// writes its delta, `post` computed over the thread's observation; or
    /// that the gate disables it, or a memory bound cuts it.
    fn write(
        &self,
        component: &Self::Component,
        thread: &ThreadPath,
        access: &Access<'_>,
        pre: &ModelSimState,
        post: ModelSimState,
    ) -> Written<Self::Component>;

    /// (d) The internal steps enabled at (`component`, `model`), each with
    /// its identity and effect.
    fn internal_steps(
        &self,
        component: &Self::Component,
        model: &ModelSimState,
    ) -> Vec<MemoryStep<Self::Component>>;

    /// (e) Whether `thread`'s `step` is enabled by the model.
    fn gate(&self, component: &Self::Component, thread: &ThreadPath, step: GatedStep) -> bool;

    /// (f) The component after `parent` forks `branches` of a `parallel`
    /// whose resolved model is `model`.
    fn split(
        &self,
        component: &Self::Component,
        parent: &ThreadPath,
        branches: &[ThreadPath],
        model: MemoryModelName,
    ) -> Self::Component;

    /// (f) The component after `parent` joins `branches`.
    fn merge(
        &self,
        component: &Self::Component,
        parent: &ThreadPath,
        branches: &[ThreadPath],
    ) -> Self::Component;

    /// (g) The model state a temporal atom reads each location of.
    fn atom_values<'s>(
        &self,
        component: &Self::Component,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState>;

    /// (h) The constraints the model adds to every infinite-trace clause,
    /// with a `memory` origin.
    fn fairness(&self) -> Vec<FairnessConstraint>;

    /// ADR-025 MR-1: add to `footprint`, `thread`'s `step`'s protocol and
    /// model footprint, the memory-component locations it reads, writes or
    /// is enabled by, and take out a model write the model does not apply in
    /// place; `false` when `step` is not a step of this model at
    /// `component`.
    fn footprint(
        &self,
        component: &Self::Component,
        thread: &ThreadPath,
        step: FootprintStep<'_>,
        footprint: &mut Footprint,
    ) -> bool;
}

/// SE-2: the sequentially consistent memory model.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Sc;

/// `Sc`'s component: empty. It encodes as the empty `map`, the empty member
/// of every key (ADR-027 PS-8).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScComponent;

impl Encode for ScComponent {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("entries")?;
        writer.begin_array()?;
        writer.end_array()?;
        writer.name("type")?;
        writer.string("map")?;
        writer.end_object()
    }
}

impl MemoryModel for Sc {
    type Component = ScComponent;

    fn initial(&self) -> ScComponent {
        ScComponent
    }

    /// The model state itself.
    fn observe<'s>(
        &self,
        _component: &ScComponent,
        _thread: &ThreadPath,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState> {
        Cow::Borrowed(model)
    }

    /// In place: the post-state is the state.
    fn write(
        &self,
        _component: &ScComponent,
        _thread: &ThreadPath,
        _access: &Access<'_>,
        _pre: &ModelSimState,
        post: ModelSimState,
    ) -> Written<ScComponent> {
        Written::Applied(post, ScComponent)
    }

    /// None.
    fn internal_steps(
        &self,
        _component: &ScComponent,
        _model: &ModelSimState,
    ) -> Vec<MemoryStep<ScComponent>> {
        Vec::new()
    }

    /// Always true.
    fn gate(&self, _component: &ScComponent, _thread: &ThreadPath, _step: GatedStep) -> bool {
        true
    }

    /// Nothing changes.
    fn split(
        &self,
        _component: &ScComponent,
        _parent: &ThreadPath,
        _branches: &[ThreadPath],
        _model: MemoryModelName,
    ) -> ScComponent {
        ScComponent
    }

    /// Nothing changes.
    fn merge(
        &self,
        _component: &ScComponent,
        _parent: &ThreadPath,
        _branches: &[ThreadPath],
    ) -> ScComponent {
        ScComponent
    }

    /// The model state itself.
    fn atom_values<'s>(
        &self,
        _component: &ScComponent,
        model: &'s ModelSimState,
    ) -> Cow<'s, ModelSimState> {
        Cow::Borrowed(model)
    }

    /// None.
    fn fairness(&self) -> Vec<FairnessConstraint> {
        Vec::new()
    }

    /// Nothing: `sc` names no memory location and has no memory step.
    fn footprint(
        &self,
        _component: &ScComponent,
        _thread: &ThreadPath,
        step: FootprintStep<'_>,
        _footprint: &mut Footprint,
    ) -> bool {
        !matches!(step, FootprintStep::Memory { .. })
    }
}

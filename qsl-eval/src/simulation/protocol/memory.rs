// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 §2.5 SE-1 to SE-3: the one seam through which the protocol system
//! reaches memory, and [`Sc`], the sequentially consistent model that is the
//! seam's identity (SE-2).

use std::borrow::Cow;

use quire_canonical::{Encode, Error, Sink, Writer};
use quire_semantic_value::fairness::FairnessConstraint;

use super::identity::ThreadPath;
use crate::simulation::model::ModelSimState;

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

/// One internal step of a memory model (SE-1 d): its identity, a canonical
/// text its model gives, and its effect.
#[derive(Clone, Debug)]
pub struct MemoryStep<C> {
    /// The step's identity.
    pub identity: String,
    /// The memory component after the step.
    pub component: C,
    /// The model state after the step.
    pub model: ModelSimState,
}

/// ADR-027 SE-1: the memory model a `parallel` runs under, through its
/// eight members.
pub trait MemoryModel {
    /// (a) The memory state, a member of the state key.
    type Component: Clone + Eq + std::fmt::Debug + Encode;

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

    /// (c) Where `thread`'s attempt from `pre` to `post` writes its delta:
    /// the model state and component after the step.
    fn write(
        &self,
        component: &Self::Component,
        thread: &ThreadPath,
        pre: &ModelSimState,
        post: ModelSimState,
    ) -> (ModelSimState, Self::Component);

    /// (d) The internal steps enabled at (`component`, `model`), each with
    /// its identity and effect.
    fn internal_steps(
        &self,
        component: &Self::Component,
        model: &ModelSimState,
    ) -> Vec<MemoryStep<Self::Component>>;

    /// (e) Whether `thread`'s `step` is enabled by the model.
    fn gate(&self, component: &Self::Component, thread: &ThreadPath, step: GatedStep) -> bool;

    /// (f) The component after `parent` forks `branches`.
    fn split(
        &self,
        component: &Self::Component,
        parent: &ThreadPath,
        branches: &[ThreadPath],
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
        _pre: &ModelSimState,
        post: ModelSimState,
    ) -> (ModelSimState, ScComponent) {
        (post, ScComponent)
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
}

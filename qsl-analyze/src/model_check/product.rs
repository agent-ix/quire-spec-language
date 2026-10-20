// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 SM-3, SM-4 and EN-1: a product over the existing transition seam.

use qsl_eval::simulation::{
    Expansion, ExpansionStop, Footprint, OfferedReductions, TransitionSystem,
};
use quire_canonical::{Encode, Sink, Writer};

/// Both components of a product state, preserved independently.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProductState<M, P> {
    /// The original model state.
    pub model: M,
    /// The property state after reading this position.
    pub property: P,
}

/// Independently supplied canonical keys of the two product components.
pub struct ProductKey<M, P> {
    /// Model component's full key.
    pub model: M,
    /// Property component's full key, independent of materialization order.
    pub property: P,
}

impl<M: Encode, P: Encode> Encode for ProductKey<M, P> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        self.model.encode_into(writer)?;
        self.property.encode_into(writer)?;
        writer.end_array()
    }
}

/// A graph-local edge identity; terminal stutter cannot alias a model step.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductTransition<T> {
    /// An unchanged transition identity from the model.
    Model(T),
    /// SM-4's infinite terminal repetition, with no model operation.
    TerminalStutter,
}

impl<T: Encode> Encode for ProductTransition<T> {
    fn encode_into<S: Sink + ?Sized>(
        &self,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), quire_canonical::Error> {
        writer.begin_array()?;
        match self {
            Self::Model(identity) => {
                writer.string("model")?;
                identity.encode_into(writer)?;
            }
            Self::TerminalStutter => writer.string("terminal-stutter")?,
        }
        writer.end_array()
    }
}

/// A finding retains which component produced it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProductFinding<M, P> {
    /// A finding from the model's expansion.
    Model(M),
    /// A property finding, including closed-boundary findings.
    Property(P),
}

/// Adapter for a supplied property state machine, not a formula translator.
///
/// The adapter owns letter evaluation and state advancement. `initial` reads
/// position zero; `advance` reads the post-state with the actual edge identity.
/// Implementations must put every fact affecting future moves into their key.
pub trait PropertyComponent<M: TransitionSystem> {
    /// Property state after reading a position.
    type State;
    /// Full canonical property key.
    type Key: Encode;
    /// Findings exposed without interpretation by the product.
    type Finding: Clone + Eq + std::fmt::Debug;

    /// States after reading the model's initial position.
    fn initial(&self, model: &M::State) -> Vec<Self::State>;
    /// Canonical key of a property state.
    fn key(&self, state: &Self::State) -> Self::Key;
    /// Property moves on a concrete model step or terminal repetition.
    ///
    /// # Errors
    /// Returns the original expansion stop when letter evaluation cannot run.
    fn advance(
        &self,
        state: &Self::State,
        source: &M::State,
        transition: &ProductTransition<M::TransitionId>,
        target: &M::State,
    ) -> Result<Vec<Self::State>, ExpansionStop>;
    /// Findings at this position.
    ///
    /// # Errors
    /// Returns the original property evaluation stop.
    fn findings(
        &self,
        state: &Self::State,
        model: &M::State,
    ) -> Result<Vec<Self::Finding>, ExpansionStop>;
    /// Close a bounded terminal execution with the profile's boundary rule.
    ///
    /// No model step is synthesized. The supplied monitor owns false-extension
    /// and constant handling, and reports its closure findings here.
    ///
    /// # Errors
    /// Returns the original monitor closure stop.
    fn close(
        &self,
        state: &Self::State,
        model: &M::State,
    ) -> Result<Vec<Self::Finding>, ExpansionStop>;
}

/// A model/property product using FR-101's actual `TransitionSystem` seam.
pub struct Product<'a, M, P> {
    model: &'a M,
    property: &'a P,
    infinite: bool,
}

impl<'a, M, P> Product<'a, M, P> {
    /// A bounded product: terminal executions close through the monitor.
    pub const fn bounded(model: &'a M, property: &'a P) -> Self {
        Self {
            model,
            property,
            infinite: false,
        }
    }
    /// An infinite product: terminal executions advance along stutter steps.
    pub const fn infinite(model: &'a M, property: &'a P) -> Self {
        Self {
            model,
            property,
            infinite: true,
        }
    }
}

impl<M, P> TransitionSystem for Product<'_, M, P>
where
    M: TransitionSystem,
    M::State: Clone,
    P: PropertyComponent<M>,
{
    type State = ProductState<M::State, P::State>;
    type TransitionId = ProductTransition<M::TransitionId>;
    type Key = ProductKey<M::Key, P::Key>;
    type Finding = ProductFinding<M::Finding, P::Finding>;

    fn initial(&self) -> Vec<Self::State> {
        self.model
            .initial()
            .into_iter()
            .flat_map(|model| {
                self.property
                    .initial(&model)
                    .into_iter()
                    .map(move |property| ProductState {
                        model: model.clone(),
                        property,
                    })
            })
            .collect()
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        ProductKey {
            model: self.model.key(&state.model),
            property: self.property.key(&state.property),
        }
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        let expansion = self.model.successors(&state.model)?;
        let mut findings: Vec<_> = expansion
            .findings
            .into_iter()
            .map(ProductFinding::Model)
            .collect();
        findings.extend(
            self.property
                .findings(&state.property, &state.model)?
                .into_iter()
                .map(ProductFinding::Property),
        );
        let mut steps: Vec<_> = expansion
            .successors
            .into_iter()
            .map(|(identity, model)| (ProductTransition::Model(identity), model))
            .collect();
        if steps.is_empty() {
            if self.infinite {
                steps.push((ProductTransition::TerminalStutter, state.model.clone()));
            } else {
                findings.extend(
                    self.property
                        .close(&state.property, &state.model)?
                        .into_iter()
                        .map(ProductFinding::Property),
                );
            }
        }
        let mut successors = Vec::new();
        for (transition, model) in steps {
            for property in
                self.property
                    .advance(&state.property, &state.model, &transition, &model)?
            {
                successors.push((
                    transition.clone(),
                    ProductState {
                        model: model.clone(),
                        property,
                    },
                ));
            }
        }
        Ok(Expansion {
            successors,
            findings,
        })
    }

    /// ADR-021 EI-4: the product offers the partial-order reduction its
    /// model component offers, and nothing else: the property component is
    /// never reduced.
    fn offered_reductions(&self) -> OfferedReductions {
        OfferedReductions {
            partial_order: self.model.offered_reductions().partial_order,
            ..OfferedReductions::NONE
        }
    }

    /// FR-162: a model transition has its model component's footprint, and
    /// the terminal stutter edge has an empty one, so no ample-set
    /// candidate holds it.
    fn footprint(
        &self,
        transition: &Self::TransitionId,
    ) -> Option<Result<Footprint, ExpansionStop>> {
        match transition {
            ProductTransition::Model(transition) => self.model.footprint(transition),
            ProductTransition::TerminalStutter => Some(Ok(Footprint::default())),
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-339 "Producer" (FR-126 "Certificates"): the component certificate of a
//! liveness run that found no fair accepting component.
//!
//! The closure lists every explored product state, written as the core's
//! checker will recompute it: the model state's digest and the canonical key
//! of its automaton state in the key context of its instance. The
//! components are the strongly connected components the fairness kernels
//! decomposed each binding's graph into, with the witness the filter found
//! for each, listed in FR-126 AC-18's topological order: bindings in
//! canonical instance order, and within a binding (and within each `sub`
//! list) the unlisted component with the least vertex index whose
//! predecessors are all listed comes next.
//!
//! The constraints the witnesses index are resolved once over the
//! transitions of every binding, ascending by their FR-101 canonical bytes,
//! the order the checker resolves them in.

use super::*;

use qsl_eval::fairness::{is_live, Outcome as Found};
use qsl_eval::simulation::ConstraintError;
use qsl_replay::{
    ClosureCertificate, Component, ComponentCertificate, ComponentWitness, ProductStateRef,
    Translation,
};

/// One binding's product states written as the checker writes them.
type Refs = Vec<ProductStateRef>;

impl<S: LiveSubject> Run<'_, S> {
    /// The component certificate of the fully explored `explored` products,
    /// one per binding in canonical instance order; the cancellation that
    /// stopped the fairness analysis instead.
    pub(super) fn component_certificate(
        &self,
        explored: &[Explored<'_, S>],
    ) -> Result<Result<ComponentCertificate, CancelCause>, ModelCheckRefusal> {
        let instances = (self.certify.instances)().map_err(ModelCheckRefusal::Item)?;
        if instances.len() != explored.len() {
            return Err(fault("every-binding-is-an-instance"));
        }
        let mut refs = Vec::with_capacity(explored.len());
        for (run, instance) in explored.iter().zip(&instances) {
            refs.push(self.product_refs(run, instance)?);
        }
        let universe = self.universe(explored)?;
        let numbered = |transition: &Tr<S>| {
            self.system
                .transition_key(transition)
                .ok()
                .and_then(|key| universe.identities.get(&key).copied())
        };
        let constraints = self
            .system
            .clause_constraints(self.clause.fairness(), &universe.transitions, &numbered)
            .map_err(|error| match error {
                ConstraintError::UnknownOperation { name } => {
                    LivenessRefusal::UnknownOperation { name }.into()
                }
                ConstraintError::Protocol(error) => error.into(),
                ConstraintError::Unnumbered => fault("every-seen-transition-is-numbered"),
            })?;
        let identity = |transition: &Tr<S>| {
            numbered(transition).ok_or_else(|| fault("every-seen-transition-is-numbered"))
        };
        let mut components = Vec::new();
        for (run, refs) in explored.iter().zip(&refs) {
            let graph = FairModel::graph(run, &identity)?;
            let all: BTreeSet<Vertex> = graph.vertices().collect();
            let analyses = match analyze(&graph, &all, &constraints, self.cancel) {
                Ok(analyses) => analyses,
                Err(FairnessError::Cancelled) => {
                    return match self.cancel.cause() {
                        Some(source) => Ok(Err(source)),
                        None => Err(fault("a-cancelled-run-has-a-cause")),
                    }
                }
                Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                    return Err(fault("the-fairness-graph-is-built-from-retained-states"))
                }
            };
            components.extend(Self::listed(&graph, &analyses, refs)?);
        }
        let mut states: Vec<ProductStateRef> = refs.into_iter().flatten().collect();
        states.sort();
        states.dedup();
        Ok(Ok(ComponentCertificate {
            closure: ClosureCertificate { states },
            components,
        }))
    }

    /// The product states of `run`, indexed as its graph's vertices.
    fn product_refs(
        &self,
        run: &Explored<'_, S>,
        instance: &Instance,
    ) -> Result<Refs, ModelCheckRefusal> {
        let Translation::Liveness(translation) = &instance.translation else {
            return Err(fault("a-liveness-instance-has-a-liveness-translation"));
        };
        let keys = translation.keys();
        let automaton = run.product.automaton.borrow();
        run.graph
            .states()
            .iter()
            .map(|state| {
                let Property::After(at) = &state.state.property else {
                    return Err(fault("a-proved-product-state-has-a-letter"));
                };
                let structural = automaton
                    .buchi_state(*at, keys.max_key_bytes())
                    .map_err(ModelCheckRefusal::Translation)?;
                Ok(ProductStateRef {
                    model_state: self.digest(&state.state.model)?,
                    automaton_state: keys.key(&structural)?,
                })
            })
            .collect()
    }

    /// The transitions of every binding's graph, ascending by canonical
    /// bytes, and the identity each carries.
    fn universe(&self, explored: &[Explored<'_, S>]) -> Result<Universe<S>, ModelCheckRefusal> {
        let mut seen: BTreeMap<Vec<u8>, Tr<S>> = BTreeMap::new();
        for run in explored {
            let graph = &run.graph;
            let edges = graph
                .edges()
                .iter()
                .filter_map(|edge| match &edge.transition {
                    ProductTransition::Model(transition) => Some(transition),
                    ProductTransition::TerminalStutter => None,
                });
            let enabled = graph.states().iter().flat_map(|state| {
                state.findings.iter().flat_map(|finding| match finding {
                    Finding::Enabled(enabled) => enabled.iter(),
                    _ => [].iter(),
                })
            });
            for transition in edges.chain(enabled) {
                seen.entry(self.system.transition_key(transition)?)
                    .or_insert_with(|| transition.clone());
            }
        }
        let identities = seen
            .keys()
            .enumerate()
            .map(|(index, key)| (key.clone(), Identity::new(widen(index))))
            .collect();
        Ok(Universe {
            transitions: seen.into_values().collect(),
            identities,
        })
    }

    /// `analyses` as FR-126 AC-18's component list: each component's states
    /// ascending, the components in topological order.
    fn listed(
        graph: &Graph,
        analyses: &[Analysis],
        refs: &Refs,
    ) -> Result<Vec<Component>, ModelCheckRefusal> {
        let scope: BTreeSet<Vertex> = analyses
            .iter()
            .flat_map(|analysis| analysis.vertices().iter().copied())
            .collect();
        let removed = analyses.first().map(Analysis::removed);
        let mut owner: BTreeMap<Vertex, usize> = BTreeMap::new();
        for (index, analysis) in analyses.iter().enumerate() {
            for vertex in analysis.vertices() {
                owner.insert(*vertex, index);
            }
        }
        // The components each component's live edges lead to.
        let mut successors: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); analyses.len()];
        let mut waiting = vec![0_usize; analyses.len()];
        if let Some(removed) = removed {
            for (index, analysis) in analyses.iter().enumerate() {
                for vertex in analysis.vertices() {
                    for edge in graph.edges_from(*vertex) {
                        let Some(target) = owner.get(&edge.target).copied() else {
                            continue;
                        };
                        if target != index
                            && scope.contains(&edge.target)
                            && is_live(edge, removed)
                            && successors[index].insert(target)
                        {
                            waiting[target] += 1;
                        }
                    }
                }
            }
        }
        let mut ready: BTreeSet<(Vertex, usize)> = BTreeSet::new();
        let least = |index: usize| analyses[index].vertices().iter().min().copied();
        for (index, count) in waiting.iter().enumerate() {
            if *count == 0 {
                ready.insert((
                    least(index).ok_or_else(|| fault("a-component-has-a-vertex"))?,
                    index,
                ));
            }
        }
        let mut listed = Vec::with_capacity(analyses.len());
        while let Some((_, index)) = ready.pop_first() {
            listed.push(Self::component(graph, &analyses[index], refs)?);
            for target in &successors[index] {
                waiting[*target] -= 1;
                if waiting[*target] == 0 {
                    ready.insert((
                        least(*target).ok_or_else(|| fault("a-component-has-a-vertex"))?,
                        *target,
                    ));
                }
            }
        }
        if listed.len() != analyses.len() {
            return Err(fault("the-components-of-a-graph-are-acyclic"));
        }
        Ok(listed)
    }

    fn component(
        graph: &Graph,
        analysis: &Analysis,
        refs: &Refs,
    ) -> Result<Component, ModelCheckRefusal> {
        let mut states = analysis
            .vertices()
            .iter()
            .map(|vertex| {
                refs.get(vertex.index())
                    .cloned()
                    .ok_or_else(|| fault("a-component-state-is-a-product-state"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        states.sort();
        let index = |at: usize| u32::try_from(at).map_err(|_| fault("an-index-fits-u32"));
        let witness = match analysis.outcome() {
            Found::Trivial => ComponentWitness::Trivial,
            Found::MissingAcceptance { set } => {
                ComponentWitness::MissingAcceptance { set: index(*set)? }
            }
            Found::UnfairWeak { constraint } => ComponentWitness::UnfairWeak {
                constraint: index(*constraint)?,
            },
            Found::UnfairVisibility { constraints, sub } => ComponentWitness::UnfairVisibility {
                constraints: constraints
                    .iter()
                    .map(|at| index(*at))
                    .collect::<Result<_, _>>()?,
                sub: Self::listed(graph, sub, refs)?,
            },
            Found::UnfairStrong { constraints, sub } => ComponentWitness::UnfairStrong {
                constraints: constraints
                    .iter()
                    .map(|at| index(*at))
                    .collect::<Result<_, _>>()?,
                sub: Self::listed(graph, sub, refs)?,
            },
            Found::Fair => return Err(fault("a-proved-run-has-no-fair-component")),
        };
        Ok(Component { states, witness })
    }
}

/// The transitions of a proof's graphs.
struct Universe<S: LiveSubject> {
    /// Ascending by canonical bytes.
    transitions: Vec<Tr<S>>,
    identities: BTreeMap<Vec<u8>, Identity>,
}

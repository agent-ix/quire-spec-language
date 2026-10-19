// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-339 (ADR-018 PC-4, ADR-019 AM-7): the components of the component
//! certificate EN-1 returns with a liveness (TP-4) proof, and the core's
//! check of them over the product graph the checker recomputed.
//!
//! [`check_partition`] reads the certificate's [`Component`]s against a
//! [`ProductGraph`]: the reached product states of the closure, every edge
//! the checker computed between them, each state's FR-120 enabled set, and
//! the acceptance sets of the negated formula's generalized Büchi automaton.
//! It never trusts the engine's SCC search. It accepts exactly when:
//!
//! - the components partition the reached states (`NotPartition`);
//! - no computed edge goes from a component to an earlier one
//!   (`BackwardEdge`), so every cycle lies inside one component;
//! - every component's witness holds (`WitnessFails`): `Trivial` is one
//!   state with no edge to itself; `MissingAcceptance{set}` has no state in
//!   that acceptance set; `UnfairWeak{constraint}` names a weak constraint
//!   enabled at every state and taken by no edge inside the component;
//!   `UnfairStrong{constraints, sub}` names strong constraints none of
//!   which an edge inside the component takes, and `sub` partitions exactly
//!   the component's states at which none of them is enabled, under the same
//!   three rules.
//!
//! So an accepted partition proves that no fair accepting cycle exists.
//!
//! The rules run in that order over one level of components: the whole
//! partition, then every edge, then each witness in listed order. The `sub`
//! levels of `UnfairStrong` witnesses are checked after their level, depth
//! first in listed order, with an explicit stack, so a deep certificate
//! cannot overflow the call stack. The first failed rule is the rejection:
//! a duplicated, foreign or uncovered state is the `NotPartition` locus (an
//! uncovered one is the least, by [`ProductStateRef`] order), a backward
//! edge's source state the `BackwardEdge` locus, and a component's first
//! listed state its `WitnessFails` locus. An empty component, which has no
//! state of its own, rejects `NotPartition` at the first state of the
//! component it refines, or at the least reached state at the top level.
//!
//! The check materializes no state, edge or automaton state: the product
//! graph's construction already charged those budgets. It polls the cancel
//! handle at every component, state and edge it reads, and a cancellation
//! stops it with `Stopped(Cancelled)`, which is never an acceptance.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::fairness::{Constraint, EdgeLabel, FairnessError, Graph, Identity, Strength, Vertex};
use quire_exact::Cancel;

use crate::certificate::closure::{
    CertificateRejection, ProductStateRef, TemporalCertificateCheck,
};
use crate::certificate::{CertificateLocus, CertificateRule};
use crate::proof_result::IncompleteCause;

/// One component of a component certificate: its product states and the
/// reason no fair accepting cycle stays inside it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Component {
    /// The component's product states.
    pub states: Vec<ProductStateRef>,
    /// Why no fair accepting cycle stays inside it.
    pub witness: ComponentWitness,
}

/// FR-339's component witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ComponentWitness {
    /// One state with no edge to itself: the component holds no cycle.
    Trivial,
    /// No state of the component lies in acceptance set `set`.
    MissingAcceptance {
        /// The index of the acceptance set.
        set: u32,
    },
    /// The weak constraint at `constraint` is enabled at every state of the
    /// component and taken by no edge with both ends in it.
    UnfairWeak {
        /// The index into the clause's resolved constraints.
        constraint: u32,
    },
    /// Every strong constraint in `constraints` is taken by no edge with
    /// both ends in the component, and `sub` partitions the component's
    /// states at which none of them is enabled (ADR-019 AM-7).
    UnfairStrong {
        /// The indices of every failing strong constraint.
        constraints: Vec<u32>,
        /// The refinement's components, in topological order.
        sub: Vec<Component>,
    },
}

/// Why a [`ProductGraph`] refused a construction step: the checker's own
/// product, not the certificate, is at fault.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ProductGraphError {
    /// The product state was added before.
    #[error("the product state was added twice")]
    DuplicateState,
    /// A fairness-graph refusal.
    #[error(transparent)]
    Fairness(#[from] FairnessError),
}

/// The product graph the checker recomputed for a TP-4 item: its reached
/// product states, in the order the checker reached them, the edges it
/// computed between them, each state's enabled set and the acceptance sets.
#[derive(Clone, Debug, Default)]
pub(crate) struct ProductGraph {
    graph: Graph,
    states: Vec<ProductStateRef>,
    vertices: BTreeMap<ProductStateRef, Vertex>,
}

impl ProductGraph {
    /// An empty product graph.
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Add the reached product state `state`, at which exactly the
    /// transition identities in `enabled` are enabled (ADR-019 FS-1: its
    /// model state's enabled set).
    pub(crate) fn add_state(
        &mut self,
        state: ProductStateRef,
        enabled: impl IntoIterator<Item = Identity>,
    ) -> Result<Vertex, ProductGraphError> {
        if self.vertices.contains_key(&state) {
            return Err(ProductGraphError::DuplicateState);
        }
        let vertex = self.graph.add_vertex(enabled);
        self.vertices.insert(state.clone(), vertex);
        self.states.push(state);
        Ok(vertex)
    }

    /// Add an edge the checker computed, after the edges already out of
    /// `from`.
    pub(crate) fn add_edge(
        &mut self,
        from: Vertex,
        to: Vertex,
        label: EdgeLabel,
    ) -> Result<(), ProductGraphError> {
        Ok(self.graph.add_edge(from, to, label)?)
    }

    /// Add the next acceptance set of the negated formula's automaton, in
    /// FR-338's canonical set order.
    pub(crate) fn add_acceptance_set(
        &mut self,
        members: impl IntoIterator<Item = Vertex>,
    ) -> Result<(), ProductGraphError> {
        Ok(self.graph.add_acceptance_set(members)?)
    }

    fn state(&self, vertex: Vertex) -> Option<&ProductStateRef> {
        self.states.get(vertex.index())
    }
}

/// A finished check: the rejection or stop to return.
type Step<T> = Result<T, TemporalCertificateCheck>;

/// The component index owning each state of a level's scope, and each
/// component's vertices in listed order.
type Partition = (BTreeMap<Vertex, usize>, Vec<Vec<Vertex>>);

/// One level of components to check: the states they must partition, the
/// components, and the locus of an empty component.
struct Level<'c> {
    scope: BTreeSet<Vertex>,
    components: &'c [Component],
    fallback: ProductStateRef,
}

/// The product graph holds no state. A closure the checker reached always
/// holds its initial states, so this is a fault of the caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the product graph holds no state")]
pub(crate) struct EmptyProduct;

/// FR-339's partition, edge and witness checks of `components` over
/// `product` under the clause's resolved `constraints`, polling `cancel` at
/// every component, state and edge.
pub(crate) fn check_partition(
    product: &ProductGraph,
    components: &[Component],
    constraints: &[Constraint],
    cancel: &Cancel,
) -> Result<TemporalCertificateCheck, EmptyProduct> {
    let least = product.states.iter().min().cloned().ok_or(EmptyProduct)?;
    let checker = Checker {
        product,
        constraints,
        cancel,
    };
    let mut stack = vec![Level {
        scope: product.graph.vertices().collect(),
        components,
        fallback: least,
    }];
    while let Some(level) = stack.pop() {
        match checker.level(&level) {
            Ok(mut sub) => {
                sub.reverse();
                stack.extend(sub);
            }
            Err(halt) => return Ok(halt),
        }
    }
    Ok(TemporalCertificateCheck::Accepted)
}

struct Checker<'p> {
    product: &'p ProductGraph,
    constraints: &'p [Constraint],
    cancel: &'p Cancel,
}

impl<'p> Checker<'p> {
    fn poll(&self) -> Step<()> {
        if self.cancel.poll() {
            if let Some(source) = self.cancel.cause() {
                return Err(TemporalCertificateCheck::Stopped(
                    IncompleteCause::Cancelled { source },
                ));
            }
        }
        Ok(())
    }

    /// Check one level; the `sub` levels of its `UnfairStrong` witnesses, in
    /// listed order.
    fn level<'c>(&self, level: &Level<'c>) -> Step<Vec<Level<'c>>> {
        let (owner, members) = self.partition(level)?;
        self.edges(level.components, &members, &owner)?;
        let mut sub = Vec::new();
        for (component, members) in level.components.iter().zip(&members) {
            self.poll()?;
            if let Some(refinement) = self.witness(component, members, &level.fallback)? {
                sub.push(refinement);
            }
        }
        Ok(sub)
    }

    /// The component that owns each state of the scope, and each
    /// component's vertices, in listed order; `NotPartition` when the
    /// components do not partition the scope.
    fn partition(&self, level: &Level<'_>) -> Step<Partition> {
        let mut owner = BTreeMap::new();
        let mut members = Vec::with_capacity(level.components.len());
        for (index, component) in level.components.iter().enumerate() {
            self.poll()?;
            if component.states.is_empty() {
                return Err(reject(
                    CertificateRule::NotPartition,
                    level.fallback.clone(),
                ));
            }
            let mut vertices = Vec::with_capacity(component.states.len());
            for state in &component.states {
                self.poll()?;
                let vertex = self
                    .product
                    .vertices
                    .get(state)
                    .copied()
                    .filter(|vertex| level.scope.contains(vertex))
                    .ok_or_else(|| reject(CertificateRule::NotPartition, state.clone()))?;
                if owner.insert(vertex, index).is_some() {
                    return Err(reject(CertificateRule::NotPartition, state.clone()));
                }
                vertices.push(vertex);
            }
            members.push(vertices);
        }
        let uncovered = level
            .scope
            .iter()
            .filter(|vertex| !owner.contains_key(vertex))
            .filter_map(|vertex| self.product.state(*vertex))
            .min();
        if let Some(state) = uncovered {
            return Err(reject(CertificateRule::NotPartition, state.clone()));
        }
        Ok((owner, members))
    }

    /// `BackwardEdge` at the source of the first computed edge, between two
    /// states of the scope, that goes to an earlier component.
    fn edges(
        &self,
        components: &[Component],
        members: &[Vec<Vertex>],
        owner: &BTreeMap<Vertex, usize>,
    ) -> Step<()> {
        for (index, (component, vertices)) in components.iter().zip(members).enumerate() {
            for (state, vertex) in component.states.iter().zip(vertices) {
                for edge in self.product.graph.edges_from(*vertex) {
                    self.poll()?;
                    if owner
                        .get(&edge.target)
                        .is_some_and(|target| *target < index)
                    {
                        return Err(reject(CertificateRule::BackwardEdge, state.clone()));
                    }
                }
            }
        }
        Ok(())
    }

    /// Check `component`'s witness over its vertices `members`, which the
    /// partition check found nonempty: the level its `UnfairStrong`
    /// refinement must pass, if it has one.
    fn witness<'c>(
        &self,
        component: &'c Component,
        members: &[Vertex],
        fallback: &ProductStateRef,
    ) -> Step<Option<Level<'c>>> {
        // The partition check rejected an empty component before this.
        let first = component.states.first().unwrap_or(fallback).clone();
        let fails = || reject(CertificateRule::WitnessFails, first.clone());
        let inside: BTreeSet<Vertex> = members.iter().copied().collect();
        let holds = match &component.witness {
            ComponentWitness::Trivial => {
                inside.len() == 1 && !self.taken_inside(&inside, |_| true)?
            }
            ComponentWitness::MissingAcceptance { set } => index(*set)
                .and_then(|set| self.product.graph.acceptance_sets().get(set))
                .is_some_and(|accepting| accepting.is_disjoint(&inside)),
            ComponentWitness::UnfairWeak { constraint } => match self.constraint(*constraint) {
                Some(constraint) if constraint.strength() == Strength::Weak => {
                    self.enabled_everywhere(&inside, constraint)?
                        && !self.taken_inside(&inside, |label| constraint.is_taken_by(label))?
                }
                Some(_) | None => false,
            },
            ComponentWitness::UnfairStrong { constraints, sub } => {
                let mut failing = Vec::with_capacity(constraints.len());
                for constraint in constraints {
                    match self.constraint(*constraint) {
                        Some(constraint) if constraint.strength() == Strength::Strong => {
                            failing.push(constraint);
                        }
                        Some(_) | None => return Err(fails()),
                    }
                }
                for constraint in &failing {
                    if self.taken_inside(&inside, |label| constraint.is_taken_by(label))? {
                        return Err(fails());
                    }
                }
                let mut scope = BTreeSet::new();
                for vertex in &inside {
                    self.poll()?;
                    let enabled = self.product.graph.enabled(*vertex);
                    let quiet = !failing.iter().any(|constraint| {
                        enabled.is_some_and(|enabled| constraint.is_enabled_in(enabled))
                    });
                    if quiet {
                        scope.insert(*vertex);
                    }
                }
                return Ok(Some(Level {
                    scope,
                    components: sub,
                    fallback: first,
                }));
            }
        };
        if holds {
            Ok(None)
        } else {
            Err(fails())
        }
    }

    fn constraint(&self, at: u32) -> Option<&'p Constraint> {
        index(at).and_then(|at| self.constraints.get(at))
    }

    fn enabled_everywhere(&self, inside: &BTreeSet<Vertex>, constraint: &Constraint) -> Step<bool> {
        for vertex in inside {
            self.poll()?;
            let enabled = self.product.graph.enabled(*vertex);
            if !enabled.is_some_and(|enabled| constraint.is_enabled_in(enabled)) {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Whether an edge with both ends in `inside` has a label `takes`
    /// admits.
    fn taken_inside(
        &self,
        inside: &BTreeSet<Vertex>,
        takes: impl Fn(EdgeLabel) -> bool,
    ) -> Step<bool> {
        for vertex in inside {
            for edge in self.product.graph.edges_from(*vertex) {
                self.poll()?;
                if inside.contains(&edge.target) && takes(edge.label) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }
}

/// A certificate index as a slice index; `None` when it does not fit, which
/// names no set or constraint.
fn index(at: u32) -> Option<usize> {
    usize::try_from(at).ok()
}

fn reject(rule: CertificateRule, at: ProductStateRef) -> TemporalCertificateCheck {
    TemporalCertificateCheck::Rejected(CertificateRejection {
        rule,
        at: CertificateLocus::ProductState(at),
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_eval::automaton::AutomatonStateKey;
    use qsl_foundation::digest::{DigestDomain, DigestRecord};
    use quire_exact::CancelCause;

    use super::*;

    const UPD_A: Identity = Identity::new(0);
    const UPD_B: Identity = Identity::new(1);

    fn product_state(model: [u8; 2], automaton: &str) -> ProductStateRef {
        let mut digest = [0; 32];
        digest[..2].copy_from_slice(&model);
        ProductStateRef {
            model_state: DigestRecord::mint(DigestDomain::SimulationStateKeyV1, digest),
            automaton_state: AutomatonStateKey::imported(automaton.as_bytes().to_vec()),
        }
    }

    /// ADR-018 §6's example subject with universe `{a, b}`, both versions
    /// 0, in product with the negation `eventually always not holds(v = 2)`
    /// of FR-126-AC-1's `always eventually holds(c.versionNumber = 2)` for
    /// `c = b`: `q0` carries the eventuality, `q1` its discharged `always`
    /// and is the one acceptance set. Each `attemptUpdate` takes a version
    /// from 0 to 1 to 2 and back to 0, and both are enabled everywhere. Its
    /// 15 product states are every `(va, vb, q0)` and `(va, vb, q1)` with
    /// `vb` not 2.
    struct Example {
        product: ProductGraph,
        /// FR-123's resolved weak `each` constraints: `upd(a)`, `upd(b)`.
        constraints: Vec<Constraint>,
    }

    fn reached(model: [u8; 2], automaton: &str) -> bool {
        automaton == "q0" || model[1] != 2
    }

    fn example() -> Example {
        let mut product = ProductGraph::new();
        let mut vertices = BTreeMap::new();
        for automaton in ["q0", "q1"] {
            for va in 0..3 {
                for vb in 0..3 {
                    if reached([va, vb], automaton) {
                        let vertex = product
                            .add_state(product_state([va, vb], automaton), [UPD_A, UPD_B])
                            .expect("a new state");
                        vertices.insert(([va, vb], automaton), vertex);
                    }
                }
            }
        }
        for (&(model, automaton), &from) in &vertices {
            let moves = [
                (UPD_A, [(model[0] + 1) % 3, model[1]]),
                (UPD_B, [model[0], (model[1] + 1) % 3]),
            ];
            let targets: &[&str] = if automaton == "q0" {
                &["q0", "q1"]
            } else {
                &["q1"]
            };
            for (identity, next) in moves {
                for target in targets {
                    if let Some(to) = vertices.get(&(next, *target)) {
                        product
                            .add_edge(from, *to, EdgeLabel::Transition(identity))
                            .expect("known vertices");
                    }
                }
            }
        }
        let accepting: Vec<Vertex> = vertices
            .iter()
            .filter(|((_, automaton), _)| *automaton == "q1")
            .map(|(_, vertex)| *vertex)
            .collect();
        product
            .add_acceptance_set(accepting)
            .expect("known vertices");
        Example {
            product,
            constraints: vec![Constraint::weak([UPD_A]), Constraint::weak([UPD_B])],
        }
    }

    fn layer(vb: Option<u8>, automaton: &str) -> Vec<ProductStateRef> {
        let mut states = Vec::new();
        for va in 0..3 {
            for b in 0..3 {
                if vb.is_none_or(|vb| vb == b) && reached([va, b], automaton) {
                    states.push(product_state([va, b], automaton));
                }
            }
        }
        states
    }

    /// EN-1's partition of the example: the `q0` layer reaches both
    /// accepting components, `{(*, 0, q1)}` reaches `{(*, 1, q1)}`.
    fn certificate() -> Vec<Component> {
        vec![
            Component {
                states: layer(None, "q0"),
                witness: ComponentWitness::MissingAcceptance { set: 0 },
            },
            Component {
                states: layer(Some(0), "q1"),
                witness: ComponentWitness::UnfairWeak { constraint: 1 },
            },
            Component {
                states: layer(Some(1), "q1"),
                witness: ComponentWitness::UnfairWeak { constraint: 1 },
            },
        ]
    }

    fn check(
        product: &ProductGraph,
        components: &[Component],
        constraints: &[Constraint],
    ) -> TemporalCertificateCheck {
        check_partition(product, components, constraints, &Cancel::new())
            .expect("the product holds states")
    }

    fn rejected(rule: CertificateRule, at: ProductStateRef) -> TemporalCertificateCheck {
        reject(rule, at)
    }

    /// The weak `each` proof's partition: its accepting components carry
    /// `UnfairWeak` naming `upd(b)`, enabled throughout and taken inside
    /// neither, and the check accepts it.
    #[trace("TC-526")]
    #[test]
    fn the_weak_each_partition_is_accepted() {
        let example = example();
        assert_eq!(example.product.states.len(), 15);
        let mut product = example.product.clone();
        assert_eq!(
            product.add_state(product_state([0, 0], "q0"), [UPD_A, UPD_B]),
            Err(ProductGraphError::DuplicateState)
        );
        assert_eq!(
            check(&example.product, &certificate(), &example.constraints),
            TemporalCertificateCheck::Accepted
        );
    }

    /// Each tampered partition of the example is rejected at its locus:
    /// `UnfairWeak` naming `upd(a)`, which edges inside `{(*, 0, q1)}`
    /// take; the accepting components listed before the component that
    /// reaches them; one state in two components; and a `Trivial` witness
    /// over a state with a self-edge.
    #[trace("TC-526", "FR-339-AC-2")]
    #[test]
    fn a_tampered_partition_is_rejected_at_its_locus() {
        let example = example();

        let mut taken = certificate();
        taken[1].witness = ComponentWitness::UnfairWeak { constraint: 0 };
        assert_eq!(
            check(&example.product, &taken, &example.constraints),
            rejected(CertificateRule::WitnessFails, taken[1].states[0].clone())
        );

        let mut backward = certificate();
        backward.rotate_left(1);
        assert_eq!(
            check(&example.product, &backward, &example.constraints),
            rejected(CertificateRule::BackwardEdge, product_state([0, 0], "q0"))
        );

        let mut twice = certificate();
        let shared = twice[1].states[0].clone();
        twice[2].states.push(shared.clone());
        assert_eq!(
            check(&example.product, &twice, &example.constraints),
            rejected(CertificateRule::NotPartition, shared)
        );

        let mut stutter = ProductGraph::new();
        let terminal = product_state([0, 0], "q1");
        let vertex = stutter
            .add_state(terminal.clone(), [])
            .expect("a new state");
        stutter
            .add_edge(vertex, vertex, EdgeLabel::Stutter)
            .expect("a known vertex");
        let trivial = [Component {
            states: vec![terminal.clone()],
            witness: ComponentWitness::Trivial,
        }];
        assert_eq!(
            check(&stutter, &trivial, &[]),
            rejected(CertificateRule::WitnessFails, terminal)
        );
    }

    /// A partition that leaves a reached state out, or names a state the
    /// checker never reached, is not a partition: the least uncovered state
    /// and the foreign state are the loci.
    #[trace("TC-526")]
    #[test]
    fn an_uncovered_or_foreign_state_is_not_a_partition() {
        let example = example();
        let mut uncovered = certificate();
        let dropped = uncovered[2].states.remove(0);
        assert_eq!(
            check(&example.product, &uncovered, &example.constraints),
            rejected(CertificateRule::NotPartition, dropped)
        );

        let mut foreign = certificate();
        let unreached = product_state([0, 2], "q1");
        foreign[2].states.push(unreached.clone());
        assert_eq!(
            check(&example.product, &foreign, &example.constraints),
            rejected(CertificateRule::NotPartition, unreached)
        );
    }

    /// ADR-019 AM-7: a two-state cycle `s -x-> t -y-> s` with a strong
    /// constraint on `z` enabled at `s` alone and never taken. Refining away
    /// `s` leaves `{t}`, which holds no cycle, so `UnfairStrong` holds; a
    /// refinement that keeps `s`, or names a constraint an inside edge
    /// takes, fails.
    #[trace("TC-526")]
    #[test]
    fn a_strong_refinement_is_checked_within_its_component() {
        let (x, y, z) = (Identity::new(0), Identity::new(1), Identity::new(2));
        let (s, t) = (product_state([0, 0], "q1"), product_state([1, 0], "q1"));
        let mut product = ProductGraph::new();
        let sv = product.add_state(s.clone(), [x, z]).expect("a new state");
        let tv = product.add_state(t.clone(), [y]).expect("a new state");
        product
            .add_edge(sv, tv, EdgeLabel::Transition(x))
            .expect("known vertices");
        product
            .add_edge(tv, sv, EdgeLabel::Transition(y))
            .expect("known vertices");
        let strong = |constraint, sub| {
            vec![Component {
                states: vec![s.clone(), t.clone()],
                witness: ComponentWitness::UnfairStrong {
                    constraints: vec![constraint],
                    sub,
                },
            }]
        };
        let only_t = || {
            vec![Component {
                states: vec![t.clone()],
                witness: ComponentWitness::Trivial,
            }]
        };
        let constraints = [Constraint::strong([z]), Constraint::strong([x])];
        assert_eq!(
            check(&product, &strong(0, only_t()), &constraints),
            TemporalCertificateCheck::Accepted
        );

        let keeps_s = vec![Component {
            states: vec![s.clone(), t.clone()],
            witness: ComponentWitness::Trivial,
        }];
        assert_eq!(
            check(&product, &strong(0, keeps_s), &constraints),
            rejected(CertificateRule::NotPartition, s.clone())
        );
        assert_eq!(
            check(&product, &strong(1, only_t()), &constraints),
            rejected(CertificateRule::WitnessFails, s.clone())
        );

        let weak = [Constraint::weak([z])];
        assert_eq!(
            check(&product, &strong(0, only_t()), &weak),
            rejected(CertificateRule::WitnessFails, s)
        );
    }

    /// A cancelled handle stops the check with its source; it is never an
    /// acceptance.
    #[trace("TC-526")]
    #[test]
    fn a_cancelled_check_stops_with_its_source() {
        let example = example();
        let cancel = Cancel::new();
        cancel.cancel(CancelCause::Requested);
        assert_eq!(
            check_partition(
                &example.product,
                &certificate(),
                &example.constraints,
                &cancel
            ),
            Ok(TemporalCertificateCheck::Stopped(
                IncompleteCause::Cancelled {
                    source: CancelCause::Requested,
                }
            ))
        );
    }
}

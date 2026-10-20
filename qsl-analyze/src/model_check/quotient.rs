// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 AQ-1 to AQ-8 (FR-153): fairness on the annotated quotient.
//!
//! A symmetry-reduced product keeps, on every edge, the permutation `π` its
//! target was canonicalised by (AQ-1). A `whole` constraint reads the same
//! on the quotient as on the subject. An `each` constraint names one
//! transition identity, which a permutation maps to another, so for each
//! SCC `C` the [`Analyzer`] builds a breadth-first spanning tree from the
//! entry `c0` (the SCC's first state in discovery order), the tree word
//! `u_c` of every node (the composition of `π⁻¹` along the tree path), and
//! the cycle group's generators: the non-identity closed-walk words `u_c ∘
//! π⁻¹ ∘ u_c'⁻¹` of `C`'s edges, in canonical edge order (AQ-2). An `each`
//! constraint's witness sets are the identities `u_c(t)` taken, enabled and
//! disabled in `C`, read in the entry's frame (AQ-4), and the tests read
//! their orbits under the generators, closed breadth-first, never by
//! enumerating the group (AQ-5, AQ-6). A component that fails a strong
//! constraint is refined as ADR-019 refines it: the quotient states `R`
//! where a lift enables a failing identity are removed and the SCCs of the
//! rest are analysed, each with its own cycle group.
//!
//! [`Analyzer::fair_loop`] builds the loop of a passing component as AQ-7
//! states, from the entry and the concrete entry's `σ`: one segment from
//! the entry back to it per obligation (acceptance sets, then `whole`
//! constraints, then each `each` identity in identity order).
//!
//! Every tree edge, word, generator and permutation application charges
//! the run's meter and polls its cancellation (AQ-8, FR-153 "Limits"). No
//! count of generators, identities or refinement rounds is capped.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::fairness::{components, EdgeLabel, FairnessError, Graph, Strength, Vertex};
use qsl_eval::simulation::{ExpansionStop, Permutation};
use quire_exact::{Cancel, Charge, ChargePoint, Meter};
use quire_semantic_value::fairness::FairnessGranularity;

use super::graph::ProductStateId;

/// One edge of the annotated quotient (AQ-1): its transition identity
/// (`None` for the terminal stutter edge), canonical target and `π`.
pub(crate) struct QuotientEdge<I> {
    /// The transition identity; `None` for the terminal stutter edge.
    pub(crate) label: Option<I>,
    /// The canonical target.
    pub(crate) target: ProductStateId,
    /// The permutation the target was canonicalised by.
    pub(crate) permutation: Permutation,
}

/// The annotated quotient: each state's full enabled set (ADR-019 SR-1)
/// and outgoing edges in canonical edge order, and the property
/// automaton's acceptance sets.
pub(crate) struct Quotient<I> {
    /// The identities enabled at each state, by discovery index.
    pub(crate) enabled: Vec<BTreeSet<I>>,
    /// Each state's outgoing edges, in canonical successor order.
    pub(crate) edges: Vec<Vec<QuotientEdge<I>>>,
    /// The acceptance sets.
    pub(crate) acceptance: Vec<BTreeSet<ProductStateId>>,
}

/// One fairness constraint as the quotient reads it.
pub(crate) struct QuotientConstraint<I> {
    /// Weak or strong (ADR-019).
    pub(crate) strength: Strength,
    /// `whole` or `each` (ADR-018 FA-1).
    pub(crate) granularity: FairnessGranularity,
    /// Every transition identity of the constraint's operation, a set closed
    /// under the run's group.
    pub(crate) identities: BTreeSet<I>,
}

/// FR-153's cycle group of one SCC.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CycleGroup {
    /// The SCC's first state in discovery order.
    pub(crate) entry: ProductStateId,
    /// Each node's tree word `u_c`.
    pub(crate) tree_word: BTreeMap<ProductStateId, Permutation>,
    /// The non-identity closed-walk words, without repeats, in canonical
    /// edge order.
    pub(crate) generators: Vec<Permutation>,
}

/// A cycle group and the spanning tree it was read from.
#[derive(Clone, Debug)]
pub(crate) struct Spanning {
    /// The cycle group.
    pub(crate) group: CycleGroup,
    /// Each non-entry node's tree edge: its source and its position among
    /// the source's edges.
    parent: BTreeMap<ProductStateId, (ProductStateId, usize)>,
}

/// An `each` constraint's witness sets in one SCC (AQ-4), in the entry's
/// frame, before closure.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct Witnesses<I> {
    /// `{ u_c(t) : edge (c, t, …) of C, t of O }`.
    pub(crate) taken: BTreeSet<I>,
    /// `{ u_c(d) : d of O enabled at c }`.
    pub(crate) enabled: BTreeSet<I>,
    /// `{ u_c(d) : d of O disabled at c }`.
    pub(crate) disabled: BTreeSet<I>,
}

/// What the filter found for one component, in the order it tests.
#[derive(Clone, Debug)]
pub(crate) enum Verdict {
    /// The component has no edge.
    Trivial,
    /// The component misses an acceptance set.
    MissingAcceptance,
    /// A weak constraint fails.
    UnfairWeak,
    /// Strong constraints fail: the states `R` where a lift enables a
    /// failing identity are removed, and the SCCs of the rest, which
    /// together hold every state not removed, follow as the analyses whose
    /// `parent` is this one.
    UnfairStrong,
    /// The component holds a fair accepting cycle.
    Fair,
}

/// One component and what the filter found for it.
#[derive(Clone, Debug)]
pub(crate) struct Analysis {
    /// The component's states, ascending.
    pub(crate) states: Vec<ProductStateId>,
    /// Its spanning tree and cycle group; `None` for a component the filter
    /// rejected before building one.
    pub(crate) spanning: Option<Spanning>,
    /// What the filter found.
    pub(crate) verdict: Verdict,
    /// The index of the component whose refinement this one is a component
    /// of; `None` for an SCC of the analysed set.
    pub(crate) parent: Option<usize>,
}

impl Analysis {
    /// The first fair component among `analyses`, which [`Analyzer::analyze`]
    /// lists depth first through each refinement (ADR-019 FS-6).
    pub(crate) fn first_fair_in(analyses: &[Self]) -> Option<&Self> {
        analyses
            .iter()
            .find(|analysis| matches!(analysis.verdict, Verdict::Fair))
    }
}

/// A component waiting in [`Analyzer::analyze`]'s worklist.
struct Pending {
    states: Vec<ProductStateId>,
    trivial: bool,
    parent: Option<usize>,
}

/// What classifying one component found, and the states its refinement
/// keeps when strong constraints fail.
struct Classified {
    analysis: Analysis,
    refined: Option<BTreeSet<ProductStateId>>,
}

/// Why the analysis stopped.
#[derive(Debug)]
pub(crate) enum QuotientStop {
    /// The run's meter is spent.
    Meter,
    /// The run was cancelled.
    Cancelled,
    /// A permutation could not be applied to an identity.
    Rename(ExpansionStop),
    /// An invariant of the quotient broke.
    Fault(&'static str),
}

/// How a permutation acts on a transition identity (ADR-021 SYM-1).
pub(crate) type Rename<'r, I> = &'r dyn Fn(&I, &Permutation) -> Result<I, ExpansionStop>;

/// One step of a loop: the source state and the position of the edge among
/// its outgoing edges.
pub(crate) type LoopEdge = (ProductStateId, usize);

/// The annotated quotient's fairness filter for one product.
pub(crate) struct Analyzer<'q, I> {
    quotient: &'q Quotient<I>,
    constraints: &'q [QuotientConstraint<I>],
    rename: Rename<'q, I>,
    meter: &'q mut Meter,
    cancel: &'q Cancel,
    /// The quotient as the SCC kernel reads it.
    graph: Graph,
    /// The kernel's vertex of each quotient state.
    vertices: Vec<Vertex>,
}

impl<'q, I: Clone + Ord> Analyzer<'q, I> {
    /// The filter over `quotient` for `constraints`, charging `meter` (the
    /// run's) and polling `cancel`.
    ///
    /// # Errors
    ///
    /// [`QuotientStop::Fault`] when an edge names a state the quotient does
    /// not hold.
    pub(crate) fn new(
        quotient: &'q Quotient<I>,
        constraints: &'q [QuotientConstraint<I>],
        rename: Rename<'q, I>,
        meter: &'q mut Meter,
        cancel: &'q Cancel,
    ) -> Result<Self, QuotientStop> {
        let mut graph = Graph::new();
        let vertices: Vec<Vertex> = quotient
            .enabled
            .iter()
            .map(|_| graph.add_vertex([]))
            .collect();
        for (source, edges) in quotient.edges.iter().enumerate() {
            for edge in edges {
                let (Some(from), Some(to)) =
                    (vertices.get(source), vertices.get(edge.target.index()))
                else {
                    return Err(QuotientStop::Fault("an-edge-joins-two-quotient-states"));
                };
                graph
                    .add_edge(*from, *to, EdgeLabel::Stutter)
                    .map_err(|_| QuotientStop::Fault("an-edge-joins-two-quotient-states"))?;
            }
        }
        Ok(Self {
            quotient,
            constraints,
            rename,
            meter,
            cancel,
            graph,
            vertices,
        })
    }

    fn charge(&mut self) -> Result<(), QuotientStop> {
        if self.cancel.poll() && self.cancel.cause().is_some() {
            return Err(QuotientStop::Cancelled);
        }
        self.meter
            .charge(Charge::new(ChargePoint::GraphExpand))
            .map_err(|_| QuotientStop::Meter)
    }

    fn apply(&mut self, identity: &I, permutation: &Permutation) -> Result<I, QuotientStop> {
        self.charge()?;
        (self.rename)(identity, permutation).map_err(QuotientStop::Rename)
    }

    fn edges(&self, state: ProductStateId) -> &'q [QuotientEdge<I>] {
        self.quotient
            .edges
            .get(state.index())
            .map_or(&[], Vec::as_slice)
    }

    fn enabled(&self, state: ProductStateId) -> Option<&'q BTreeSet<I>> {
        self.quotient.enabled.get(state.index())
    }

    /// `fair` over `within`: its SCCs in discovery order, each classified,
    /// a component that fails a strong constraint refined and the SCCs of
    /// what is left analysed again. The result lists every component depth
    /// first (a refined component, then its refinement's components, then
    /// its next sibling), each naming the component it refines. A worklist
    /// replaces recursion, so a refinement that peels one state per round
    /// deepens no stack; each component charges the meter.
    ///
    /// # Errors
    ///
    /// [`QuotientStop`] when the meter is spent, the run is cancelled, or a
    /// permutation cannot be applied.
    pub(crate) fn analyze(
        &mut self,
        within: &BTreeSet<ProductStateId>,
    ) -> Result<Vec<Analysis>, QuotientStop> {
        let mut analyses = Vec::new();
        // Components still to classify, the next on top.
        let mut pending: Vec<Pending> = Vec::new();
        self.push_components(within, None, &mut pending)?;
        while let Some(next) = pending.pop() {
            self.charge()?;
            let index = analyses.len();
            if next.trivial {
                analyses.push(Analysis {
                    states: next.states,
                    spanning: None,
                    verdict: Verdict::Trivial,
                    parent: next.parent,
                });
                continue;
            }
            let Classified {
                mut analysis,
                refined,
            } = self.classify(next.states)?;
            analysis.parent = next.parent;
            analyses.push(analysis);
            if let Some(rest) = refined {
                self.push_components(&rest, Some(index), &mut pending)?;
            }
        }
        Ok(analyses)
    }

    /// The SCCs of `within`, pushed so the first in discovery order is on
    /// top.
    fn push_components(
        &mut self,
        within: &BTreeSet<ProductStateId>,
        parent: Option<usize>,
        pending: &mut Vec<Pending>,
    ) -> Result<(), QuotientStop> {
        let vertices: BTreeSet<Vertex> = within
            .iter()
            .map(|id| self.vertices.get(id.index()).copied())
            .collect::<Option<_>>()
            .ok_or(QuotientStop::Fault("a-state-is-a-quotient-state"))?;
        let found =
            components(&self.graph, &vertices, self.cancel).map_err(|error| match error {
                FairnessError::Cancelled => QuotientStop::Cancelled,
                FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop => {
                    QuotientStop::Fault("a-component-is-built-from-quotient-states")
                }
            })?;
        for component in found.into_iter().rev() {
            pending.push(Pending {
                states: component
                    .vertices()
                    .iter()
                    .map(|vertex| ProductStateId(vertex.index()))
                    .collect(),
                trivial: component.is_trivial(),
                parent,
            });
        }
        Ok(())
    }

    /// The verdict for a component that holds an edge: acceptance, then the
    /// weak constraints in fairness-set order, then the strong ones together.
    fn classify(&mut self, states: Vec<ProductStateId>) -> Result<Classified, QuotientStop> {
        let members: BTreeSet<ProductStateId> = states.iter().copied().collect();
        if self
            .quotient
            .acceptance
            .iter()
            .any(|accepting| accepting.is_disjoint(&members))
        {
            return Ok(Classified {
                analysis: Analysis {
                    states,
                    spanning: None,
                    verdict: Verdict::MissingAcceptance,
                    parent: None,
                },
                refined: None,
            });
        }
        let spanning = self.spanning(&members)?;
        let constraints = self.constraints;
        for constraint in constraints {
            if constraint.strength != Strength::Weak {
                continue;
            }
            let fair = match constraint.granularity {
                FairnessGranularity::Whole => {
                    !self.enabled_everywhere(&members, constraint)
                        || self.taken_inside(&members, constraint)
                }
                FairnessGranularity::Each => {
                    let witnesses = self.witnesses(&members, &spanning, constraint)?;
                    let seeds: BTreeSet<I> = witnesses
                        .taken
                        .union(&witnesses.disabled)
                        .cloned()
                        .collect();
                    let covered = self.close(seeds, &spanning.group.generators)?;
                    constraint.identities.is_subset(&covered)
                }
            };
            if !fair {
                return Ok(Classified {
                    analysis: Analysis {
                        states,
                        spanning: Some(spanning),
                        verdict: Verdict::UnfairWeak,
                        parent: None,
                    },
                    refined: None,
                });
            }
        }
        let mut failing = false;
        let mut removed = BTreeSet::new();
        for constraint in constraints {
            if constraint.strength != Strength::Strong {
                continue;
            }
            match constraint.granularity {
                FairnessGranularity::Whole => {
                    let enabled_at: Vec<ProductStateId> = members
                        .iter()
                        .copied()
                        .filter(|state| self.enabled_at(*state, constraint))
                        .collect();
                    if !enabled_at.is_empty() && !self.taken_inside(&members, constraint) {
                        failing = true;
                        removed.extend(enabled_at);
                    }
                }
                FairnessGranularity::Each => {
                    let witnesses = self.witnesses(&members, &spanning, constraint)?;
                    let enabled = self.close(witnesses.enabled, &spanning.group.generators)?;
                    let taken = self.close(witnesses.taken, &spanning.group.generators)?;
                    let failing_identities: BTreeSet<I> =
                        enabled.difference(&taken).cloned().collect();
                    if failing_identities.is_empty() {
                        continue;
                    }
                    failing = true;
                    // AQ-6: whether a lift of `c` enables a failing identity
                    // depends on `c` alone.
                    for state in &members {
                        if self.lift_enables(*state, &spanning, constraint, &failing_identities)? {
                            removed.insert(*state);
                        }
                    }
                }
            }
        }
        if !failing {
            return Ok(Classified {
                analysis: Analysis {
                    states,
                    spanning: Some(spanning),
                    verdict: Verdict::Fair,
                    parent: None,
                },
                refined: None,
            });
        }
        Ok(Classified {
            analysis: Analysis {
                states,
                spanning: Some(spanning),
                verdict: Verdict::UnfairStrong,
                parent: None,
            },
            refined: Some(members.difference(&removed).copied().collect()),
        })
    }

    fn enabled_at(&self, state: ProductStateId, constraint: &QuotientConstraint<I>) -> bool {
        self.enabled(state)
            .is_some_and(|enabled| !enabled.is_disjoint(&constraint.identities))
    }

    fn enabled_everywhere(
        &self,
        members: &BTreeSet<ProductStateId>,
        constraint: &QuotientConstraint<I>,
    ) -> bool {
        members
            .iter()
            .all(|state| self.enabled_at(*state, constraint))
    }

    /// Whether an edge inside `members` takes a `whole` constraint.
    fn taken_inside(
        &self,
        members: &BTreeSet<ProductStateId>,
        constraint: &QuotientConstraint<I>,
    ) -> bool {
        members.iter().any(|state| {
            self.edges(*state).iter().any(|edge| {
                members.contains(&edge.target)
                    && edge
                        .label
                        .as_ref()
                        .is_some_and(|label| constraint.identities.contains(label))
            })
        })
    }

    /// Whether `B` meets `u_c(E(c))`.
    fn lift_enables(
        &mut self,
        state: ProductStateId,
        spanning: &Spanning,
        constraint: &QuotientConstraint<I>,
        failing: &BTreeSet<I>,
    ) -> Result<bool, QuotientStop> {
        let word = spanning
            .group
            .tree_word
            .get(&state)
            .ok_or(QuotientStop::Fault("a-component-state-has-a-tree-word"))?;
        let Some(enabled) = self.enabled(state) else {
            return Err(QuotientStop::Fault("a-quotient-state-has-an-enabled-set"));
        };
        for identity in enabled.intersection(&constraint.identities) {
            if failing.contains(&self.apply(identity, word)?) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// FR-153 "Spanning tree" and "Cycle group" for the SCC `members`.
    ///
    /// # Errors
    ///
    /// [`QuotientStop`] when the meter is spent or the run is cancelled.
    pub(crate) fn spanning(
        &mut self,
        members: &BTreeSet<ProductStateId>,
    ) -> Result<Spanning, QuotientStop> {
        let entry = *members
            .first()
            .ok_or(QuotientStop::Fault("a-component-has-a-state"))?;
        let mut tree_word = BTreeMap::from([(entry, Permutation::identity())]);
        let mut parent = BTreeMap::new();
        let mut queue = VecDeque::from([entry]);
        while let Some(source) = queue.pop_front() {
            let word = tree_word
                .get(&source)
                .cloned()
                .ok_or(QuotientStop::Fault("a-queued-state-has-a-word"))?;
            for (position, edge) in self.edges(source).iter().enumerate() {
                self.charge()?;
                if !members.contains(&edge.target) || tree_word.contains_key(&edge.target) {
                    continue;
                }
                tree_word.insert(edge.target, word.compose(&edge.permutation.inverse()));
                parent.insert(edge.target, (source, position));
                queue.push_back(edge.target);
            }
        }
        let mut generators: Vec<Permutation> = Vec::new();
        for source in members {
            for edge in self.edges(*source) {
                if !members.contains(&edge.target) {
                    continue;
                }
                self.charge()?;
                let (Some(from), Some(to)) = (tree_word.get(source), tree_word.get(&edge.target))
                else {
                    return Err(QuotientStop::Fault("a-component-is-strongly-connected"));
                };
                let word = from
                    .compose(&edge.permutation.inverse())
                    .compose(&to.inverse());
                if !word.is_identity() && !generators.contains(&word) {
                    generators.push(word);
                }
            }
        }
        Ok(Spanning {
            group: CycleGroup {
                entry,
                tree_word,
                generators,
            },
            parent,
        })
    }

    /// FR-153 "Witness sets" of `constraint` in the SCC `members`.
    ///
    /// # Errors
    ///
    /// [`QuotientStop`] when the meter is spent, the run is cancelled, or a
    /// permutation cannot be applied.
    pub(crate) fn witnesses(
        &mut self,
        members: &BTreeSet<ProductStateId>,
        spanning: &Spanning,
        constraint: &QuotientConstraint<I>,
    ) -> Result<Witnesses<I>, QuotientStop> {
        let mut witnesses = Witnesses {
            taken: BTreeSet::new(),
            enabled: BTreeSet::new(),
            disabled: BTreeSet::new(),
        };
        for state in members {
            let word = spanning
                .group
                .tree_word
                .get(state)
                .ok_or(QuotientStop::Fault("a-component-state-has-a-tree-word"))?;
            for edge in self.edges(*state) {
                let Some(label) = &edge.label else { continue };
                if members.contains(&edge.target) && constraint.identities.contains(label) {
                    witnesses.taken.insert(self.apply(label, word)?);
                }
            }
            let enabled = self
                .enabled(*state)
                .ok_or(QuotientStop::Fault("a-quotient-state-has-an-enabled-set"))?;
            for identity in &constraint.identities {
                let image = self.apply(identity, word)?;
                if enabled.contains(identity) {
                    witnesses.enabled.insert(image);
                } else {
                    witnesses.disabled.insert(image);
                }
            }
        }
        Ok(witnesses)
    }

    /// FR-153 "Orbit closure": `seeds` closed under `generators` by
    /// breadth-first application.
    ///
    /// # Errors
    ///
    /// [`QuotientStop`] when the meter is spent, the run is cancelled, or a
    /// permutation cannot be applied.
    pub(crate) fn close(
        &mut self,
        seeds: BTreeSet<I>,
        generators: &[Permutation],
    ) -> Result<BTreeSet<I>, QuotientStop> {
        let mut queue: VecDeque<I> = seeds.iter().cloned().collect();
        let mut closed = seeds;
        while let Some(identity) = queue.pop_front() {
            for generator in generators {
                let image = self.apply(&identity, generator)?;
                if closed.insert(image.clone()) {
                    queue.push_back(image);
                }
            }
        }
        Ok(closed)
    }

    /// ADR-021 AQ-7: one pass of the loop of the fair component `fair` from
    /// its entry, whose concrete state is `sigma(entry)`. Each obligation is
    /// one segment from the entry back to it: a state of each acceptance set
    /// the entry is not in; for each `whole` constraint an edge that takes it
    /// or (weak) a state that disables it; for each identity `e` of each
    /// `each` constraint, the closed walks of a generator word `h`, the tree
    /// path to a witness `x` at `c` with `σ h u_c(x) = e`, and `x` taken (or
    /// the stay at `c`, where a weak constraint's `x` is disabled). A strong
    /// obligation is owed only where the component enables it. With no
    /// segment the pass is the shortest cycle through the entry.
    ///
    /// # Errors
    ///
    /// [`QuotientStop`] when the meter is spent, the run is cancelled, a
    /// permutation cannot be applied, or `fair` is no fair component.
    pub(crate) fn fair_loop(
        &mut self,
        fair: &Analysis,
        sigma: &Permutation,
    ) -> Result<Vec<LoopEdge>, QuotientStop> {
        let spanning = fair
            .spanning
            .as_ref()
            .ok_or(QuotientStop::Fault("a-fair-component-has-a-cycle-group"))?;
        let members: BTreeSet<ProductStateId> = fair.states.iter().copied().collect();
        let mut walk = Walker::new(self, &members, spanning, sigma.clone())?;
        let entry = spanning.group.entry;
        let quotient = self.quotient;
        for set in &quotient.acceptance {
            if set.contains(&entry) {
                continue;
            }
            let state = members
                .iter()
                .copied()
                .find(|state| set.contains(state))
                .ok_or(QuotientStop::Fault("a-fair-component-meets-every-set"))?;
            walk.visit(self, state)?;
        }
        let constraints = self.constraints;
        for constraint in constraints {
            if constraint.granularity != FairnessGranularity::Whole {
                continue;
            }
            let owed = match constraint.strength {
                Strength::Weak => true,
                Strength::Strong => self.enabled_somewhere(&members, constraint),
            };
            if !owed {
                continue;
            }
            if constraint.strength == Strength::Weak {
                if let Some(state) = members
                    .iter()
                    .copied()
                    .find(|state| !self.enabled_at(*state, constraint))
                {
                    walk.visit(self, state)?;
                    continue;
                }
            }
            let edge = self
                .first_taking(&members, &constraint.identities)
                .ok_or(QuotientStop::Fault("a-fair-component-takes-its-constraint"))?;
            walk.take(self, edge)?;
        }
        for constraint in constraints {
            if constraint.granularity != FairnessGranularity::Each {
                continue;
            }
            // A strong identity is owed only where a lift enables it.
            let owed = match constraint.strength {
                Strength::Weak => None,
                Strength::Strong => {
                    let witnesses = self.witnesses(&members, spanning, constraint)?;
                    Some(self.close(witnesses.enabled, &spanning.group.generators)?)
                }
            };
            let seeds = self.seeds(&members, spanning, constraint)?;
            for identity in &constraint.identities {
                let wanted = self.apply(identity, &walk.sigma.inverse())?;
                if owed.as_ref().is_some_and(|owed| !owed.contains(&wanted)) {
                    continue;
                }
                let (path, witness) = walk.solve(self, &seeds, &wanted)?;
                for generator in path.iter().rev() {
                    let edge = walk
                        .closed
                        .get(*generator)
                        .map(|(_, edge)| *edge)
                        .ok_or(QuotientStop::Fault("a-generator-has-a-closed-walk"))?;
                    walk.take(self, edge)?;
                }
                match witness {
                    Seed::Taken(edge) => walk.take(self, edge)?,
                    Seed::Disabled(state) => walk.visit(self, state)?,
                }
            }
        }
        if walk.edges.is_empty() {
            walk.cycle(self)?;
        }
        Ok(walk.edges)
    }

    fn enabled_somewhere(
        &self,
        members: &BTreeSet<ProductStateId>,
        constraint: &QuotientConstraint<I>,
    ) -> bool {
        members
            .iter()
            .any(|state| self.enabled_at(*state, constraint))
    }

    /// The first edge inside `members`, in canonical edge order, labelled
    /// with an identity of `identities`.
    fn first_taking(
        &self,
        members: &BTreeSet<ProductStateId>,
        identities: &BTreeSet<I>,
    ) -> Option<LoopEdge> {
        members.iter().find_map(|state| {
            self.edges(*state)
                .iter()
                .position(|edge| {
                    members.contains(&edge.target)
                        && edge
                            .label
                            .as_ref()
                            .is_some_and(|label| identities.contains(label))
                })
                .map(|position| (*state, position))
        })
    }

    /// The witnesses an obligation may be met by, each with its identity in
    /// the entry's frame, in canonical order (state, then taken edges, then
    /// disabled identities): taken edges, and for a weak constraint the
    /// identities disabled at a state.
    fn seeds(
        &mut self,
        members: &BTreeSet<ProductStateId>,
        spanning: &Spanning,
        constraint: &QuotientConstraint<I>,
    ) -> Result<Vec<(I, Seed)>, QuotientStop> {
        let mut seeds = Vec::new();
        for state in members {
            let word = spanning
                .group
                .tree_word
                .get(state)
                .ok_or(QuotientStop::Fault("a-component-state-has-a-tree-word"))?;
            for (position, edge) in self.edges(*state).iter().enumerate() {
                let Some(label) = &edge.label else { continue };
                if members.contains(&edge.target) && constraint.identities.contains(label) {
                    seeds.push((self.apply(label, word)?, Seed::Taken((*state, position))));
                }
            }
            if constraint.strength == Strength::Weak {
                let enabled = self
                    .enabled(*state)
                    .ok_or(QuotientStop::Fault("a-quotient-state-has-an-enabled-set"))?;
                for identity in constraint.identities.difference(enabled) {
                    seeds.push((self.apply(identity, word)?, Seed::Disabled(*state)));
                }
            }
        }
        Ok(seeds)
    }
}

/// A witness of an `each` obligation.
#[derive(Clone, Copy)]
enum Seed {
    /// The edge takes it.
    Taken(LoopEdge),
    /// The state disables it.
    Disabled(ProductStateId),
}

/// A loop being built from the entry: its edges and `σ` at its end.
struct Walker<'s> {
    spanning: &'s Spanning,
    entry: ProductStateId,
    /// Each state's next edge on a shortest path to the entry inside the
    /// component.
    toward_entry: BTreeMap<ProductStateId, LoopEdge>,
    /// The closed walks the generator search uses: each edge of the
    /// component's walk from the entry by the tree, the edge, and the
    /// shortest way back, with its word; non-identity words only, without
    /// repeats, in canonical edge order.
    closed: Vec<(Permutation, LoopEdge)>,
    sigma: Permutation,
    edges: Vec<LoopEdge>,
}

impl<'s> Walker<'s> {
    fn new<I: Clone + Ord>(
        analyzer: &mut Analyzer<'_, I>,
        members: &BTreeSet<ProductStateId>,
        spanning: &'s Spanning,
        sigma: Permutation,
    ) -> Result<Self, QuotientStop> {
        let entry = spanning.group.entry;
        // A breadth-first search from the entry over reversed edges, the
        // predecessors in canonical edge order.
        let mut incoming: BTreeMap<ProductStateId, Vec<LoopEdge>> = BTreeMap::new();
        for source in members {
            for (position, edge) in analyzer.edges(*source).iter().enumerate() {
                if members.contains(&edge.target) {
                    incoming
                        .entry(edge.target)
                        .or_default()
                        .push((*source, position));
                }
            }
        }
        let mut toward_entry = BTreeMap::new();
        let mut back_word = BTreeMap::from([(entry, Permutation::identity())]);
        let mut queue = VecDeque::from([entry]);
        while let Some(target) = queue.pop_front() {
            let word = back_word
                .get(&target)
                .cloned()
                .ok_or(QuotientStop::Fault("a-queued-state-has-a-word"))?;
            for (source, position) in incoming.get(&target).into_iter().flatten() {
                analyzer.charge()?;
                if back_word.contains_key(source) {
                    continue;
                }
                let edge = analyzer
                    .edges(*source)
                    .get(*position)
                    .ok_or(QuotientStop::Fault("an-incoming-edge-exists"))?;
                back_word.insert(*source, edge.permutation.inverse().compose(&word));
                toward_entry.insert(*source, (*source, *position));
                queue.push_back(*source);
            }
        }
        let mut closed: Vec<(Permutation, LoopEdge)> = Vec::new();
        for source in members {
            for (position, edge) in analyzer.edges(*source).iter().enumerate() {
                if !members.contains(&edge.target) {
                    continue;
                }
                analyzer.charge()?;
                let (Some(tree), Some(back)) = (
                    spanning.group.tree_word.get(source),
                    back_word.get(&edge.target),
                ) else {
                    return Err(QuotientStop::Fault("a-component-is-strongly-connected"));
                };
                let word = tree.compose(&edge.permutation.inverse()).compose(back);
                if !word.is_identity() && !closed.iter().any(|(held, _)| *held == word) {
                    closed.push((word, (*source, position)));
                }
            }
        }
        Ok(Self {
            spanning,
            entry,
            toward_entry,
            closed,
            sigma,
            edges: Vec::new(),
        })
    }

    fn step<I: Clone + Ord>(
        &mut self,
        analyzer: &mut Analyzer<'_, I>,
        edge: LoopEdge,
    ) -> Result<ProductStateId, QuotientStop> {
        analyzer.charge()?;
        let held = analyzer
            .edges(edge.0)
            .get(edge.1)
            .ok_or(QuotientStop::Fault("a-loop-edge-exists"))?;
        self.sigma = self.sigma.compose(&held.permutation.inverse());
        self.edges.push(edge);
        Ok(held.target)
    }

    /// The tree path from the entry to `state`.
    fn tree<I: Clone + Ord>(
        &mut self,
        analyzer: &mut Analyzer<'_, I>,
        state: ProductStateId,
    ) -> Result<(), QuotientStop> {
        let mut path = Vec::new();
        let mut cursor = state;
        while cursor != self.entry {
            let edge = *self
                .spanning
                .parent
                .get(&cursor)
                .ok_or(QuotientStop::Fault("a-component-state-has-a-tree-edge"))?;
            path.push(edge);
            cursor = edge.0;
        }
        for edge in path.into_iter().rev() {
            self.step(analyzer, edge)?;
        }
        Ok(())
    }

    /// The shortest way back from `state` to the entry.
    fn back<I: Clone + Ord>(
        &mut self,
        analyzer: &mut Analyzer<'_, I>,
        mut state: ProductStateId,
    ) -> Result<(), QuotientStop> {
        while state != self.entry {
            let edge = *self
                .toward_entry
                .get(&state)
                .ok_or(QuotientStop::Fault("a-component-state-reaches-the-entry"))?;
            state = self.step(analyzer, edge)?;
        }
        Ok(())
    }

    /// The tree path to `state` and back.
    fn visit<I: Clone + Ord>(
        &mut self,
        analyzer: &mut Analyzer<'_, I>,
        state: ProductStateId,
    ) -> Result<(), QuotientStop> {
        self.tree(analyzer, state)?;
        self.back(analyzer, state)
    }

    /// The tree path to `edge`'s source, `edge`, and the way back.
    fn take<I: Clone + Ord>(
        &mut self,
        analyzer: &mut Analyzer<'_, I>,
        edge: LoopEdge,
    ) -> Result<(), QuotientStop> {
        self.tree(analyzer, edge.0)?;
        let target = self.step(analyzer, edge)?;
        self.back(analyzer, target)
    }

    /// The shortest cycle through the entry: its first edge, in canonical
    /// order, of the least length back.
    fn cycle<I: Clone + Ord>(
        &mut self,
        analyzer: &mut Analyzer<'_, I>,
    ) -> Result<(), QuotientStop> {
        let mut best: Option<(usize, LoopEdge)> = None;
        for (position, edge) in analyzer.edges(self.entry).iter().enumerate() {
            if edge.target != self.entry && !self.toward_entry.contains_key(&edge.target) {
                continue;
            }
            let mut length = 1_usize;
            let mut cursor = edge.target;
            while cursor != self.entry {
                analyzer.charge()?;
                let next = self
                    .toward_entry
                    .get(&cursor)
                    .ok_or(QuotientStop::Fault("a-component-state-reaches-the-entry"))?;
                cursor = analyzer
                    .edges(next.0)
                    .get(next.1)
                    .ok_or(QuotientStop::Fault("an-incoming-edge-exists"))?
                    .target;
                length += 1;
            }
            if best.is_none_or(|(least, _)| length < least) {
                best = Some((length, (self.entry, position)));
            }
        }
        let (_, edge) = best.ok_or(QuotientStop::Fault("a-fair-component-holds-a-cycle"))?;
        let target = self.step(analyzer, edge)?;
        self.back(analyzer, target)
    }

    /// AQ-7's word search: the generators, by index into `closed`, along a
    /// breadth-first path from a seed's identity to `wanted` (each
    /// application `g(x)`), and that seed. Ties go to the earlier seed and
    /// the earlier generator.
    fn solve<I: Clone + Ord>(
        &self,
        analyzer: &mut Analyzer<'_, I>,
        seeds: &[(I, Seed)],
        wanted: &I,
    ) -> Result<(Vec<usize>, Seed), QuotientStop> {
        let mut reached: BTreeMap<I, Option<(I, usize)>> = BTreeMap::new();
        let mut origin: BTreeMap<I, Seed> = BTreeMap::new();
        let mut queue = VecDeque::new();
        for (identity, seed) in seeds {
            if !reached.contains_key(identity) {
                reached.insert(identity.clone(), None);
                origin.insert(identity.clone(), *seed);
                queue.push_back(identity.clone());
            }
        }
        while !reached.contains_key(wanted) {
            let Some(identity) = queue.pop_front() else {
                return Err(QuotientStop::Fault(
                    "a-fair-component-covers-its-identities",
                ));
            };
            let from = *origin
                .get(&identity)
                .ok_or(QuotientStop::Fault("a-reached-identity-has-a-seed"))?;
            for (index, (word, _)) in self.closed.iter().enumerate() {
                let image = analyzer.apply(&identity, word)?;
                if let std::collections::btree_map::Entry::Vacant(slot) = reached.entry(image) {
                    let image = slot.key().clone();
                    slot.insert(Some((identity.clone(), index)));
                    origin.insert(image.clone(), from);
                    queue.push_back(image);
                }
            }
        }
        let mut path = Vec::new();
        let mut cursor = wanted.clone();
        while let Some(Some((previous, generator))) = reached.get(&cursor) {
            path.push(*generator);
            cursor = previous.clone();
        }
        path.reverse();
        let seed = *origin
            .get(wanted)
            .ok_or(QuotientStop::Fault("a-reached-identity-has-a-seed"))?;
        Ok((path, seed))
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;
    use qsl_semantics::model::key::DeclarationKey;

    use super::*;

    fn population() -> DeclarationKey {
        DeclarationKey {
            package: "test/toy".to_owned(),
            node: "ix://test/toy/objects".to_owned(),
        }
    }

    fn swap(left: &str, right: &str) -> Permutation {
        Permutation::of(
            &population(),
            [
                (left.to_owned(), right.to_owned()),
                (right.to_owned(), left.to_owned()),
            ],
        )
    }

    /// A toy identity `op(k)`, renamed in its receiver `k`.
    fn rename(identity: &str, permutation: &Permutation) -> Result<String, ExpansionStop> {
        let (operation, rest) = identity.split_once('(').expect("op(k)");
        let key = rest.trim_end_matches(')');
        Ok(format!(
            "{operation}({})",
            permutation.image(&population(), key)
        ))
    }

    fn ids(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    /// A toy subject state: each object's value, by key.
    type Toy<V> = BTreeMap<String, V>;

    /// The sort canonicaliser over the class `class` (SYM-7): the class's
    /// keys sorted by their objects' values, ties in key order, reassigned
    /// to the class's keys in key order.
    fn canonical<V: Clone + Ord>(state: &Toy<V>, class: &[&str]) -> (Toy<V>, Permutation) {
        let mut sorted: Vec<(&V, &str)> = class.iter().map(|key| (&state[*key], *key)).collect();
        sorted.sort_by(|left, right| left.0.cmp(right.0));
        let permutation = Permutation::of(
            &population(),
            sorted
                .iter()
                .zip(class)
                .map(|((_, from), to)| ((*from).to_owned(), (*to).to_owned())),
        );
        let renamed = state
            .iter()
            .map(|(key, value)| {
                (
                    permutation.image(&population(), key).to_owned(),
                    value.clone(),
                )
            })
            .collect();
        (renamed, permutation)
    }

    /// The annotated quotient of a toy subject, explored breadth-first from
    /// `initial` in canonical order (edges by identity, then target): each
    /// state keeps its canonical form and enabled set, each edge its `π`.
    /// A successor `keep` rejects is no product state, and its edge is
    /// dropped (the property automaton has no move there).
    fn explore<V: Clone + Ord>(
        initial: Toy<V>,
        class: &[&str],
        successors: impl Fn(&Toy<V>) -> Vec<(String, Toy<V>)>,
        keep: impl Fn(&Toy<V>) -> bool,
    ) -> (Vec<Toy<V>>, Quotient<String>) {
        let mut states = vec![canonical(&initial, class).0];
        let mut quotient = Quotient {
            enabled: Vec::new(),
            edges: Vec::new(),
            acceptance: Vec::new(),
        };
        let mut index = 0;
        while index < states.len() {
            let state = states[index].clone();
            let mut steps: Vec<(String, Toy<V>, Permutation)> = successors(&state)
                .into_iter()
                .map(|(identity, next)| {
                    let (next, permutation) = canonical(&next, class);
                    (identity, next, permutation)
                })
                .collect();
            steps.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
            quotient.enabled.push(
                steps
                    .iter()
                    .map(|(identity, ..)| identity.clone())
                    .collect(),
            );
            let mut edges = Vec::new();
            for (identity, next, permutation) in steps {
                if !keep(&next) {
                    continue;
                }
                let target = states
                    .iter()
                    .position(|known| *known == next)
                    .unwrap_or_else(|| {
                        states.push(next.clone());
                        states.len() - 1
                    });
                edges.push(QuotientEdge {
                    label: Some(identity),
                    target: ProductStateId(target),
                    permutation,
                });
            }
            quotient.edges.push(edges);
            index += 1;
        }
        (states, quotient)
    }

    /// ADR-021 §7.1's subject: three versions modulo 3, one `upd` per
    /// receiver, reduced by the stabiliser of `a` (the class `[b, c]`).
    fn configs() -> (Vec<Toy<u8>>, Quotient<String>) {
        let initial: Toy<u8> = ["a", "b", "c"].map(|key| (key.to_owned(), 0)).into();
        explore(
            initial,
            &["b", "c"],
            |state| {
                state
                    .keys()
                    .map(|key| {
                        let mut next = state.clone();
                        next.insert(key.clone(), (state[key] + 1) % 3);
                        (format!("upd({key})"), next)
                    })
                    .collect()
            },
            |_| true,
        )
    }

    fn versions(state: &Toy<u8>) -> (u8, u8, u8) {
        (state["a"], state["b"], state["c"])
    }

    fn weak_each(identities: BTreeSet<String>) -> QuotientConstraint<String> {
        QuotientConstraint {
            strength: Strength::Weak,
            granularity: FairnessGranularity::Each,
            identities,
        }
    }

    fn limits(work_units: u64) -> quire_exact::ScalarLimits {
        quire_exact::ScalarLimits {
            integer_bits: 1 << 20,
            decimal_digits: 1 << 20,
            scale_expansion: 1 << 20,
            text_input_bytes: 1 << 20,
            text_scalars: 1 << 20,
            normalized_scalars: 1 << 20,
            unit_edges: 1 << 20,
            value_occurrences: 1 << 20,
            work_units,
            result_units: 1 << 30,
        }
    }

    fn filter<'q>(
        quotient: &'q Quotient<String>,
        constraints: &'q [QuotientConstraint<String>],
        meter: &'q mut Meter,
        cancel: &'q Cancel,
    ) -> Analyzer<'q, String> {
        Analyzer::new(
            quotient,
            constraints,
            &|identity: &String, permutation: &Permutation| rename(identity, permutation),
            meter,
            cancel,
        )
        .expect("the quotient is well formed")
    }

    fn id(states: &[Toy<u8>], values: (u8, u8, u8)) -> ProductStateId {
        ProductStateId(
            states
                .iter()
                .position(|state| versions(state) == values)
                .expect("a canonical state"),
        )
    }

    /// FR-153-AC-1: under `fair weak each attemptUpdate` for `c = a`, the
    /// accepting SCC with `va = 0` (the negation's `q1` states, `a` never
    /// 2) has the six canonical states `(0, y, z)`, `y <= z`, and entry
    /// `(0,0,0)`; the tree reaches `(0,0,1)` by `upd(b)` with `π = (b c)`, so
    /// its word is `(b c)`; the non-tree edge `(0,0,0) -upd(c)-> (0,0,1)`
    /// gives the one generator `(b c)`; `W_taken` closes to `{upd(b),
    /// upd(c)}`, `W_disabled` is empty, and the SCC fails weak `each` for
    /// want of `upd(a)`. The SCC with `va = 1` fails the same way, so no
    /// component is fair. Repeating the analysis gives equal generators.
    #[trace("TC-570", "FR-153-AC-1", "FR-153-AC-4")]
    #[test]
    fn weak_each_rejects_the_cycle_that_never_updates_the_bound_object() {
        let (states, mut quotient) = configs();
        assert_eq!(states.len(), 18, "the stabiliser quotient");
        let waiting: BTreeSet<ProductStateId> = (0..states.len())
            .filter(|index| states[*index]["a"] != 2)
            .map(ProductStateId)
            .collect();
        quotient.acceptance = vec![waiting.clone()];
        let constraints = [weak_each(ids(&["upd(a)", "upd(b)", "upd(c)"]))];
        let cancel = Cancel::new();
        let mut meter = Meter::new(limits(1 << 30));
        let mut analyzer = filter(&quotient, &constraints, &mut meter, &cancel);
        let analyses = analyzer.analyze(&waiting).expect("the analysis completes");
        let accepting = analyses
            .iter()
            .find(|analysis| analysis.states.contains(&id(&states, (0, 0, 0))))
            .expect("the va = 0 component");
        let mut members: Vec<(u8, u8, u8)> = accepting
            .states
            .iter()
            .map(|state| versions(&states[state.index()]))
            .collect();
        members.sort_unstable();
        assert_eq!(
            members,
            [
                (0, 0, 0),
                (0, 0, 1),
                (0, 0, 2),
                (0, 1, 1),
                (0, 1, 2),
                (0, 2, 2)
            ]
        );
        let spanning = accepting.spanning.as_ref().expect("a cycle group");
        assert_eq!(spanning.group.entry, id(&states, (0, 0, 0)));
        assert_eq!(
            spanning.group.tree_word[&id(&states, (0, 0, 1))],
            swap("b", "c")
        );
        assert_eq!(spanning.group.generators, [swap("b", "c")]);
        assert!(matches!(accepting.verdict, Verdict::UnfairWeak));
        let component: BTreeSet<ProductStateId> = accepting.states.iter().copied().collect();
        let witnesses = analyzer
            .witnesses(&component, spanning, &constraints[0])
            .expect("the witnesses");
        assert!(witnesses.disabled.is_empty());
        let taken = analyzer
            .close(witnesses.taken, &spanning.group.generators)
            .expect("the closure");
        assert_eq!(taken, ids(&["upd(b)", "upd(c)"]));
        assert!(Analysis::first_fair_in(&analyses).is_none());
        let again = analyzer.spanning(&component).expect("the same tree");
        assert_eq!(again.group, spanning.group);
    }

    /// Every component of `analyses` with its cycle group, in analysis
    /// order: what a repeated analysis must reproduce (FR-153-AC-4).
    fn groups(analyses: &[Analysis]) -> Vec<(Vec<ProductStateId>, Option<CycleGroup>)> {
        analyses
            .iter()
            .map(|analysis| {
                (
                    analysis.states.clone(),
                    analysis
                        .spanning
                        .as_ref()
                        .map(|spanning| spanning.group.clone()),
                )
            })
            .collect()
    }

    /// The identities a loop takes, concretely: each edge's label under `σ`,
    /// `σ` updated to `σ ∘ π⁻¹`, from `σ` the identity.
    fn concrete_labels(quotient: &Quotient<String>, walk: &[LoopEdge]) -> Vec<String> {
        let mut sigma = Permutation::identity();
        let mut labels = Vec::new();
        for (source, position) in walk {
            let edge = &quotient.edges[source.index()][*position];
            labels
                .push(rename(edge.label.as_ref().expect("a model edge"), &sigma).expect("renamed"));
            sigma = sigma.compose(&edge.permutation.inverse());
        }
        labels
    }

    /// FR-153-AC-2: for `eventually always holds(c.versionNumber != 2)`
    /// (negation `always eventually (a = 2)`), the accepting SCC is all 18
    /// canonical states with edges of all three labels, its cycle group is
    /// generated by `(b c)`, `W_taken` closes over `upd(a)`, `upd(b)` and
    /// `upd(c)`, and the SCC passes; AQ-7's loop from `(0,0,0)` takes all
    /// three concretely and returns to the entry. Repeating the analysis
    /// gives equal generators (FR-153-AC-4).
    #[trace("TC-570", "FR-153-AC-2", "FR-153-AC-4")]
    #[test]
    fn weak_each_passes_the_component_whose_orbits_cover_every_identity() {
        let (states, mut quotient) = configs();
        let all: BTreeSet<ProductStateId> = (0..states.len()).map(ProductStateId).collect();
        quotient.acceptance = vec![all
            .iter()
            .copied()
            .filter(|state| states[state.index()]["a"] == 2)
            .collect()];
        let labels: BTreeSet<String> = quotient
            .edges
            .iter()
            .flatten()
            .filter_map(|edge| edge.label.clone())
            .collect();
        assert_eq!(labels, ids(&["upd(a)", "upd(b)", "upd(c)"]));
        let constraints = [weak_each(labels)];
        let cancel = Cancel::new();
        let mut meter = Meter::new(limits(1 << 30));
        let mut analyzer = filter(&quotient, &constraints, &mut meter, &cancel);
        let analyses = analyzer.analyze(&all).expect("the analysis completes");
        assert_eq!(analyses.len(), 1, "one SCC");
        let fair = Analysis::first_fair_in(&analyses).expect("the SCC passes");
        assert_eq!(fair.states.len(), 18);
        let spanning = fair.spanning.as_ref().expect("a cycle group");
        assert_eq!(spanning.group.generators, [swap("b", "c")]);
        let witnesses = analyzer
            .witnesses(&all, spanning, &constraints[0])
            .expect("the witnesses");
        let taken = analyzer
            .close(witnesses.taken, &spanning.group.generators)
            .expect("the closure");
        assert_eq!(taken, constraints[0].identities);
        let walk = analyzer
            .fair_loop(fair, &Permutation::identity())
            .expect("a loop");
        let taken: BTreeSet<String> = concrete_labels(&quotient, &walk).into_iter().collect();
        assert_eq!(taken, constraints[0].identities);
        let (first, _) = walk.first().expect("a non-empty loop");
        let (last, position) = walk.last().expect("a non-empty loop");
        assert_eq!(*first, spanning.group.entry);
        assert_eq!(
            quotient.edges[last.index()][*position].target,
            spanning.group.entry
        );
        let mut meter = Meter::new(limits(1 << 30));
        let mut again = filter(&quotient, &constraints, &mut meter, &cancel);
        let repeated = again.analyze(&all).expect("the analysis completes");
        assert_eq!(groups(&repeated), groups(&analyses));
    }

    /// FR-153-AC-3's refinement: over the `Toggle` subject (two
    /// interchangeable toggles, full group), the product of `always not
    /// (every toggle fired)` under `fair weak each flip` and `fair strong
    /// each fire`. The component holding the initial state and the loop that
    /// only flips enables both `fire` identities and takes neither, so it
    /// fails strong `each`; the refinement removes exactly the states where
    /// a `fire` is enabled, and no component left, at any depth, is fair.
    /// Repeating the analysis gives equal generators at every depth
    /// (FR-153-AC-4).
    #[trace("TC-571", "FR-153-AC-3", "FR-153-AC-4")]
    #[test]
    fn strong_each_refines_away_the_states_that_enable_an_untaken_fire() {
        type Toggle = (bool, bool);
        let initial: Toy<Toggle> = [
            ("a".to_owned(), (false, false)),
            ("b".to_owned(), (false, false)),
        ]
        .into();
        let (states, mut quotient) = explore(
            initial,
            &["a", "b"],
            |state| {
                let mut steps = Vec::new();
                for (key, (on, fired)) in state {
                    let mut flipped = state.clone();
                    flipped.insert(key.clone(), (!on, *fired));
                    steps.push((format!("flip({key})"), flipped));
                    if *on && !fired {
                        let mut fired = state.clone();
                        fired.insert(key.clone(), (true, true));
                        steps.push((format!("fire({key})"), fired));
                    }
                }
                steps
            },
            |state| !state.values().all(|(_, fired)| *fired),
        );
        let all: BTreeSet<ProductStateId> = (0..states.len()).map(ProductStateId).collect();
        quotient.acceptance = vec![all.clone()];
        let fires = ids(&["fire(a)", "fire(b)"]);
        let constraints = [
            weak_each(ids(&["flip(a)", "flip(b)"])),
            QuotientConstraint {
                strength: Strength::Strong,
                granularity: FairnessGranularity::Each,
                identities: fires.clone(),
            },
        ];
        let cancel = Cancel::new();
        let mut meter = Meter::new(limits(1 << 30));
        let mut analyzer = filter(&quotient, &constraints, &mut meter, &cancel);
        let analyses = analyzer.analyze(&all).expect("the analysis completes");
        let (index, looping) = analyses
            .iter()
            .enumerate()
            .find(|(_, analysis)| analysis.states.contains(&ProductStateId(0)))
            .expect("the initial state's component");
        assert!(
            matches!(looping.verdict, Verdict::UnfairStrong),
            "the component fails strong each: {:?}",
            looping.verdict
        );
        let kept: BTreeSet<ProductStateId> = analyses
            .iter()
            .filter(|analysis| analysis.parent == Some(index))
            .flat_map(|analysis| analysis.states.iter().copied())
            .collect();
        let removed: BTreeSet<ProductStateId> = looping
            .states
            .iter()
            .copied()
            .filter(|state| !kept.contains(state))
            .collect();
        let enabling: BTreeSet<ProductStateId> = looping
            .states
            .iter()
            .copied()
            .filter(|state| !quotient.enabled[state.index()].is_disjoint(&fires))
            .collect();
        assert!(!removed.is_empty());
        assert_eq!(removed, enabling);
        assert!(Analysis::first_fair_in(&analyses).is_none());
        let mut meter = Meter::new(limits(1 << 30));
        let mut again = filter(&quotient, &constraints, &mut meter, &cancel);
        let repeated = again.analyze(&all).expect("the analysis completes");
        assert!(
            groups(&repeated).iter().any(|(_, group)| group.is_some()),
            "a cycle group is built"
        );
        assert_eq!(groups(&repeated), groups(&analyses));
        // Without the strong constraint the loop that only flips is fair:
        // each `fire` is disabled somewhere in it.
        let weak = [weak_each(ids(&["flip(a)", "flip(b)"])), weak_each(fires)];
        let mut meter = Meter::new(limits(1 << 30));
        let mut analyzer = filter(&quotient, &weak, &mut meter, &cancel);
        let analyses = analyzer.analyze(&all).expect("the analysis completes");
        let fair = Analysis::first_fair_in(&analyses).expect("a fair component");
        assert!(fair.states.contains(&ProductStateId(0)));
    }

    /// FR-153 "Limits": a meter too small for the spanning tree stops the
    /// analysis with `Meter` (the meter clause of FR-153-AC-4).
    #[trace("TC-571")]
    #[test]
    fn the_analysis_charges_the_meter() {
        let (states, mut quotient) = configs();
        let all: BTreeSet<ProductStateId> = (0..states.len()).map(ProductStateId).collect();
        quotient.acceptance = vec![all.clone()];
        let constraints = [weak_each(ids(&["upd(a)", "upd(b)", "upd(c)"]))];
        let cancel = Cancel::new();
        let mut meter = Meter::new(limits(10));
        let mut analyzer = filter(&quotient, &constraints, &mut meter, &cancel);
        assert!(matches!(analyzer.analyze(&all), Err(QuotientStop::Meter)));
    }
}

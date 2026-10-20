// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-019 FS-2's `fair` function: classify each SCC of a vertex set against
//! the acceptance sets and the fairness set, refining a component that fails
//! a strong constraint (by its enabled vertices) or a visibility constraint
//! (ADR-025 MF-2, by its load edges).

use std::collections::BTreeSet;

use quire_exact::Cancel;

use crate::fairness::constraint::{Constraint, Obligation, Strength};
use crate::fairness::error::FairnessError;
use crate::fairness::graph::{EdgeLabel, Graph, Identity, Vertex};
use crate::fairness::scc::{is_live, live_components};

/// What the filter found for one component, in the order it tests: the first
/// that applies is the witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Outcome {
    /// The component has no edge, so it holds no cycle.
    Trivial,
    /// The component holds no vertex of acceptance set `set`, the lowest
    /// such set.
    MissingAcceptance {
        /// The index of the acceptance set.
        set: usize,
    },
    /// The enablement weak constraint at `constraint`, the first such, is
    /// enabled at every vertex of the component and taken by no edge inside
    /// it.
    UnfairWeak {
        /// The constraint's index in the fairness set.
        constraint: usize,
    },
    /// Every visibility constraint in `constraints` (ADR-025 MF-2) is loaded
    /// by an edge inside the component and taken by none, so a fair cycle
    /// avoids its loads. The component is refined: the edges that load any
    /// of them are removed and `sub` is the analysis of the rest.
    UnfairVisibility {
        /// The indices of the failing visibility constraints, ascending.
        constraints: Vec<usize>,
        /// The analyses of the components of the refinement, in discovery
        /// order.
        sub: Vec<Analysis>,
    },
    /// Every strong constraint in `constraints` is enabled at some vertex of
    /// the component and taken by no edge inside it. The component is
    /// refined: the vertices where any of them is enabled are removed and
    /// `sub` is the analysis of the rest.
    UnfairStrong {
        /// The indices of the failing strong constraints, ascending.
        constraints: Vec<usize>,
        /// The analyses of the components of the refinement, in discovery
        /// order.
        sub: Vec<Analysis>,
    },
    /// The component satisfies every acceptance set and every constraint: it
    /// holds a fair accepting cycle.
    Fair,
}

/// One component and what the filter found for it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Analysis {
    vertices: Vec<Vertex>,
    outcome: Outcome,
}

impl Analysis {
    /// The component's vertices, ascending.
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// What the filter found.
    pub fn outcome(&self) -> &Outcome {
        &self.outcome
    }

    /// The first fair component under this one, depth first: this component
    /// when it is fair, otherwise the first in its refinement.
    pub fn first_fair(&self) -> Option<&Analysis> {
        match &self.outcome {
            Outcome::Fair => Some(self),
            Outcome::UnfairStrong { sub, .. } | Outcome::UnfairVisibility { sub, .. } => {
                Self::first_fair_in(sub)
            }
            Outcome::Trivial | Outcome::MissingAcceptance { .. } | Outcome::UnfairWeak { .. } => {
                None
            }
        }
    }

    /// The first fair component among `analyses`, in order, depth first
    /// through each refinement (ADR-019 FS-6).
    pub fn first_fair_in(analyses: &[Analysis]) -> Option<&Analysis> {
        analyses.iter().find_map(Analysis::first_fair)
    }
}

/// `fair` over the vertex set `within`: decompose the subgraph it induces
/// into SCCs in discovery order and classify each against `graph`'s
/// acceptance sets and `constraints`. A component that fails strong or
/// visibility constraints is refined and analysed again, so the result is
/// the whole witness tree the certificate checker reads.
///
/// `cancel` is charged before each decomposition and each vertex entered.
///
/// # Errors
///
/// [`FairnessError::UnknownVertex`] when `within` names a vertex not in the
/// graph; [`FairnessError::Cancelled`] when `cancel` is cancelled.
pub fn analyze(
    graph: &Graph,
    within: &BTreeSet<Vertex>,
    constraints: &[Constraint],
    cancel: &Cancel,
) -> Result<Vec<Analysis>, FairnessError> {
    Filter {
        graph,
        constraints,
        cancel,
    }
    .analyze(within, &BTreeSet::new())
}

/// One run of the filter. Each level reads the subgraph induced by its
/// vertices with the edges taking `removed` left out: the loads that
/// enclosing visibility refinements removed.
struct Filter<'a> {
    graph: &'a Graph,
    constraints: &'a [Constraint],
    cancel: &'a Cancel,
}

impl Filter<'_> {
    fn analyze(
        &self,
        within: &BTreeSet<Vertex>,
        removed: &BTreeSet<Identity>,
    ) -> Result<Vec<Analysis>, FairnessError> {
        live_components(self.graph, within, removed, self.cancel)?
            .into_iter()
            .map(|component| -> Result<Analysis, FairnessError> {
                let members: BTreeSet<Vertex> = component.vertices().iter().copied().collect();
                let outcome = if component.is_trivial() {
                    Outcome::Trivial
                } else {
                    self.classify(&members, removed)?
                };
                Ok(Analysis {
                    vertices: component.vertices().to_vec(),
                    outcome,
                })
            })
            .collect()
    }

    /// The outcome for a component that holds an edge.
    fn classify(
        &self,
        members: &BTreeSet<Vertex>,
        removed: &BTreeSet<Identity>,
    ) -> Result<Outcome, FairnessError> {
        let graph = self.graph;
        if let Some(set) = graph
            .acceptance_sets()
            .iter()
            .position(|accepting| accepting.is_disjoint(members))
        {
            return Ok(Outcome::MissingAcceptance { set });
        }
        let taken = |constraint: &Constraint| {
            self.inside(members, removed, |label| constraint.is_taken_by(label))
        };
        for (index, constraint) in self.constraints.iter().enumerate() {
            let unfair = constraint.obligation() == &Obligation::Enablement(Strength::Weak)
                && self.enabled_everywhere(members, constraint)
                && !taken(constraint);
            if unfair {
                return Ok(Outcome::UnfairWeak { constraint: index });
            }
        }
        let mut cut = removed.clone();
        let mut visibility = Vec::new();
        for (index, constraint) in self.constraints.iter().enumerate() {
            if let Obligation::Visibility { loads } = constraint.obligation() {
                let owed = self.inside(members, removed, |label| constraint.is_owed_by(label));
                if owed && !taken(constraint) {
                    visibility.push(index);
                    cut.extend(loads.iter().copied());
                }
            }
        }
        if !visibility.is_empty() {
            let sub = self.analyze(members, &cut)?;
            return Ok(Outcome::UnfairVisibility {
                constraints: visibility,
                sub,
            });
        }
        let failing: Vec<usize> = self
            .constraints
            .iter()
            .enumerate()
            .filter(|(_, constraint)| {
                constraint.obligation() == &Obligation::Enablement(Strength::Strong)
                    && !self.enabled_nowhere(members, constraint)
                    && !taken(constraint)
            })
            .map(|(index, _)| index)
            .collect();
        if failing.is_empty() {
            return Ok(Outcome::Fair);
        }
        let remaining: BTreeSet<Vertex> = members
            .iter()
            .copied()
            .filter(|vertex| {
                !failing.iter().any(|index| {
                    self.constraints
                        .get(*index)
                        .is_some_and(|constraint| self.enabled_at(*vertex, constraint))
                })
            })
            .collect();
        let sub = self.analyze(&remaining, removed)?;
        Ok(Outcome::UnfairStrong {
            constraints: failing,
            sub,
        })
    }

    fn enabled_at(&self, vertex: Vertex, constraint: &Constraint) -> bool {
        self.graph
            .enabled(vertex)
            .and_then(|enabled| constraint.is_enabled_in(enabled))
            .unwrap_or(false)
    }

    fn enabled_everywhere(&self, members: &BTreeSet<Vertex>, constraint: &Constraint) -> bool {
        members
            .iter()
            .all(|vertex| self.enabled_at(*vertex, constraint))
    }

    fn enabled_nowhere(&self, members: &BTreeSet<Vertex>, constraint: &Constraint) -> bool {
        !members
            .iter()
            .any(|vertex| self.enabled_at(*vertex, constraint))
    }

    /// Whether a live edge between two vertices of `members` has a label
    /// `takes` admits.
    fn inside(
        &self,
        members: &BTreeSet<Vertex>,
        removed: &BTreeSet<Identity>,
        takes: impl Fn(EdgeLabel) -> bool,
    ) -> bool {
        members.iter().any(|vertex| {
            self.graph.edges_from(*vertex).iter().any(|edge| {
                members.contains(&edge.target) && is_live(edge, removed) && takes(edge.label)
            })
        })
    }
}

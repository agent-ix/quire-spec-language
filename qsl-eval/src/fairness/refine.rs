// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-019 FS-2's `fair` function: classify each SCC of a vertex set against
//! the acceptance sets and the fairness set, refining a component that fails
//! a strong constraint.

use std::collections::BTreeSet;

use quire_exact::Cancel;

use crate::fairness::constraint::{Constraint, Strength};
use crate::fairness::error::FairnessError;
use crate::fairness::graph::{Graph, Vertex};
use crate::fairness::scc::components;

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
    /// The weak constraint at `constraint`, the first such, is enabled at
    /// every vertex of the component and taken by no edge inside it.
    UnfairWeak {
        /// The constraint's index in the fairness set.
        constraint: usize,
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
            Outcome::UnfairStrong { sub, .. } => Self::first_fair_in(sub),
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
/// acceptance sets and `constraints`. A component that fails strong
/// constraints is refined and analysed again, so the result is the whole
/// witness tree the certificate checker reads.
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
    components(graph, within, cancel)?
        .into_iter()
        .map(|component| -> Result<Analysis, FairnessError> {
            let members: BTreeSet<Vertex> = component.vertices().iter().copied().collect();
            let outcome = if component.is_trivial() {
                Outcome::Trivial
            } else {
                classify(graph, &members, constraints, cancel)?
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
    graph: &Graph,
    members: &BTreeSet<Vertex>,
    constraints: &[Constraint],
    cancel: &Cancel,
) -> Result<Outcome, FairnessError> {
    if let Some(set) = graph
        .acceptance_sets()
        .iter()
        .position(|accepting| accepting.is_disjoint(members))
    {
        return Ok(Outcome::MissingAcceptance { set });
    }
    for (index, constraint) in constraints.iter().enumerate() {
        let unfair = constraint.strength() == Strength::Weak
            && enabled_everywhere(graph, members, constraint)
            && !taken_inside(graph, members, constraint);
        if unfair {
            return Ok(Outcome::UnfairWeak { constraint: index });
        }
    }
    let failing: Vec<usize> = constraints
        .iter()
        .enumerate()
        .filter(|(_, constraint)| {
            constraint.strength() == Strength::Strong
                && !enabled_nowhere(graph, members, constraint)
                && !taken_inside(graph, members, constraint)
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
                constraints
                    .get(*index)
                    .is_some_and(|constraint| enabled_at(graph, *vertex, constraint))
            })
        })
        .collect();
    let sub = analyze(graph, &remaining, constraints, cancel)?;
    Ok(Outcome::UnfairStrong {
        constraints: failing,
        sub,
    })
}

fn enabled_at(graph: &Graph, vertex: Vertex, constraint: &Constraint) -> bool {
    graph
        .enabled(vertex)
        .is_some_and(|enabled| constraint.is_enabled_in(enabled))
}

fn enabled_everywhere(graph: &Graph, members: &BTreeSet<Vertex>, constraint: &Constraint) -> bool {
    members
        .iter()
        .all(|vertex| enabled_at(graph, *vertex, constraint))
}

fn enabled_nowhere(graph: &Graph, members: &BTreeSet<Vertex>, constraint: &Constraint) -> bool {
    !members
        .iter()
        .any(|vertex| enabled_at(graph, *vertex, constraint))
}

/// Whether an edge between two vertices of `members` takes `constraint`.
fn taken_inside(graph: &Graph, members: &BTreeSet<Vertex>, constraint: &Constraint) -> bool {
    members.iter().any(|vertex| {
        graph
            .edges_from(*vertex)
            .iter()
            .any(|edge| members.contains(&edge.target) && constraint.is_taken_by(edge.label))
    })
}

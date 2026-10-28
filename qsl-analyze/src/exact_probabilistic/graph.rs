// SPDX-License-Identifier: AGPL-3.0-or-later
//! Graph decomposition used by unbounded objectives and scheduler fairness.

use std::collections::VecDeque;

use super::{ProbProduct, ProbStateId, ProductAction, ProductKind};
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

/// The maximal end components of an MDP product, or bottom strongly connected
/// components of a DTMC product.
pub fn recurrent_components(product: &ProbProduct) -> Vec<Vec<ProbStateId>> {
    if product.kind == ProductKind::Dtmc {
        return bottom_components(product);
    }
    maximal_end_components(product)
}

/// Bottom strongly connected components of a finite DTMC.
pub fn bottom_components(product: &ProbProduct) -> Vec<Vec<ProbStateId>> {
    let all = vec![true; product.states.len()];
    let components = strongly_connected(product, &all, None);
    components
        .into_iter()
        .filter(|component| {
            let inside = component
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            component.iter().all(|state| {
                product
                    .outgoing(ProbStateId(*state))
                    .iter()
                    .filter(|edge| !edge.probability.is_zero())
                    .all(|edge| inside.contains(&edge.to.index()))
            })
        })
        .map(as_state_ids)
        .collect()
}

/// Maximal end components of a finite MDP. An action is internal only when
/// every positive-probability successor stays in the component.
pub fn maximal_end_components(product: &ProbProduct) -> Vec<Vec<ProbStateId>> {
    maximal_end_components_avoiding(product, &vec![false; product.states.len()])
}

/// One maximal fair end component of an EN-5 product (FR-200 FS-3, FS-4).
#[derive(Clone, Debug)]
pub struct FairEndComponent {
    /// Its states, in canonical product order.
    pub states: Vec<ProbStateId>,
    /// Every action whose positive-probability successors stay in this
    /// component, grouped by source state and canonical action order.
    pub actions: Vec<(ProbStateId, ProductAction)>,
}

/// Compute the maximal fair end components of a probabilistic product by
/// refining its maximal end components under the claim's fairness set.
pub fn maximal_fair_end_components(product: &ProbProduct) -> Vec<FairEndComponent> {
    maximal_fair_end_components_avoiding(product, &vec![false; product.states.len()])
}

/// Compute fair end components after excluding objective-absorbing or
/// otherwise ineligible states, such as the target states of reachability.
pub fn maximal_fair_end_components_avoiding(
    product: &ProbProduct,
    excluded: &[bool],
) -> Vec<FairEndComponent> {
    if product.kind != ProductKind::Mdp {
        return Vec::new();
    }
    if excluded.len() != product.states.len() {
        return Vec::new();
    }
    let classes = fairness_classes(product);
    let mut pending = maximal_end_components_avoiding(product, excluded);
    let mut fair = Vec::new();
    while let Some(component) = pending.pop() {
        let mut included = vec![false; product.states.len()];
        for state in &component {
            included[state.index()] = true;
        }
        let mut changed = false;
        for class in &classes {
            let enabled = component
                .iter()
                .copied()
                .filter(|state| has_action(product, *state, class))
                .collect::<Vec<_>>();
            let taken = component.iter().copied().any(|state| {
                internal_action_groups(product, state, &included)
                    .iter()
                    .any(|(action, _, _)| class.matches(action))
            });
            if taken {
                continue;
            }
            match class.kind {
                FairnessKind::Weak if enabled.len() == component.len() => {
                    changed = true;
                    break;
                }
                FairnessKind::Strong if !enabled.is_empty() => {
                    for state in enabled {
                        included[state.index()] = false;
                    }
                    let excluded = included.iter().map(|member| !member).collect::<Vec<_>>();
                    pending.extend(maximal_end_components_avoiding(product, &excluded));
                    changed = true;
                    break;
                }
                FairnessKind::Weak | FairnessKind::Strong => {}
            }
        }
        if changed {
            continue;
        }
        let actions = component
            .iter()
            .copied()
            .flat_map(|state| {
                internal_action_groups(product, state, &included)
                    .into_iter()
                    .map(move |(action, _, _)| (state, action))
            })
            .collect();
        fair.push(FairEndComponent {
            states: component,
            actions,
        });
    }
    fair.sort_by(|left, right| left.states[0].cmp(&right.states[0]));
    fair
}

#[derive(Clone)]
struct FairActionClass {
    operation: String,
    identity: Option<qsl_eval::simulation::model::ScheduledIdentity>,
    kind: FairnessKind,
}

impl FairActionClass {
    fn matches(&self, action: &ProductAction) -> bool {
        let ProductAction::Identity(identity) = action else {
            return false;
        };
        identity.operation == self.operation
            && self
                .identity
                .as_ref()
                .is_none_or(|member| member == identity)
    }
}

fn fairness_classes(product: &ProbProduct) -> Vec<FairActionClass> {
    let mut classes = Vec::new();
    for fairness in &product.fairness {
        match fairness.constraint.granularity() {
            FairnessGranularity::Whole => classes.push(FairActionClass {
                operation: fairness.operation.clone(),
                identity: None,
                kind: fairness.constraint.kind(),
            }),
            FairnessGranularity::Each => {
                let mut identities = Vec::new();
                for edge in &product.edges {
                    let Some(identity) = edge.action.as_ref() else {
                        continue;
                    };
                    if identity.operation == fairness.operation
                        && !identities.iter().any(|known| known == identity)
                    {
                        identities.push(identity.clone());
                    }
                }
                classes.extend(identities.into_iter().map(|identity| FairActionClass {
                    operation: fairness.operation.clone(),
                    identity: Some(identity),
                    kind: fairness.constraint.kind(),
                }));
            }
        }
    }
    classes
}

fn has_action(product: &ProbProduct, state: ProbStateId, class: &FairActionClass) -> bool {
    action_groups(product, state)
        .iter()
        .any(|(action, _, _)| class.matches(action))
}

fn action_groups(product: &ProbProduct, state: ProbStateId) -> Vec<(ProductAction, usize, usize)> {
    let edges = product.outgoing(state);
    let base = product.edges.partition_point(|edge| edge.from < state);
    let mut groups = Vec::new();
    let mut cursor = 0;
    while cursor < edges.len() {
        let first = &edges[cursor];
        let mut end = cursor + 1;
        while end < edges.len() && same_action(&first.choice, &edges[end].choice) {
            end += 1;
        }
        groups.push((first.choice.clone(), base + cursor, base + end));
        cursor = end;
    }
    groups
}

fn internal_action_groups(
    product: &ProbProduct,
    state: ProbStateId,
    component: &[bool],
) -> Vec<(ProductAction, usize, usize)> {
    action_groups(product, state)
        .into_iter()
        .filter(|(_, start, end)| {
            let positive = product.edges[*start..*end]
                .iter()
                .filter(|edge| !edge.probability.is_zero())
                .collect::<Vec<_>>();
            !positive.is_empty() && positive.iter().all(|edge| component[edge.to.index()])
        })
        .collect()
}

fn same_action(left: &ProductAction, right: &ProductAction) -> bool {
    match (left, right) {
        (ProductAction::Identity(left), ProductAction::Identity(right)) => left == right,
        (ProductAction::PostState(left), ProductAction::PostState(right)) => left == right,
        (ProductAction::TerminalStutter, ProductAction::TerminalStutter) => true,
        _ => false,
    }
}

/// Maximal end components in the induced MDP after excluding selected states.
pub(crate) fn maximal_end_components_avoiding(
    product: &ProbProduct,
    excluded: &[bool],
) -> Vec<Vec<ProbStateId>> {
    if excluded.len() != product.states.len() {
        return Vec::new();
    }
    let included = excluded.iter().map(|value| !value).collect::<Vec<_>>();
    let mut pending = VecDeque::from([included.clone()]);
    let mut result = Vec::new();
    while let Some(mut candidate) = pending.pop_front() {
        loop {
            let removed = (0..candidate.len())
                .filter(|state| candidate[*state])
                .filter(|state| !has_internal_action(product, *state, &candidate))
                .collect::<Vec<_>>();
            if removed.is_empty() {
                break;
            }
            for state in removed {
                candidate[state] = false;
            }
        }
        if !candidate.iter().any(|included| *included) {
            continue;
        }
        let components = strongly_connected(product, &candidate, Some(&candidate));
        if components.len() == 1 {
            result.push(as_state_ids(
                components.into_iter().next().expect("one SCC"),
            ));
        } else {
            for component in components {
                let mut next = vec![false; candidate.len()];
                for state in component {
                    next[state] = true;
                }
                pending.push_back(next);
            }
        }
    }
    result.sort_by(|left, right| left[0].cmp(&right[0]));
    result
}

fn has_internal_action(product: &ProbProduct, state: usize, candidate: &[bool]) -> bool {
    let edges = product.outgoing(ProbStateId(state));
    if edges.is_empty() {
        return false;
    }
    if product.kind == ProductKind::Dtmc {
        return edges.iter().all(|edge| candidate[edge.to.index()]);
    }
    let mut cursor = 0;
    while cursor < edges.len() {
        let first = &edges[cursor];
        let mut end = cursor + 1;
        while end < edges.len() && same_action(&first.choice, &edges[end].choice) {
            end += 1;
        }
        if edges[cursor..end]
            .iter()
            .filter(|edge| !edge.probability.is_zero())
            .all(|edge| candidate[edge.to.index()])
            && edges[cursor..end]
                .iter()
                .any(|edge| !edge.probability.is_zero())
        {
            return true;
        }
        cursor = end;
    }
    false
}

/// Iterative Kosaraju decomposition. `allowed_actions`, when supplied, is
/// the state-set that restricts every MDP action to successors inside it.
fn strongly_connected(
    product: &ProbProduct,
    included: &[bool],
    allowed_actions: Option<&[bool]>,
) -> Vec<Vec<usize>> {
    let count = product.states.len();
    let mut adjacency = vec![Vec::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for state in 0..count {
        if !included[state] {
            continue;
        }
        let edges = product.outgoing(ProbStateId(state));
        if product.kind == ProductKind::Dtmc {
            for edge in edges {
                if !edge.probability.is_zero() && included[edge.to.index()] {
                    adjacency[state].push(edge.to.index());
                    reverse[edge.to.index()].push(state);
                }
            }
            continue;
        }
        let mut cursor = 0;
        while cursor < edges.len() {
            let first = &edges[cursor];
            let mut end = cursor + 1;
            while end < edges.len() && same_action(&first.choice, &edges[end].choice) {
                end += 1;
            }
            let group = &edges[cursor..end];
            if allowed_actions.is_none_or(|allowed| {
                group
                    .iter()
                    .filter(|edge| !edge.probability.is_zero())
                    .all(|edge| allowed[edge.to.index()])
                    && group.iter().any(|edge| !edge.probability.is_zero())
            }) {
                for edge in group {
                    if !edge.probability.is_zero() && included[edge.to.index()] {
                        adjacency[state].push(edge.to.index());
                        reverse[edge.to.index()].push(state);
                    }
                }
            }
            cursor = end;
        }
    }
    for edges in &mut adjacency {
        edges.sort_unstable();
        edges.dedup();
    }
    for edges in &mut reverse {
        edges.sort_unstable();
        edges.dedup();
    }

    let mut visited = vec![false; count];
    let mut order = Vec::new();
    for root in 0..count {
        if !included[root] || visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack = vec![(root, 0_usize)];
        while let Some((state, next)) = stack.last_mut() {
            if *next < adjacency[*state].len() {
                let target = adjacency[*state][*next];
                *next += 1;
                if !visited[target] {
                    visited[target] = true;
                    stack.push((target, 0));
                }
            } else {
                order.push(*state);
                stack.pop();
            }
        }
    }

    visited.fill(false);
    let mut components = Vec::new();
    for root in order.into_iter().rev() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut component = Vec::new();
        let mut stack = vec![root];
        while let Some(state) = stack.pop() {
            component.push(state);
            for target in &reverse[state] {
                if !visited[*target] {
                    visited[*target] = true;
                    stack.push(*target);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    components.sort_by_key(|component| component[0]);
    components
}

fn as_state_ids(component: Vec<usize>) -> Vec<ProbStateId> {
    component.into_iter().map(ProbStateId).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact_probabilistic::{ProbEdge, ProbKey};
    use quire_exact::{Integer, Rational};

    fn key(index: usize) -> ProbKey {
        ProbKey {
            binding: None,
            model: vec![u8::try_from(index).expect("small index")],
            monitor: None,
            accumulator: None,
            pending: None,
        }
    }

    fn edge(from: usize, action: usize, to: usize) -> ProbEdge {
        ProbEdge {
            from: ProbStateId(from),
            choice: ProductAction::PostState(Box::new(key(action))),
            action: None,
            draw: None,
            to: ProbStateId(to),
            probability: Rational::from_integer(Integer::one()),
            rewards: Vec::new(),
        }
    }

    fn zero_edge(from: usize, action: usize, to: usize) -> ProbEdge {
        ProbEdge {
            probability: Rational::from_integer(Integer::zero()),
            ..edge(from, action, to)
        }
    }

    fn product(kind: ProductKind, edges: Vec<ProbEdge>, count: usize) -> ProbProduct {
        ProbProduct {
            states: (0..count).map(key).collect(),
            kind,
            edges,
            initial: vec![],
            decided: vec![],
            labels: vec![vec![]; count],
            fairness: vec![],
        }
    }

    #[test]
    fn bottom_components_exclude_states_with_edges_leaving_the_scc() {
        let graph = product(
            ProductKind::Dtmc,
            vec![edge(0, 0, 0), edge(0, 0, 1), edge(1, 1, 1)],
            2,
        );
        assert_eq!(bottom_components(&graph), vec![vec![ProbStateId(1)]]);
    }

    #[test]
    fn maximal_end_components_keep_a_staying_action_and_an_exit_action() {
        let graph = product(
            ProductKind::Mdp,
            vec![edge(0, 0, 0), edge(0, 1, 1), edge(1, 2, 1)],
            2,
        );
        assert_eq!(
            maximal_end_components(&graph),
            vec![vec![ProbStateId(0)], vec![ProbStateId(1)]]
        );
    }

    #[test]
    fn zero_probability_successors_do_not_break_an_end_component() {
        let graph = product(
            ProductKind::Mdp,
            vec![edge(0, 0, 0), zero_edge(0, 0, 1), edge(1, 1, 1)],
            2,
        );
        assert_eq!(
            maximal_end_components(&graph),
            vec![vec![ProbStateId(0)], vec![ProbStateId(1)]]
        );
    }
}

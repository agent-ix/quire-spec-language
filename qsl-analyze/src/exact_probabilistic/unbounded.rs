// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact reachability values for EN-5's unbounded objectives (FR-198 UR-1,
//! UR-4). The worklist and linear solver here are engine-owned; the replay
//! checker will implement its own recomputation.

use std::collections::VecDeque;

use qsl_semantics::check::{ProbForm, ReachTarget};
use quire_exact::{Integer, Rational};

use super::finite::Extremum;
use super::{ProbProduct, ProbStateId, ProductAction, ProductKind};

/// Exact values and the selected memoryless action at each MDP state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReachabilitySolution {
    values: Vec<Rational>,
    /// Index of the first product edge of the chosen action, or `None` for
    /// a DTMC state or a state with no enabled action.
    policy_edges: Vec<Option<usize>>,
}

impl ReachabilitySolution {
    /// The reachability probability at `state`.
    pub fn value(&self, state: ProbStateId) -> Option<&Rational> {
        self.values.get(state.index())
    }

    /// The selected action's first edge at `state`.
    pub fn policy_edge(&self, state: ProbStateId) -> Option<usize> {
        self.policy_edges.get(state.index()).copied().flatten()
    }
}

/// Exact probability of reaching a target state under a workload or an
/// optimizing memoryless scheduler.
pub fn reachability(
    product: &ProbProduct,
    targets: &[bool],
    extremum: Extremum,
    max_policy_iterations: u64,
) -> Result<ReachabilitySolution, ReachabilityFault> {
    reachability_with_zeroes(
        product,
        targets,
        &vec![false; product.states.len()],
        extremum,
        max_policy_iterations,
    )
}

/// Exact reachability with states that make the objective false before its
/// target is reached (the `A until B` form, FR-198).
pub fn reachability_with_zeroes(
    product: &ProbProduct,
    targets: &[bool],
    zeroes: &[bool],
    extremum: Extremum,
    max_policy_iterations: u64,
) -> Result<ReachabilitySolution, ReachabilityFault> {
    if targets.len() != product.states.len() || zeroes.len() != product.states.len() {
        return Err(ReachabilityFault::TargetLength {
            expected: product.states.len(),
            actual: if targets.len() != product.states.len() {
                targets.len()
            } else {
                zeroes.len()
            },
        });
    }
    let mut policy_edges = vec![None; product.states.len()];
    if product.kind == ProductKind::Mdp {
        for state in product.ids() {
            policy_edges[state.index()] = action_groups(product, state)
                .first()
                .map(|(start, _)| *start);
        }
        if extremum == Extremum::Minimum {
            let excluded = targets
                .iter()
                .zip(zeroes)
                .map(|(target, zero)| *target || *zero)
                .collect::<Vec<_>>();
            for component in super::graph::maximal_end_components_avoiding(product, &excluded) {
                let mut member = vec![false; product.states.len()];
                for state in component {
                    member[state.index()] = true;
                }
                for state in product.ids().filter(|state| member[state.index()]) {
                    if let Some((start, _)) =
                        action_groups(product, state)
                            .into_iter()
                            .find(|(start, end)| {
                                product.edges[*start..*end]
                                    .iter()
                                    .all(|edge| member[edge.to.index()])
                            })
                    {
                        policy_edges[state.index()] = Some(start);
                    }
                }
            }
        }
    }
    if product.kind == ProductKind::Dtmc {
        let values = evaluate_chain(product, &policy_edges, targets, zeroes)?;
        return Ok(ReachabilitySolution {
            values,
            policy_edges,
        });
    }

    for iteration in 0..max_policy_iterations {
        let values = evaluate_chain(product, &policy_edges, targets, zeroes)?;
        let mut changed = false;
        for state in product.ids() {
            let index = state.index();
            if targets[index] || zeroes[index] {
                continue;
            }
            let groups = action_groups(product, state);
            if groups.is_empty() {
                continue;
            }
            let mut best: Option<(usize, Rational)> = None;
            for (start, end) in groups {
                let expected = expectation(product, &values, start, end)?;
                best = Some(match best {
                    None => (start, expected),
                    Some((best_start, best_value)) => {
                        let improves = match extremum {
                            Extremum::Minimum => expected < best_value,
                            Extremum::Maximum => expected > best_value,
                        };
                        if improves {
                            (start, expected)
                        } else {
                            (best_start, best_value)
                        }
                    }
                });
            }
            let (best_start, best_value) = best.expect("nonempty action groups");
            let current_start = policy_edges[index].ok_or(ReachabilityFault::GraphShape)?;
            let current_value = expectation(
                product,
                &values,
                current_start,
                action_end(product, state, current_start)?,
            )?;
            let improves = match extremum {
                Extremum::Minimum => best_value < current_value,
                Extremum::Maximum => best_value > current_value,
            };
            if improves {
                policy_edges[index] = Some(best_start);
                changed = true;
            }
        }
        if !changed {
            return Ok(ReachabilitySolution {
                values,
                policy_edges,
            });
        }
        if iteration + 1 == max_policy_iterations {
            return Err(ReachabilityFault::PolicyIterationLimit {
                limit: max_policy_iterations,
            });
        }
    }
    Err(ReachabilityFault::PolicyIterationLimit {
        limit: max_policy_iterations,
    })
}

/// Evaluate an FR-198 `Reach` form using the product's state-predicate
/// labels. `Always P` exchanges the extremum while solving `eventually !P`
/// and complements its values.
pub fn reach_form(
    product: &ProbProduct,
    form: &ProbForm,
    extremum: Extremum,
    max_policy_iterations: u64,
) -> Result<ReachabilitySolution, ReachabilityFault> {
    let ProbForm::Reach { target, .. } = form else {
        return Err(ReachabilityFault::ObjectiveShape);
    };
    match target {
        ReachTarget::Eventually(_) => {
            require_label_count(product, 1)?;
            reachability(
                product,
                &product
                    .labels
                    .iter()
                    .map(|labels| labels[0])
                    .collect::<Vec<_>>(),
                extremum,
                max_policy_iterations,
            )
        }
        ReachTarget::Until(_, _) => {
            require_label_count(product, 2)?;
            let targets = product
                .labels
                .iter()
                .map(|labels| labels[1])
                .collect::<Vec<_>>();
            let zeroes = product
                .labels
                .iter()
                .map(|labels| !labels[0] && !labels[1])
                .collect::<Vec<_>>();
            reachability_with_zeroes(product, &targets, &zeroes, extremum, max_policy_iterations)
        }
        ReachTarget::Always(_) => {
            require_label_count(product, 1)?;
            let violations = product
                .labels
                .iter()
                .map(|labels| !labels[0])
                .collect::<Vec<_>>();
            let swapped = match extremum {
                Extremum::Minimum => Extremum::Maximum,
                Extremum::Maximum => Extremum::Minimum,
            };
            let mut solution = reachability(product, &violations, swapped, max_policy_iterations)?;
            for value in &mut solution.values {
                *value = one().sub(value);
            }
            Ok(solution)
        }
    }
}

fn require_label_count(product: &ProbProduct, expected: usize) -> Result<(), ReachabilityFault> {
    if product.labels.len() != product.states.len()
        || product.labels.iter().any(|labels| labels.len() != expected)
    {
        Err(ReachabilityFault::ObjectiveShape)
    } else {
        Ok(())
    }
}

/// Evaluate a fixed policy as a DTMC and solve its transient equations.
fn evaluate_chain(
    product: &ProbProduct,
    policy_edges: &[Option<usize>],
    targets: &[bool],
    zeroes: &[bool],
) -> Result<Vec<Rational>, ReachabilityFault> {
    let count = product.states.len();
    if policy_edges.len() != count {
        return Err(ReachabilityFault::GraphShape);
    }
    let mut transitions = vec![Vec::<(usize, Rational)>::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for state in product.ids() {
        let index = state.index();
        let edges = selected_edges(product, policy_edges, state)?;
        if targets[index] || zeroes[index] {
            continue;
        }
        let mass = edges.iter().try_fold(zero(), |sum, edge| {
            if edge.probability < zero() {
                return Err(ReachabilityFault::NegativeProbability);
            }
            Ok(sum.add(&edge.probability))
        })?;
        if !edges.is_empty() && mass != one() {
            return Err(ReachabilityFault::ProbabilityMass);
        }
        for edge in edges {
            transitions[index].push((edge.to.index(), edge.probability.clone()));
            reverse[edge.to.index()].push(index);
        }
    }

    let mut reaches = targets.to_vec();
    let mut queue = targets
        .iter()
        .enumerate()
        .filter_map(|(index, target)| target.then_some(index))
        .collect::<VecDeque<_>>();
    while let Some(state) = queue.pop_front() {
        for predecessor in &reverse[state] {
            if !zeroes[*predecessor] && !reaches[*predecessor] {
                reaches[*predecessor] = true;
                queue.push_back(*predecessor);
            }
        }
    }
    let unknown = (0..count)
        .filter(|state| reaches[*state] && !targets[*state] && !zeroes[*state])
        .collect::<Vec<_>>();
    let mut values = targets
        .iter()
        .map(|target| if *target { one() } else { zero() })
        .collect::<Vec<_>>();
    if unknown.is_empty() {
        return Ok(values);
    }

    let mut positions = vec![None; count];
    for (row, state) in unknown.iter().copied().enumerate() {
        positions[state] = Some(row);
    }
    let size = unknown.len();
    let mut matrix = vec![vec![zero(); size + 1]; size];
    for (row, state) in unknown.iter().copied().enumerate() {
        matrix[row][row] = one();
        for (target, probability) in &transitions[state] {
            if targets[*target] {
                matrix[row][size] = matrix[row][size].add(probability);
            } else if let Some(column) = positions[*target] {
                matrix[row][column] = matrix[row][column].sub(probability);
            }
        }
    }
    gaussian_elimination(&mut matrix)?;
    for (row, state) in unknown.into_iter().enumerate() {
        values[state] = matrix[row][size].clone();
    }
    Ok(values)
}

fn gaussian_elimination(matrix: &mut [Vec<Rational>]) -> Result<(), ReachabilityFault> {
    let size = matrix.len();
    for column in 0..size {
        let pivot = (column..size)
            .find(|row| !matrix[*row][column].is_zero())
            .ok_or(ReachabilityFault::SingularSystem)?;
        matrix.swap(column, pivot);
        let divisor = matrix[column][column].clone();
        for value in &mut matrix[column][column..=size] {
            *value = value
                .div(&divisor)
                .ok_or(ReachabilityFault::SingularSystem)?;
        }
        let pivot_row = matrix[column].clone();
        for (row_index, row_values) in matrix.iter_mut().enumerate() {
            if row_index == column || row_values[column].is_zero() {
                continue;
            }
            let factor = row_values[column].clone();
            for (value, pivot_value) in row_values[column..=size]
                .iter_mut()
                .zip(&pivot_row[column..=size])
            {
                *value = value.sub(&factor.mul(pivot_value));
            }
        }
    }
    Ok(())
}

fn selected_edges<'a>(
    product: &'a ProbProduct,
    policy_edges: &[Option<usize>],
    state: ProbStateId,
) -> Result<&'a [super::ProbEdge], ReachabilityFault> {
    let outgoing = product.outgoing(state);
    if product.kind == ProductKind::Dtmc {
        return Ok(outgoing);
    }
    let Some(start) = policy_edges[state.index()] else {
        return Ok(&outgoing[..0]);
    };
    let end = action_end(product, state, start)?;
    let base = product.edges.partition_point(|edge| edge.from < state);
    let local_start = start
        .checked_sub(base)
        .ok_or(ReachabilityFault::GraphShape)?;
    let local_end = end.checked_sub(base).ok_or(ReachabilityFault::GraphShape)?;
    outgoing
        .get(local_start..local_end)
        .ok_or(ReachabilityFault::GraphShape)
}

fn action_groups(product: &ProbProduct, state: ProbStateId) -> Vec<(usize, usize)> {
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
        groups.push((base + cursor, base + end));
        cursor = end;
    }
    groups
}

fn action_end(
    product: &ProbProduct,
    state: ProbStateId,
    start: usize,
) -> Result<usize, ReachabilityFault> {
    action_groups(product, state)
        .into_iter()
        .find_map(|(candidate, end)| (candidate == start).then_some(end))
        .ok_or(ReachabilityFault::GraphShape)
}

fn expectation(
    product: &ProbProduct,
    values: &[Rational],
    start: usize,
    end: usize,
) -> Result<Rational, ReachabilityFault> {
    product
        .edges
        .get(start..end)
        .ok_or(ReachabilityFault::GraphShape)?
        .iter()
        .try_fold(zero(), |sum, edge| {
            Ok(sum.add(&edge.probability.mul(&values[edge.to.index()])))
        })
}

fn same_action(left: &ProductAction, right: &ProductAction) -> bool {
    match (left, right) {
        (ProductAction::Identity(left), ProductAction::Identity(right)) => left == right,
        (ProductAction::PostState(left), ProductAction::PostState(right)) => left == right,
        (ProductAction::TerminalStutter, ProductAction::TerminalStutter) => true,
        _ => false,
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

/// A malformed product, singular solve, or exhausted policy-improvement
/// budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReachabilityFault {
    /// The target vector does not cover every product state.
    #[error("target vector has length {actual}, expected {expected}")]
    TargetLength {
        /// Number of product states.
        expected: usize,
        /// Number of target entries supplied.
        actual: usize,
    },
    /// Product endpoints or a policy edge are inconsistent.
    #[error("probability product graph is inconsistent")]
    GraphShape,
    /// The product's state labels do not match the claim's reachability form.
    #[error("reachability form and product labels have different shapes")]
    ObjectiveShape,
    /// An edge carries negative probability.
    #[error("probability product has a negative edge")]
    NegativeProbability,
    /// An induced state or action has total probability other than one.
    #[error("induced probability mass does not sum to one")]
    ProbabilityMass,
    /// Exact linear equations have no unique transient solution.
    #[error("reachability linear system is singular")]
    SingularSystem,
    /// Policy improvement exceeded its request limit.
    #[error("policy iteration reached its limit of {limit}")]
    PolicyIterationLimit {
        /// The configured maximum number of policy iterations.
        limit: u64,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact_probabilistic::ProbKey;

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("ratio")
    }

    fn key(index: usize) -> ProbKey {
        ProbKey {
            binding: None,
            model: vec![u8::try_from(index).expect("small state")],
            monitor: None,
            accumulator: None,
            pending: None,
        }
    }

    fn edge(
        from: usize,
        action: usize,
        to: usize,
        probability: Rational,
    ) -> super::super::ProbEdge {
        super::super::ProbEdge {
            from: ProbStateId(from),
            choice: ProductAction::PostState(Box::new(key(action))),
            action: None,
            draw: None,
            to: ProbStateId(to),
            probability,
            rewards: Vec::new(),
        }
    }

    fn product(kind: ProductKind, edges: Vec<super::super::ProbEdge>) -> ProbProduct {
        ProbProduct {
            states: vec![key(0), key(1), key(2)],
            kind,
            edges,
            initial: vec![],
            decided: vec![],
            labels: vec![vec![], vec![], vec![]],
        }
    }

    #[test]
    fn dtmc_reachability_solves_a_geometric_loop_exactly() {
        let graph = product(
            ProductKind::Dtmc,
            vec![
                edge(0, 0, 0, ratio(1, 3)),
                edge(0, 0, 1, ratio(1, 3)),
                edge(0, 0, 2, ratio(1, 3)),
            ],
        );
        let result = reachability(&graph, &[false, true, false], Extremum::Minimum, 1)
            .expect("a transient DTMC system has a unique solution");
        assert_eq!(result.value(ProbStateId(0)), Some(&ratio(1, 2)));
    }

    #[test]
    fn until_zero_states_stop_before_their_later_target_paths() {
        let graph = product(
            ProductKind::Dtmc,
            vec![
                edge(0, 0, 2, Rational::from_integer(Integer::one())),
                edge(2, 2, 1, Rational::from_integer(Integer::one())),
            ],
        );
        let result = reachability_with_zeroes(
            &graph,
            &[false, true, false],
            &[false, false, true],
            Extremum::Minimum,
            0,
        )
        .expect("zero state makes the until objective false");
        assert_eq!(
            result.value(ProbStateId(0)),
            Some(&Rational::from_integer(Integer::zero()))
        );
    }

    #[test]
    fn mdp_policy_iteration_selects_the_action_with_the_best_reachability() {
        let graph = product(
            ProductKind::Mdp,
            vec![
                edge(0, 0, 0, Rational::from_integer(Integer::one())),
                edge(0, 1, 1, ratio(1, 2)),
                edge(0, 1, 2, ratio(1, 2)),
            ],
        );
        let result = reachability(&graph, &[false, true, false], Extremum::Maximum, 10)
            .expect("policy iteration settles");
        assert_eq!(result.value(ProbStateId(0)), Some(&ratio(1, 2)));
        assert_eq!(result.policy_edge(ProbStateId(0)), Some(1));
    }

    #[test]
    fn minimum_policy_starts_inside_target_free_end_component() {
        let graph = product(
            ProductKind::Mdp,
            vec![
                edge(0, 0, 0, Rational::from_integer(Integer::one())),
                edge(0, 1, 1, ratio(1, 2)),
                edge(0, 1, 2, ratio(1, 2)),
            ],
        );
        let result = reachability(&graph, &[false, true, false], Extremum::Minimum, 10)
            .expect("the target-free end component yields minimum zero");
        assert_eq!(result.value(ProbStateId(0)), Some(&zero()));
        assert_eq!(result.policy_edge(ProbStateId(0)), Some(0));
    }
}

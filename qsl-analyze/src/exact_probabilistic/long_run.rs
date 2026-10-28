// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-199's workload long-run calculation over DTMC bottom components.

use std::collections::BTreeMap;

use quire_exact::{Identifier, Integer, Rational, Value};

use super::unbounded::{reachability, ReachabilityFault};
use super::{Extremum, ProbProduct, ProbStateId, ProductKind};

/// The exact long-run ratio for one bottom component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongRunComponent {
    /// Product states in canonical discovery order.
    pub states: Vec<ProbStateId>,
    /// The component's exact stationary weighted fraction.
    pub value: Rational,
}

/// Exact long-run values for a workload's DTMC.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongRunSolution {
    /// Each reachable bottom component and its exact ratio.
    pub components: Vec<LongRunComponent>,
    /// The expected component ratio from each product state.
    values: Vec<Rational>,
}

impl LongRunSolution {
    /// The long-run value at `state`.
    pub fn value(&self, state: ProbStateId) -> Option<&Rational> {
        self.values.get(state.index())
    }
}

/// Decide a DTMC long-run fraction by stationary distributions of bottom
/// strongly connected components (FR-199). `weight` is absent for an
/// unweighted fraction.
pub fn long_run_fraction(
    product: &ProbProduct,
    predicate: &[bool],
    weight: Option<&Identifier>,
) -> Result<LongRunSolution, LongRunFault> {
    if product.kind != ProductKind::Dtmc {
        return Err(LongRunFault::RequiresDtmc);
    }
    if predicate.len() != product.states.len() {
        return Err(LongRunFault::Shape);
    }
    let components = super::graph::bottom_components(product);
    let mut solved = Vec::with_capacity(components.len());
    for states in components {
        let stationary = stationary_distribution(product, &states)?;
        let mut numerator = zero();
        let mut denominator = zero();
        for (state, mass) in states.iter().zip(stationary) {
            let leaving_weight = outgoing_weight(product, *state, weight)?;
            denominator = denominator.add(&mass.mul(&leaving_weight));
            if predicate[state.index()] {
                numerator = numerator.add(&mass.mul(&leaving_weight));
            }
        }
        if denominator.is_zero() {
            return Err(LongRunFault::ZeroWeightComponent);
        }
        let value = numerator
            .div(&denominator)
            .ok_or(LongRunFault::ZeroWeightComponent)?;
        solved.push(LongRunComponent { states, value });
    }
    let mut values = vec![zero(); product.states.len()];
    for component in &solved {
        let mut target = vec![false; product.states.len()];
        for state in &component.states {
            target[state.index()] = true;
        }
        let reach = reachability(product, &target, Extremum::Minimum, 0)?;
        for state in product.ids() {
            let probability = reach.value(state).ok_or(LongRunFault::Shape)?;
            values[state.index()] = values[state.index()].add(&probability.mul(&component.value));
        }
    }
    Ok(LongRunSolution {
        components: solved,
        values,
    })
}

fn stationary_distribution(
    product: &ProbProduct,
    states: &[ProbStateId],
) -> Result<Vec<Rational>, LongRunFault> {
    if states.is_empty() {
        return Err(LongRunFault::Shape);
    }
    let mut positions = vec![None; product.states.len()];
    for (position, state) in states.iter().copied().enumerate() {
        positions[state.index()] = Some(position);
    }
    let size = states.len();
    let mut matrix = vec![BTreeMap::new(); size];
    for row in 0..size.saturating_sub(1) {
        add(&mut matrix[row], row, one());
    }
    for (source_position, source) in states.iter().copied().enumerate() {
        let edges = product.outgoing(source);
        let mass = edges
            .iter()
            .fold(zero(), |sum, edge| sum.add(&edge.probability));
        if mass != one() {
            return Err(LongRunFault::Shape);
        }
        for edge in edges.iter().filter(|edge| !edge.probability.is_zero()) {
            let target_position = positions[edge.to.index()].ok_or(LongRunFault::Shape)?;
            if target_position < size.saturating_sub(1) {
                add(
                    &mut matrix[target_position],
                    source_position,
                    zero().sub(&edge.probability),
                );
            }
        }
    }
    for position in 0..size {
        matrix[size - 1].insert(position, one());
    }
    matrix[size - 1].insert(size, one());
    solve(&mut matrix)?;
    (0..size)
        .map(|row| matrix[row].get(&size).cloned().ok_or(LongRunFault::Shape))
        .collect()
}

fn outgoing_weight(
    product: &ProbProduct,
    state: ProbStateId,
    weight: Option<&Identifier>,
) -> Result<Rational, LongRunFault> {
    let edges = product.outgoing(state);
    let mass = edges
        .iter()
        .fold(zero(), |sum, edge| sum.add(&edge.probability));
    if mass != one() {
        return Err(LongRunFault::Shape);
    }
    edges.iter().try_fold(zero(), |sum, edge| {
        let amount = match weight {
            None => one(),
            Some(name) => edge_weight(edge, name)?,
        };
        Ok(sum.add(&edge.probability.mul(&amount)))
    })
}

fn edge_weight(edge: &super::ProbEdge, name: &Identifier) -> Result<Rational, LongRunFault> {
    let (_, value) = edge
        .rewards
        .iter()
        .find(|(reward, _)| reward == name)
        .ok_or(LongRunFault::MissingWeight)?;
    match value {
        Value::Integer(value) => Ok(Rational::from_integer(value.clone())),
        Value::Rational(value) => Ok(value.clone()),
        Value::Quantity(value) => Ok(value.magnitude().clone()),
        _ => Err(LongRunFault::InvalidWeight),
    }
}

fn solve(matrix: &mut [BTreeMap<usize, Rational>]) -> Result<(), LongRunFault> {
    let size = matrix.len();
    for column in 0..size {
        let pivot = (column..size)
            .find(|row| {
                matrix[*row]
                    .get(&column)
                    .is_some_and(|value| !value.is_zero())
            })
            .ok_or(LongRunFault::SingularSystem)?;
        matrix.swap(column, pivot);
        let divisor = matrix[column]
            .get(&column)
            .cloned()
            .ok_or(LongRunFault::SingularSystem)?;
        let pivot_values = matrix[column]
            .range(column..=size)
            .map(|(index, value)| (*index, value.clone()))
            .collect::<Vec<_>>();
        for (index, value) in pivot_values {
            matrix[column].insert(
                index,
                value.div(&divisor).ok_or(LongRunFault::SingularSystem)?,
            );
        }
        let pivot_row = matrix[column].clone();
        for (row_index, row) in matrix.iter_mut().enumerate() {
            if row_index == column {
                continue;
            }
            let Some(factor) = row.get(&column).cloned() else {
                continue;
            };
            for (index, value) in pivot_row.range(column..) {
                let current = row.get(index).cloned().unwrap_or_else(zero);
                let updated = current.sub(&factor.mul(value));
                if updated.is_zero() {
                    row.remove(index);
                } else {
                    row.insert(*index, updated);
                }
            }
        }
    }
    Ok(())
}

fn add(row: &mut BTreeMap<usize, Rational>, column: usize, value: Rational) {
    let updated = row
        .get(&column)
        .map_or(value.clone(), |current| current.add(&value));
    if updated.is_zero() {
        row.remove(&column);
    } else {
        row.insert(column, updated);
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

/// Why a DTMC long-run fraction could not be computed exactly.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum LongRunFault {
    /// This implementation requires a workload DTMC.
    #[error("long-run stationary analysis requires a DTMC product")]
    RequiresDtmc,
    /// The product, predicate or transition graph is inconsistent.
    #[error("long-run product has an inconsistent shape or probability mass")]
    Shape,
    /// A reachable component has no positive total weight.
    #[error("a long-run component has zero total weight")]
    ZeroWeightComponent,
    /// A weighted transition does not carry the requested reward.
    #[error("a long-run edge lacks the requested weight reward")]
    MissingWeight,
    /// A transition reward is not exact numeric data.
    #[error("a long-run transition weight is not exact numeric data")]
    InvalidWeight,
    /// The stationary linear equations are singular.
    #[error("the stationary equations have no unique solution")]
    SingularSystem,
    /// The component-reachability computation failed.
    #[error(transparent)]
    Reachability(#[from] ReachabilityFault),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact_probabilistic::{ProbKey, ProbProduct, ProductAction};

    fn state(index: u8) -> ProbKey {
        ProbKey {
            binding: None,
            model: vec![index],
            monitor: None,
            accumulator: None,
            pending: None,
        }
    }

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("ratio")
    }

    fn edge(from: usize, to: usize, probability: Rational, weight: i64) -> super::super::ProbEdge {
        super::super::ProbEdge {
            from: ProbStateId(from),
            choice: ProductAction::PostState(Box::new(state(to as u8))),
            action: None,
            draw: None,
            to: ProbStateId(to),
            probability,
            rewards: vec![(
                Identifier::new("duration").expect("an identifier"),
                Value::Integer(Integer::from(weight)),
            )],
        }
    }

    /// FR-199-AC-1/3 (TC-634): the stationary solve preserves the exact
    /// probability ratio, and outgoing rewards weight each source position.
    #[ix_trace_rs::trace("TC-634", "FR-199-AC-1", "FR-199-AC-3")]
    #[test]
    fn dtmc_long_run_solves_stationary_and_weighted_ratios_exactly() {
        let product = ProbProduct {
            states: vec![state(0), state(1)],
            kind: ProductKind::Dtmc,
            edges: vec![
                edge(0, 0, ratio(1799, 1800), 1),
                edge(0, 1, ratio(1, 1800), 1),
                edge(1, 0, one(), 3),
            ],
            initial: vec![],
            decided: vec![],
            labels: vec![vec![true], vec![false]],
        };
        let unweighted = long_run_fraction(&product, &[true, false], None)
            .expect("a positive recurrent component has a stationary distribution");
        assert_eq!(unweighted.components.len(), 1);
        assert_eq!(unweighted.components[0].value, ratio(1800, 1801));
        assert_eq!(unweighted.value(ProbStateId(0)), Some(&ratio(1800, 1801)));

        let duration = Identifier::new("duration").expect("an identifier");
        let weighted = long_run_fraction(&product, &[true, false], Some(&duration))
            .expect("the recurrent component has positive total duration");
        assert_eq!(weighted.components[0].value, ratio(600, 601));
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-204's row-factored operator at a `FreeDelays` state.
//!
//! The digital product builds, at a state where given-delay and free-delay
//! identities race together, one row choice per drawn given vector `g` and
//! free assignment `a`, its edges carrying `P(g)` times the race's mass
//! (FR-196). The semantic actions are the functions from rows to row choices,
//! and FR-204 decides the state by
//! `Σ_g P(g) · opt_a Q(g, a)`, never by enumerating the functions.
//!
//! The evaluators (FR-197, FR-198) realize that operator by evaluating an
//! [`Evaluation`] of the product: each row of a `FreeDelays` state becomes
//! one evaluation node after the product's states, the state moves to row
//! `g`'s node with probability `P(g)`, and the node chooses among the row's
//! choices with their edges scaled by `1 / P(g)`. The node's value is
//! `opt_a Q(g, a)`, so the state's value is the row-factored operator, and a
//! graph precomputation reads the state's support as the union of its rows'
//! supports. The nodes exist only inside the evaluators: the product, its
//! keys and its transition count are unchanged.

use std::borrow::Cow;

use quire_exact::Rational;

use super::{
    ActionChoice, FreeDelayRow, ProbEdge, ProbProduct, ProbStateId, ProductAction, SchedulerChoice,
};

/// The product's `FreeDelays` edges do not form rows: a row-choice edge
/// beside another kind of choice, a body without exactly one row, a repeated
/// row, or row choices of one row with different given-vector masses.
#[derive(Debug)]
pub(super) struct MalformedRows;

/// One row of a `FreeDelays` state: its row choices in canonical order, each
/// its body and its edges.
type Row<'p> = Vec<(&'p FreeDelayRow, Vec<&'p ProbEdge>)>;

/// The rows of the `FreeDelays` state whose edges are `edges`, in canonical
/// order; `None` when the state is not a `FreeDelays` state.
fn rows_of(edges: &[ProbEdge]) -> Result<Option<Vec<Row<'_>>>, MalformedRows> {
    let Some(first) = edges.first() else {
        return Ok(None);
    };
    if row_choice(first)?.is_none() {
        if edges
            .iter()
            .any(|edge| matches!(edge.choice, ProductAction::FreeDelays(_)))
        {
            return Err(MalformedRows);
        }
        return Ok(None);
    }
    let mut rows: Vec<Row<'_>> = Vec::new();
    for edge in edges {
        let body = row_choice(edge)?.ok_or(MalformedRows)?;
        let same_choice = rows
            .last()
            .and_then(|row| row.last())
            .is_some_and(|(last, _)| *last == body);
        if same_choice {
            if let Some((_, choice_edges)) = rows.last_mut().and_then(|row| row.last_mut()) {
                choice_edges.push(edge);
            }
            continue;
        }
        let same_row = rows
            .last()
            .and_then(|row| row.first())
            .is_some_and(|(first, _)| first.given == body.given);
        if same_row {
            if let Some(row) = rows.last_mut() {
                row.push((body, vec![edge]));
            }
            continue;
        }
        if rows.iter().any(|row| {
            row.first()
                .is_some_and(|(first, _)| first.given == body.given)
        }) {
            return Err(MalformedRows);
        }
        rows.push(vec![(body, vec![edge])]);
    }
    Ok(Some(rows))
}

/// The one-row body of a row-choice edge; `None` for another kind of
/// choice.
fn row_choice(edge: &ProbEdge) -> Result<Option<&FreeDelayRow>, MalformedRows> {
    match &edge.choice {
        ProductAction::FreeDelays(rows) => match rows.as_slice() {
            [row] => Ok(Some(row)),
            _ => Err(MalformedRows),
        },
        _ => Ok(None),
    }
}

/// The probability of a row's given vector: the mass of each of its row
/// choices, which must agree and be positive.
fn given_mass(row: &Row<'_>) -> Result<Rational, MalformedRows> {
    let mut masses = row.iter().map(|(_, edges)| {
        edges
            .iter()
            .fold(zero(), |sum, edge| sum.add(&edge.probability))
    });
    let mass = masses.next().ok_or(MalformedRows)?;
    if mass <= zero() || masses.any(|other| other != mass) {
        return Err(MalformedRows);
    }
    Ok(mass)
}

fn zero() -> Rational {
    Rational::from_integer(quire_exact::Integer::from(0_i64))
}

fn one() -> Rational {
    Rational::from_integer(quire_exact::Integer::from(1_i64))
}

/// A product as its evaluators read it: unchanged without a `FreeDelays`
/// state, otherwise with one evaluation node per row after its states.
pub(super) struct Evaluation<'p> {
    /// The product the evaluators run on.
    pub(super) product: Cow<'p, ProbProduct>,
    /// The `FreeDelays` state each evaluation node belongs to, in node order.
    parents: Vec<ProbStateId>,
    /// The number of the product's own states.
    states: usize,
}

impl<'p> Evaluation<'p> {
    /// The evaluation of `product`.
    pub(super) fn of(product: &'p ProbProduct) -> Result<Self, MalformedRows> {
        let states = product.states.len();
        // A product without row choices, or one that already carries its row
        // nodes (a dispatch edge), is its own evaluation.
        let mut free_delays = product.edges.iter().filter_map(|edge| match &edge.choice {
            ProductAction::FreeDelays(rows) => Some(rows.is_empty()),
            _ => None,
        });
        let own = match free_delays.next() {
            None => true,
            Some(dispatch) => dispatch || free_delays.any(|dispatch| dispatch),
        };
        if own {
            return Ok(Self {
                product: Cow::Borrowed(product),
                parents: Vec::new(),
                states,
            });
        }
        let mut edges = Vec::with_capacity(product.edges.len());
        let mut node_edges = Vec::new();
        let mut parents = Vec::new();
        for state in product.ids() {
            let outgoing = product.outgoing(state);
            let Some(rows) = rows_of(outgoing)? else {
                edges.extend(outgoing.iter().cloned());
                continue;
            };
            for row in &rows {
                let mass = given_mass(row)?;
                let node = ProbStateId(states + parents.len());
                parents.push(state);
                edges.push(ProbEdge {
                    from: state,
                    choice: ProductAction::FreeDelays(Vec::new()),
                    action: None,
                    draw: None,
                    to: node,
                    probability: mass.clone(),
                    rewards: Vec::new(),
                });
                for (_, choice_edges) in row {
                    for edge in choice_edges {
                        let mut scaled = (*edge).clone();
                        scaled.from = node;
                        scaled.probability = edge.probability.div(&mass).ok_or(MalformedRows)?;
                        node_edges.push(scaled);
                    }
                }
            }
        }
        edges.append(&mut node_edges);
        let mut evaluation = Self {
            product: Cow::Borrowed(product),
            parents,
            states,
        };
        let keys = evaluation.extend(&product.states).ok_or(MalformedRows)?;
        let labels = if product.labels.is_empty() {
            Vec::new()
        } else {
            evaluation.extend(&product.labels).ok_or(MalformedRows)?
        };
        evaluation.product = Cow::Owned(ProbProduct {
            states: keys,
            kind: product.kind,
            edges,
            initial: product.initial.clone(),
            decided: product.decided.clone(),
            labels,
            fairness_operations: product.fairness_operations.clone(),
            fairness: product.fairness.clone(),
        });
        Ok(evaluation)
    }

    /// The product state an evaluation state stands for: itself, or the
    /// `FreeDelays` state a row node belongs to.
    pub(super) fn state_of(&self, state: ProbStateId) -> ProbStateId {
        state
            .index()
            .checked_sub(self.states)
            .and_then(|node| self.parents.get(node).copied())
            .unwrap_or(state)
    }

    /// Whether `state` is one of the product's own states.
    pub(super) fn is_product_state(&self, state: ProbStateId) -> bool {
        state.index() < self.states
    }

    /// A per-state vector of the product extended to the evaluation: each
    /// node takes its state's entry. `None` when `per_state` does not have
    /// one entry per product state.
    pub(super) fn extend<T: Clone>(&self, per_state: &[T]) -> Option<Vec<T>> {
        if per_state.len() != self.states {
            return None;
        }
        let mut extended = per_state.to_vec();
        for parent in &self.parents {
            extended.push(per_state.get(parent.index())?.clone());
        }
        Some(extended)
    }

    /// A per-node vector of the evaluation restricted to the product's
    /// states.
    pub(super) fn restrict<T>(&self, mut per_node: Vec<T>) -> Vec<T> {
        per_node.truncate(self.states);
        per_node
    }
}

/// The index of the first evaluation node of the `FreeDelays` state `state`'s
/// rows, and the rows; `None` when `state` is not a `FreeDelays` state.
fn rows_at(product: &ProbProduct, state: ProbStateId) -> Option<(usize, Vec<Row<'_>>)> {
    let mut node = product.states.len();
    for earlier in product.ids().take_while(|earlier| *earlier < state) {
        node += rows_of(product.outgoing(earlier))
            .ok()?
            .map_or(0, |rows| rows.len());
    }
    let rows = rows_of(product.outgoing(state)).ok()??;
    Some((node, rows))
}

/// The whole `FreeDelays` action a policy takes at `state` (FR-204, FR-202):
/// for each row in order, the row choice the policy selects at the row's
/// evaluation node. `policy` is an evaluator's policy, whose entries after the
/// product's states select, row by row in state order, a row choice by its
/// index. `None` when `state` is not a `FreeDelays` state or the policy does
/// not select a choice for every row.
pub fn free_delays_choice(
    product: &ProbProduct,
    state: ProbStateId,
    policy: &[Option<usize>],
) -> Option<ProductAction> {
    let (node, rows) = rows_at(product, state)?;
    let mut chosen = Vec::with_capacity(rows.len());
    for (offset, row) in rows.iter().enumerate() {
        let index = (*policy.get(node + offset)?)?;
        let (body, _) = row.get(index)?;
        chosen.push((*body).clone());
    }
    Some(ProductAction::FreeDelays(chosen))
}

/// Why a randomized policy gives no entry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DistributionFault {
    /// Not a `FreeDelays` state, a row without a selected choice, an
    /// out-of-range choice, or a distribution FR-202 refuses.
    NoChoice,
    /// The entry's alternatives exceed the transition limit.
    TooMany,
}

/// The exact randomized `FreeDelays` action a randomized policy takes at
/// `state` (FR-204 "Evidence": randomized exact-rational entries): each
/// row's [`SchedulerChoice`] is a distribution over the row's choices, and
/// the entry is the product measure over the whole actions, so every row has
/// the distribution the policy gives it. The alternatives are in canonical
/// order (rows in order, each row's choices by index) and number at most
/// `max_alternatives`.
///
/// # Errors
///
/// [`DistributionFault`] when `state` is not a `FreeDelays` state, the policy
/// does not select a choice for every row, a row's distribution is refused
/// by [`scheduler_distribution`], or the product has too many alternatives.
pub(super) fn free_delays_distribution(
    product: &ProbProduct,
    state: ProbStateId,
    policy: &[Option<SchedulerChoice>],
    max_alternatives: usize,
) -> Result<Vec<(ProductAction, Rational)>, DistributionFault> {
    let fault = DistributionFault::NoChoice;
    let (node, rows) = rows_at(product, state).ok_or(fault)?;
    let mut actions: Vec<(Vec<FreeDelayRow>, Rational)> = vec![(Vec::new(), one())];
    for (offset, row) in rows.iter().enumerate() {
        let choice = policy
            .get(node + offset)
            .and_then(Option::as_ref)
            .ok_or(fault)?;
        let weighted = scheduler_distribution(choice).ok_or(fault)?;
        if actions.len().saturating_mul(weighted.len()) > max_alternatives {
            return Err(DistributionFault::TooMany);
        }
        let mut next = Vec::with_capacity(actions.len() * weighted.len());
        for (chosen, mass) in &actions {
            for (index, weight) in &weighted {
                let (body, _) = row.get(*index).ok_or(fault)?;
                let mut extended = chosen.clone();
                extended.push((*body).clone());
                next.push((extended, mass.mul(weight)));
            }
        }
        actions = next;
    }
    Ok(actions
        .into_iter()
        .map(|(rows, mass)| (ProductAction::FreeDelays(rows), mass))
        .collect())
}

/// The alternatives of a scheduler choice by action index, ascending
/// (FR-202's canonical choice order); `None` when an index repeats, a
/// probability is zero or negative, or the probabilities do not sum to 1.
pub(super) fn scheduler_distribution(choice: &SchedulerChoice) -> Option<Vec<(usize, Rational)>> {
    let mut weighted: Vec<(usize, Rational)> = match choice {
        SchedulerChoice::Deterministic(ActionChoice(index)) => vec![(*index, one())],
        SchedulerChoice::Randomized(alternatives) => alternatives
            .iter()
            .map(|(ActionChoice(index), probability)| (*index, probability.clone()))
            .collect(),
    };
    weighted.sort_by_key(|(index, _)| *index);
    if weighted.windows(2).any(|pair| pair[0].0 == pair[1].0)
        || weighted
            .iter()
            .any(|(_, probability)| *probability <= zero())
    {
        return None;
    }
    let total = weighted
        .iter()
        .fold(zero(), |sum, (_, probability)| sum.add(probability));
    (total == one()).then_some(weighted)
}

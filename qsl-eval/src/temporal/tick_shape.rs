// SPDX-License-Identifier: AGPL-3.0-or-later
//! The timing shape of a tick-monitor formula (QSL FR-251 "Buffers"): for
//! every temporal operator its direction, the *lookahead* after which its
//! value at a position can no longer change, and, for every interval
//! operator, the *retention* in ticks an entry is kept for.
//!
//! The retention of an interval operator is the span for which a consumer can
//! still read the entry: the operator's own window if it looks backwards,
//! plus the time until every enclosing operator has settled the instances
//! that read it. For a chain of nested interval operators this is the sum of
//! the upper bounds along the path plus one tick cover (`1 + 2u` ticks) per
//! level (FR-251-AC-18, AC-21); the shape also counts the lookahead of the
//! operator's own operands, so an entry survives until its readers settle.

use std::collections::BTreeMap;

use qsl_semantics::check::{BinaryTemporal, CheckedFormula, FormulaNode, UnaryTemporal};
use quire_exact::{Integer, Rational};

use super::tick_monitor::TickBound;

fn int(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

fn larger(a: Rational, b: Rational) -> Rational {
    if a >= b {
        a
    } else {
        b
    }
}

/// The timing facts of one temporal operator.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeShape {
    /// Whether the operator reads later events.
    pub future: bool,
    /// Whether the operator is the dual (`always`, `historically`,
    /// `release`, `triggered`).
    pub dual: bool,
    /// The left operand of a binary operator.
    pub left: Option<usize>,
    /// The (right) operand.
    pub right: usize,
    /// The tick interval, or `None` on an unbounded operator.
    pub bound: Option<TickBound>,
    /// Ticks after an instance's tick at which its value is settled, or
    /// `None` when it never settles by time (an unbounded operator below).
    pub lookahead: Option<Rational>,
    /// For an interval operator, the ticks an entry is retained for.
    pub retention: Rational,
}

/// The timing shape of a whole formula.
#[derive(Clone, Debug)]
pub struct TickShape {
    nodes: BTreeMap<usize, NodeShape>,
    width: Rational,
}

/// The temporal operators reached from `index` through Boolean nodes only.
pub fn temporal_leaves(formula: &CheckedFormula, index: usize) -> Vec<usize> {
    let nodes = formula.nodes();
    let mut out = Vec::new();
    let mut stack = vec![index];
    while let Some(next) = stack.pop() {
        match &nodes[next] {
            FormulaNode::Unary { .. } | FormulaNode::Binary { .. } => out.push(next),
            other => stack.extend(other.operands().map(|operand| operand.index())),
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}

/// The direction and duality of a temporal operator node, with its operands.
pub fn operator_parts(node: &FormulaNode) -> Option<(bool, bool, Option<usize>, usize)> {
    match node {
        FormulaNode::Unary {
            operator, operand, ..
        } => {
            let (future, dual) = match operator {
                UnaryTemporal::Eventually => (true, false),
                UnaryTemporal::Always => (true, true),
                UnaryTemporal::Once => (false, false),
                UnaryTemporal::Historically => (false, true),
            };
            Some((future, dual, None, operand.index()))
        }
        FormulaNode::Binary {
            operator,
            left,
            right,
            ..
        } => {
            let (future, dual) = match operator {
                BinaryTemporal::Until => (true, false),
                BinaryTemporal::Release => (true, true),
                BinaryTemporal::Since => (false, false),
                BinaryTemporal::Triggered => (false, true),
            };
            Some((future, dual, Some(left.index()), right.index()))
        }
        _ => None,
    }
}

impl TickShape {
    /// The shape of `formula` under the interval `bounds` (by node index) and
    /// the clock `uncertainty` in ticks.
    pub fn of(
        formula: &CheckedFormula,
        bounds: &BTreeMap<usize, TickBound>,
        uncertainty: &Rational,
    ) -> Self {
        let width = int(1).add(&int(2).mul(uncertainty));
        let arena = formula.nodes();
        let operands = |index: usize| -> Vec<usize> {
            let (_, _, left, right) =
                operator_parts(&arena[index]).unwrap_or((false, false, None, 0));
            left.into_iter()
                .chain([right])
                .flat_map(|operand| temporal_leaves(formula, operand))
                .collect()
        };
        let temporal: Vec<usize> = (0..arena.len())
            .filter(|index| operator_parts(&arena[*index]).is_some())
            .collect();
        // Lookahead, children first: an operator precedes its operands in the
        // arena, so descending index order visits operands before operators.
        let mut lookahead: BTreeMap<usize, Option<Rational>> = BTreeMap::new();
        for &index in temporal.iter().rev() {
            let (future, _, _, _) =
                operator_parts(&arena[index]).unwrap_or((false, false, None, 0));
            let mut below = Some(int(0));
            for child in operands(index) {
                below = match (below, lookahead.get(&child).cloned().flatten()) {
                    (Some(a), Some(b)) => Some(larger(a, b)),
                    _ => None,
                };
            }
            let value = match bounds.get(&index) {
                Some(bound) if future => below.map(|b| bound.upper.add(&width).add(&b)),
                Some(_) => below,
                None => None,
            };
            lookahead.insert(index, value);
        }
        let finite = |index: usize| -> Rational {
            lookahead
                .get(&index)
                .cloned()
                .flatten()
                .unwrap_or_else(|| int(0))
        };
        // The evaluation reach `E` of every operator, parents first.
        let root = formula.root().index();
        let mut reach: BTreeMap<usize, Rational> = BTreeMap::new();
        let mut rooted: BTreeMap<usize, bool> = BTreeMap::new();
        let join = |leaves: &[usize]| -> Rational {
            leaves
                .iter()
                .fold(int(0), |acc, leaf| larger(acc, finite(*leaf)))
        };
        let root_leaves = temporal_leaves(formula, root);
        for &index in &temporal {
            let mut candidates = vec![finite(index)];
            let mut is_rooted = false;
            if index == root {
                is_rooted = true;
            }
            if root_leaves.contains(&index) && !temporal.contains(&root) {
                is_rooted = true;
                candidates.push(join(&root_leaves));
            }
            for &parent in temporal.iter().filter(|p| **p < index) {
                if !operands(parent).contains(&index) {
                    continue;
                }
                let Some(bound) = bounds.get(&parent) else {
                    // An unbounded consumer: its instances fold an entry when
                    // it is dropped, so the operands of one context are kept
                    // for the context's longest lookahead.
                    is_rooted = true;
                    candidates.push(join(&operands(parent)));
                    continue;
                };
                let (future, ..) =
                    operator_parts(&arena[parent]).unwrap_or((false, false, None, 0));
                let offset = if future {
                    int(0)
                } else {
                    bound.upper.add(&width)
                };
                let parent_reach = reach.get(&parent).cloned().unwrap_or_else(|| int(0));
                candidates.push(offset.add(&parent_reach));
            }
            rooted.insert(index, is_rooted);
            reach.insert(index, candidates.into_iter().fold(int(0), larger));
        }
        let mut nodes = BTreeMap::new();
        for &index in &temporal {
            let Some((future, dual, left, right)) = operator_parts(&arena[index]) else {
                continue;
            };
            let bound = bounds.get(&index).cloned();
            let own = bound.as_ref().map_or_else(
                || int(0),
                |bound| {
                    if future {
                        int(0)
                    } else {
                        bound.upper.add(&width)
                    }
                },
            );
            let e = reach.get(&index).cloned().unwrap_or_else(|| int(0));
            let retention = if rooted.get(&index).copied().unwrap_or(false) {
                larger(own, e)
            } else {
                own.add(&e)
            };
            nodes.insert(
                index,
                NodeShape {
                    future,
                    dual,
                    left,
                    right,
                    bound,
                    lookahead: lookahead.get(&index).cloned().flatten(),
                    retention,
                },
            );
        }
        Self { nodes, width }
    }

    /// The shape of the temporal operator at `index`.
    pub fn node(&self, index: usize) -> Option<&NodeShape> {
        self.nodes.get(&index)
    }

    /// All temporal operators, by ascending index.
    pub fn nodes(&self) -> &BTreeMap<usize, NodeShape> {
        &self.nodes
    }

    /// The widened tick-cover width `1 + 2u`.
    pub fn width(&self) -> &Rational {
        &self.width
    }
}

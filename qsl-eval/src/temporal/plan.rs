// SPDX-License-Identifier: AGPL-3.0-or-later
//! The evaluation plan of a checked formula: its nodes in the four scan
//! families the evaluator runs, the past reach and left floor each node
//! needs (ADR-018 SM-8), and the safety-fragment test (ADR-014 A-4 as
//! amended by ADR-018 IV-4).
//!
//! `eventually`/`always`, `once`/`historically`, `until`/`release` and
//! `since`/`triggered` are one scan each: the second of each pair is the
//! Boolean dual of the first, `not (not p until[a,b] not q)`, so it is the
//! same scan over negated operands with a negated result. Every walk here
//! runs over the arena in index order, forward or reverse, so none recurses.

use qsl_foundation::bound::{TemporalInterval, TemporalUpper};
use qsl_foundation::InternalFault;
use qsl_semantics::check::{BinaryTemporal, CheckedFormula, FormulaNode, UnaryTemporal};

pub(super) const STAGE: &str = "eval.temporal";

pub(super) fn fault(invariant: &'static str) -> InternalFault {
    InternalFault::new(STAGE, invariant)
}

/// The slot `index` of `slice`, or the fault that the plan indexes outside
/// its own arena.
pub(super) fn slot<T: Copy>(slice: &[T], index: usize) -> Result<T, InternalFault> {
    slice
        .get(index)
        .copied()
        .ok_or_else(|| fault("plan-index-in-arena"))
}

/// Whether a scan looks forward or back from its position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Direction {
    Future,
    Past,
}

/// The four scan families; each has a dual that negates its operands and
/// its result.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Base {
    /// `eventually`; dual `always`.
    Eventually,
    /// `once`; dual `historically`.
    Once,
    /// `until`; dual `release`.
    Until,
    /// `since`; dual `triggered`.
    Since,
}

impl Base {
    pub(super) const fn direction(self) -> Direction {
        match self {
            Self::Eventually | Self::Until => Direction::Future,
            Self::Once | Self::Since => Direction::Past,
        }
    }
}

/// An operator's offsets.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Window {
    /// `[lower, upper]`, both inclusive.
    Closed { lower: u64, upper: u64 },
    /// `[lower, *]`; an operator with no interval is `[0, *]`.
    From { lower: u64 },
}

impl Window {
    fn of(interval: Option<TemporalInterval>) -> Self {
        match interval {
            None => Self::From { lower: 0 },
            Some(interval) => match interval.upper() {
                TemporalUpper::Finite(upper) => Self::Closed {
                    lower: interval.lower(),
                    upper,
                },
                TemporalUpper::Open => Self::From {
                    lower: interval.lower(),
                },
            },
        }
    }
}

/// One scan node.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct Scan {
    pub(super) base: Base,
    /// Whether this is the dual of `base`.
    pub(super) dual: bool,
    pub(super) window: Window,
    /// The left operand of `until`/`since` and their duals.
    pub(super) guard: Option<usize>,
    /// The operand whose value is the scan's witness.
    pub(super) witness: usize,
}

/// One node of the plan.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Node {
    Constant(bool),
    Atom,
    Not(usize),
    And(usize, usize),
    Or(usize, usize),
    Implies(usize, usize),
    Scan(Scan),
}

/// A checked formula as the evaluator reads it.
#[derive(Debug)]
pub(super) struct Plan {
    pub(super) nodes: Vec<Node>,
    pub(super) root: usize,
    /// The `holds` atoms, in clause-node (arena) order.
    pub(super) atoms: Vec<usize>,
    /// For each node, `T` such that its truth at every position `q <= -T`
    /// equals its truth at `-T`: atoms read false before position 0 and
    /// constants are unchanged (QSpec FR-092), so a node's truth settles a
    /// bounded distance before the origin.
    pub(super) left_floor: Vec<u128>,
}

impl Plan {
    pub(super) fn new(formula: &CheckedFormula) -> Result<Self, InternalFault> {
        let mut nodes = Vec::with_capacity(formula.nodes().len());
        let mut atoms = Vec::new();
        for (index, node) in formula.nodes().iter().enumerate() {
            nodes.push(match node {
                FormulaNode::Constant(value) => Node::Constant(*value),
                FormulaNode::Holds { .. } => {
                    atoms.push(index);
                    Node::Atom
                }
                FormulaNode::Not(operand) => Node::Not(operand.index()),
                FormulaNode::And(left, right) => Node::And(left.index(), right.index()),
                FormulaNode::Or(left, right) => Node::Or(left.index(), right.index()),
                FormulaNode::Implies(left, right) => Node::Implies(left.index(), right.index()),
                FormulaNode::Unary {
                    operator,
                    interval,
                    operand,
                } => {
                    let (base, dual) = match operator {
                        UnaryTemporal::Eventually => (Base::Eventually, false),
                        UnaryTemporal::Always => (Base::Eventually, true),
                        UnaryTemporal::Once => (Base::Once, false),
                        UnaryTemporal::Historically => (Base::Once, true),
                    };
                    Node::Scan(Scan {
                        base,
                        dual,
                        window: Window::of(*interval),
                        guard: None,
                        witness: operand.index(),
                    })
                }
                FormulaNode::Binary {
                    operator,
                    interval,
                    left,
                    right,
                } => {
                    let (base, dual) = match operator {
                        BinaryTemporal::Until => (Base::Until, false),
                        BinaryTemporal::Release => (Base::Until, true),
                        BinaryTemporal::Since => (Base::Since, false),
                        BinaryTemporal::Triggered => (Base::Since, true),
                    };
                    Node::Scan(Scan {
                        base,
                        dual,
                        window: Window::of(*interval),
                        guard: Some(left.index()),
                        witness: right.index(),
                    })
                }
            });
        }
        let mut plan = Self {
            nodes,
            root: formula.root().index(),
            atoms,
            left_floor: Vec::new(),
        };
        plan.left_floor = plan.floors()?;
        Ok(plan)
    }

    /// The operands of `node`.
    fn operands(node: Node) -> [Option<usize>; 2] {
        match node {
            Node::Constant(_) | Node::Atom => [None, None],
            Node::Not(operand) => [Some(operand), None],
            Node::And(left, right) | Node::Or(left, right) | Node::Implies(left, right) => {
                [Some(left), Some(right)]
            }
            Node::Scan(scan) => [scan.guard, Some(scan.witness)],
        }
    }

    /// Fold `per_node` over the arena from the last node to the first, so
    /// every operand's value exists before its operator's. `combine` gets
    /// the node and the maximum of its operands' values.
    fn fold_up(&self, combine: impl Fn(Node, u128) -> u128) -> Result<Vec<u128>, InternalFault> {
        let mut values = vec![0_u128; self.nodes.len()];
        for index in (0..self.nodes.len()).rev() {
            let node = slot(&self.nodes, index)?;
            let mut deepest = 0_u128;
            for operand in Self::operands(node).into_iter().flatten() {
                if operand <= index {
                    return Err(fault("operands-follow-operator"));
                }
                deepest = deepest.max(slot(&values, operand)?);
            }
            let value = combine(node, deepest);
            *values
                .get_mut(index)
                .ok_or_else(|| fault("plan-index-in-arena"))? = value;
        }
        Ok(values)
    }

    fn floors(&self) -> Result<Vec<u128>, InternalFault> {
        self.fold_up(|node, operands| match node {
            Node::Constant(_) => 0,
            Node::Atom => 1,
            Node::Not(_) | Node::And(..) | Node::Or(..) | Node::Implies(..) => operands,
            Node::Scan(scan) => match (scan.base.direction(), scan.window) {
                (Direction::Future, Window::Closed { upper, .. }) => {
                    operands.saturating_add(u128::from(upper))
                }
                (Direction::Future, Window::From { lower }) => {
                    operands.saturating_add(u128::from(lower))
                }
                (Direction::Past, _) => operands,
            },
        })
    }

    /// ADR-018 SM-8: each node's past reach on a lasso whose loop holds
    /// `loop_len` positions.
    pub(super) fn past_reach(&self, loop_len: u128) -> Result<Vec<u128>, InternalFault> {
        self.fold_up(|node, operands| match node {
            Node::Constant(_) | Node::Atom => 0,
            Node::Not(_) | Node::And(..) | Node::Or(..) | Node::Implies(..) => operands,
            Node::Scan(scan) => match (scan.base.direction(), scan.window) {
                (Direction::Future, _) => operands,
                (Direction::Past, Window::Closed { upper, .. }) => {
                    operands.saturating_add(u128::from(upper))
                }
                (Direction::Past, Window::From { lower }) => operands
                    .saturating_add(u128::from(lower))
                    .saturating_add(loop_len),
            },
        })
    }

    /// Whether the formula is in the safety fragment (ADR-014 A-4, ADR-018
    /// IV-4): its negation normal form uses only atoms, negated atoms,
    /// `and`, `or`, `always`, `release`, the past operators and interval
    /// operators, so `eventually[a,*]` and `until[a,*]` are excluded like
    /// `eventually` and `until`. Read from the checked formula alone.
    pub(super) fn is_safety(&self) -> Result<bool, InternalFault> {
        let mut pending = vec![(self.root, false)];
        while let Some((index, negated)) = pending.pop() {
            match slot(&self.nodes, index)? {
                Node::Constant(_) | Node::Atom => {}
                Node::Not(operand) => pending.push((operand, !negated)),
                Node::And(left, right) | Node::Or(left, right) => {
                    pending.push((left, negated));
                    pending.push((right, negated));
                }
                Node::Implies(left, right) => {
                    pending.push((left, !negated));
                    pending.push((right, negated));
                }
                Node::Scan(scan) => {
                    // A scan is `eventually`/`until`-shaped unless negation
                    // normal form makes it `always`/`release`-shaped.
                    let universal = scan.dual != negated;
                    let admitted = match (scan.base.direction(), scan.window) {
                        (Direction::Past, _) => true,
                        (Direction::Future, Window::Closed { .. }) => true,
                        (Direction::Future, Window::From { .. }) => universal,
                    };
                    if !admitted {
                        return Ok(false);
                    }
                    pending.extend(scan.guard.map(|guard| (guard, negated)));
                    pending.push((scan.witness, negated));
                }
            }
        }
        Ok(true)
    }
}

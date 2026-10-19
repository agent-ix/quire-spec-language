// SPDX-License-Identifier: AGPL-3.0-or-later
//! A checked temporal formula as the translation reads it: an interned arena
//! in which the negation of every future operator is already pushed to its
//! dual (ADR-014 A-4's negation normal form), and a state formula, one with
//! no future operator, stays whole as a leaf of that skeleton.
//!
//! `historically` and `triggered` are the negation of `once` and `since`
//! over negated operands, and an operand is always interned before its
//! operator, so every walk here runs over the arena in index order and none
//! recurses.

use std::collections::BTreeMap;

use qsl_foundation::bound::{TemporalInterval, TemporalUpper};
use qsl_semantics::check::{
    BinaryTemporal, CheckedFormula, FormulaNode, FormulaNodeId, UnaryTemporal,
};

use crate::automaton::error::AutomatonError;
use crate::automaton::letter::Atom;

/// One node of the arena, by index. Operands have smaller indexes than their
/// operators.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct NodeId(usize);

impl NodeId {
    pub(super) const fn index(self) -> usize {
        self.0
    }
}

/// The offsets of an operator.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
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

    pub(super) const fn lower(self) -> u64 {
        match self {
            Self::Closed { lower, .. } | Self::From { lower } => lower,
        }
    }

    /// The same window after its `lower` positions have passed (ADR-018
    /// IV-3: an interval operator is a shift by its lower bound, then the
    /// operator with lower bound 0).
    fn shifted(self) -> Self {
        match self {
            Self::Closed { lower, upper } => Self::Closed {
                lower: 0,
                upper: upper.saturating_sub(lower),
            },
            Self::From { .. } => Self::From { lower: 0 },
        }
    }
}

/// A future operator; the second of each pair is the dual of the first.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum FutureKind {
    Eventually,
    Always,
    Until,
    Release,
}

impl FutureKind {
    /// Whether an unbounded operator of this kind promises something that
    /// must eventually happen (a liveness obligation).
    pub(super) const fn is_eventuality(self) -> bool {
        matches!(self, Self::Eventually | Self::Until)
    }
}

/// A past operator whose negation is `historically` or `triggered`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum PastKind {
    Once,
    Since,
}

/// One node of the arena.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) enum Node {
    Constant(bool),
    Atom(Atom),
    /// The negation of a state formula.
    Not(NodeId),
    And(NodeId, NodeId),
    Or(NodeId, NodeId),
    Past {
        kind: PastKind,
        window: Window,
        /// The left operand of `since`.
        left: Option<NodeId>,
        /// The operand of `once`, the witness of `since`.
        right: NodeId,
    },
    Future {
        kind: FutureKind,
        window: Window,
        /// The left operand of `until` and `release`.
        left: Option<NodeId>,
        /// The operand of `eventually` and `always`, the witness of
        /// `until`, the held operand of `release`.
        right: NodeId,
    },
}

/// The arena.
#[derive(Debug, Default)]
pub(super) struct Ir {
    nodes: Vec<Node>,
    has_future: Vec<bool>,
    /// For a future operator, the same operator with lower bound 0; the
    /// node itself otherwise.
    cores: Vec<NodeId>,
    interned: BTreeMap<Node, NodeId>,
}

pub(super) fn invariant(invariant: &'static str) -> AutomatonError {
    AutomatonError::Invariant { invariant }
}

impl Ir {
    pub(super) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(super) fn node(&self, id: NodeId) -> Result<Node, AutomatonError> {
        self.nodes
            .get(id.0)
            .copied()
            .ok_or_else(|| invariant("node-in-arena"))
    }

    /// Whether the node holds a future operator.
    pub(super) fn has_future(&self, id: NodeId) -> bool {
        self.has_future.get(id.0).copied().unwrap_or(false)
    }

    /// The node's lower-bound-0 core (itself unless it is a future operator
    /// with a lower bound).
    pub(super) fn core(&self, id: NodeId) -> NodeId {
        self.cores.get(id.0).copied().unwrap_or(id)
    }

    /// The operands of `node`.
    pub(super) fn operands(node: Node) -> [Option<NodeId>; 2] {
        match node {
            Node::Constant(_) | Node::Atom(_) => [None, None],
            Node::Not(only) => [Some(only), None],
            Node::And(left, right) | Node::Or(left, right) => [Some(left), Some(right)],
            Node::Past { left, right, .. } | Node::Future { left, right, .. } => {
                [left, Some(right)]
            }
        }
    }

    fn intern(&mut self, node: Node, future: bool, core: Option<NodeId>) -> NodeId {
        if let Some(&id) = self.interned.get(&node) {
            return id;
        }
        let id = NodeId(self.nodes.len());
        self.nodes.push(node);
        self.has_future.push(future);
        self.cores.push(core.unwrap_or(id));
        self.interned.insert(node, id);
        id
    }

    pub(super) fn constant(&mut self, value: bool) -> NodeId {
        self.intern(Node::Constant(value), false, None)
    }

    pub(super) fn atom(&mut self, atom: Atom) -> NodeId {
        self.intern(Node::Atom(atom), false, None)
    }

    /// The negation of a state formula.
    pub(super) fn not(&mut self, operand: NodeId) -> Result<NodeId, AutomatonError> {
        if self.has_future(operand) {
            return Err(invariant("negation-of-state-formula"));
        }
        Ok(match self.node(operand)? {
            Node::Constant(value) => self.constant(!value),
            Node::Not(inner) => inner,
            _ => self.intern(Node::Not(operand), false, None),
        })
    }

    pub(super) fn and(&mut self, left: NodeId, right: NodeId) -> Result<NodeId, AutomatonError> {
        self.connective(left, right, true)
    }

    pub(super) fn or(&mut self, left: NodeId, right: NodeId) -> Result<NodeId, AutomatonError> {
        self.connective(left, right, false)
    }

    /// `left and right` (`conjunction`) or `left or right`, folding the
    /// constants and the repeated operand.
    fn connective(
        &mut self,
        left: NodeId,
        right: NodeId,
        conjunction: bool,
    ) -> Result<NodeId, AutomatonError> {
        for (constant, other) in [(left, right), (right, left)] {
            if let Node::Constant(value) = self.node(constant)? {
                return Ok(if value == conjunction {
                    other
                } else {
                    constant
                });
            }
        }
        if left == right {
            return Ok(left);
        }
        let (first, second) = (left.min(right), left.max(right));
        let node = if conjunction {
            Node::And(first, second)
        } else {
            Node::Or(first, second)
        };
        let future = self.has_future(left) || self.has_future(right);
        Ok(self.intern(node, future, None))
    }

    /// A past operator; its operands must be state formulas.
    pub(super) fn past(
        &mut self,
        kind: PastKind,
        window: Window,
        left: Option<NodeId>,
        right: NodeId,
    ) -> Result<NodeId, AutomatonError> {
        if self.has_future(right) || left.is_some_and(|left| self.has_future(left)) {
            return Err(AutomatonError::PastOverFuture);
        }
        Ok(self.intern(
            Node::Past {
                kind,
                window,
                left,
                right,
            },
            false,
            None,
        ))
    }

    /// A future operator, with its lower-bound-0 core interned first.
    pub(super) fn future(
        &mut self,
        kind: FutureKind,
        window: Window,
        left: Option<NodeId>,
        right: NodeId,
    ) -> NodeId {
        let core = (window.lower() != 0).then(|| self.future(kind, window.shifted(), left, right));
        self.intern(
            Node::Future {
                kind,
                window,
                left,
                right,
            },
            true,
            core,
        )
    }

    /// The nodes reachable from `root` through operands and cores, with the
    /// negation of every state operand of a future operator (the translation
    /// asserts it when a counter's operand does not hold), in ascending
    /// order.
    pub(super) fn reachable(&mut self, root: NodeId) -> Result<Vec<NodeId>, AutomatonError> {
        let mut seen = vec![false; self.len()];
        let mut pending = vec![root];
        while let Some(id) = pending.pop() {
            if seen.get(id.0).copied().unwrap_or(true) {
                continue;
            }
            if let Some(slot) = seen.get_mut(id.0) {
                *slot = true;
            }
            let node = self.node(id)?;
            pending.extend(Self::operands(node).into_iter().flatten());
            if let Node::Future { left, right, .. } = node {
                pending.push(self.core(id));
                for operand in left.into_iter().chain([right]) {
                    if !self.has_future(operand) {
                        pending.push(self.not(operand)?);
                    }
                }
                seen.resize(self.len(), false);
            }
        }
        Ok(seen
            .iter()
            .enumerate()
            .filter_map(|(index, reached)| reached.then_some(NodeId(index)))
            .collect())
    }
}

/// A checked formula lowered into the arena.
pub(super) struct Lowered {
    pub(super) ir: Ir,
    /// The `holds` atoms, in clause-node order; atom `i` is `atoms[i]`.
    pub(super) atoms: Vec<FormulaNodeId>,
    /// The formula, and its negation in negation normal form.
    pub(super) positive: NodeId,
    pub(super) negative: NodeId,
}

/// Lower `formula`: every `holds` atom becomes a letter atom, `implies`
/// becomes `or`, and each node is built with its negation.
pub(super) fn lower(formula: &CheckedFormula) -> Result<Lowered, AutomatonError> {
    let mut ir = Ir::default();
    let mut atoms = Vec::new();
    let mut letters = vec![None; formula.nodes().len()];
    for (index, node) in formula.nodes().iter().enumerate() {
        if matches!(node, FormulaNode::Holds { .. }) {
            let id = formula
                .node_id(index)
                .ok_or_else(|| invariant("atom-has-node-id"))?;
            if let Some(slot) = letters.get_mut(index) {
                *slot = Some(Atom::new(atoms.len())?);
            }
            atoms.push(id);
        }
    }
    // Both polarities of each node, filled from the last node to the first,
    // so every operand's pair exists before its operator's.
    let mut pairs: Vec<Option<(NodeId, NodeId)>> = vec![None; formula.nodes().len()];
    for (index, node) in formula.nodes().iter().enumerate().rev() {
        let pair_of = |operand: FormulaNodeId| {
            pairs
                .get(operand.index())
                .copied()
                .flatten()
                .ok_or_else(|| invariant("operands-follow-operator"))
        };
        let pair = match node {
            FormulaNode::Constant(value) => (ir.constant(*value), ir.constant(!*value)),
            FormulaNode::Holds { .. } => {
                let atom = letters
                    .get(index)
                    .copied()
                    .flatten()
                    .ok_or_else(|| invariant("atom-has-letter"))?;
                let positive = ir.atom(atom);
                (positive, ir.not(positive)?)
            }
            FormulaNode::Not(operand) => {
                let (positive, negative) = pair_of(*operand)?;
                (negative, positive)
            }
            FormulaNode::And(left, right) => {
                let ((lp, ln), (rp, rn)) = (pair_of(*left)?, pair_of(*right)?);
                (ir.and(lp, rp)?, ir.or(ln, rn)?)
            }
            FormulaNode::Or(left, right) => {
                let ((lp, ln), (rp, rn)) = (pair_of(*left)?, pair_of(*right)?);
                (ir.or(lp, rp)?, ir.and(ln, rn)?)
            }
            FormulaNode::Implies(left, right) => {
                let ((lp, ln), (rp, rn)) = (pair_of(*left)?, pair_of(*right)?);
                (ir.or(ln, rp)?, ir.and(lp, rn)?)
            }
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                let (positive, negative) = pair_of(*operand)?;
                let window = Window::of(*interval);
                match operator {
                    UnaryTemporal::Eventually => (
                        ir.future(FutureKind::Eventually, window, None, positive),
                        ir.future(FutureKind::Always, window, None, negative),
                    ),
                    UnaryTemporal::Always => (
                        ir.future(FutureKind::Always, window, None, positive),
                        ir.future(FutureKind::Eventually, window, None, negative),
                    ),
                    UnaryTemporal::Once => {
                        let once = ir.past(PastKind::Once, window, None, positive)?;
                        (once, ir.not(once)?)
                    }
                    UnaryTemporal::Historically => {
                        let once = ir.past(PastKind::Once, window, None, negative)?;
                        (ir.not(once)?, once)
                    }
                }
            }
            FormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => {
                let ((lp, ln), (rp, rn)) = (pair_of(*left)?, pair_of(*right)?);
                let window = Window::of(*interval);
                match operator {
                    BinaryTemporal::Until => (
                        ir.future(FutureKind::Until, window, Some(lp), rp),
                        ir.future(FutureKind::Release, window, Some(ln), rn),
                    ),
                    BinaryTemporal::Release => (
                        ir.future(FutureKind::Release, window, Some(lp), rp),
                        ir.future(FutureKind::Until, window, Some(ln), rn),
                    ),
                    BinaryTemporal::Since => {
                        let since = ir.past(PastKind::Since, window, Some(lp), rp)?;
                        (since, ir.not(since)?)
                    }
                    BinaryTemporal::Triggered => {
                        let since = ir.past(PastKind::Since, window, Some(ln), rn)?;
                        (ir.not(since)?, since)
                    }
                }
            }
        };
        if let Some(slot) = pairs.get_mut(index) {
            *slot = Some(pair);
        }
    }
    let (positive, negative) = pairs
        .get(formula.root().index())
        .copied()
        .flatten()
        .ok_or_else(|| invariant("root-has-pair"))?;
    Ok(Lowered {
        ir,
        atoms,
        positive,
        negative,
    })
}

impl Ir {
    /// The negation of a state formula that `reachable` interned.
    pub(super) fn negation(&self, id: NodeId) -> Result<NodeId, AutomatonError> {
        let found = match self.node(id)? {
            Node::Not(inner) => Some(inner),
            Node::Constant(value) => self.interned.get(&Node::Constant(!value)).copied(),
            _ => self.interned.get(&Node::Not(id)).copied(),
        };
        found.ok_or_else(|| invariant("negation-interned"))
    }
}

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

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::bound::TemporalUpper;
use qsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, FormulaNode, FormulaNodeId, UnaryTemporal,
};

use crate::automaton::error::AutomatonError;
use crate::automaton::key_state::{Occurrence, PastOperator, Polarity};
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
    /// The offsets of a step interval. A timed interval counts time, not
    /// positions, and has no offsets: the timed evaluator reads it
    /// (`temporal::timed`), and a clause holding one is refused before a
    /// step reader sees it.
    fn of(interval: Option<&CheckedInterval>) -> Result<Self, AutomatonError> {
        Ok(match interval {
            None => Self::From { lower: 0 },
            Some(CheckedInterval::Steps(interval)) => match interval.upper() {
                TemporalUpper::Finite(upper) => Self::Closed {
                    lower: interval.lower(),
                    upper,
                },
                TemporalUpper::Open => Self::From {
                    lower: interval.lower(),
                },
            },
            Some(CheckedInterval::Timed(_)) => {
                return Err(AutomatonError::Invariant {
                    invariant: "step-interval-has-offsets",
                })
            }
        })
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
    /// For an arena in normal form (ADR-023 CS-2), the flattened operands of
    /// each node that is an `and` or `or`, and none for any other. The
    /// operands of one are a set: sorted by node, without repeats, folded to
    /// the right, so an `and` or `or` is the same node however it was
    /// written, and an exchange of atoms maps the arena onto itself.
    operands: Option<Vec<Vec<NodeId>>>,
}

pub(super) fn invariant(invariant: &'static str) -> AutomatonError {
    AutomatonError::Invariant { invariant }
}

impl Ir {
    /// An arena in normal form.
    pub(super) fn normal() -> Self {
        Self {
            operands: Some(Vec::new()),
            ..Self::default()
        }
    }

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
        self.intern_operands(node, future, core, Vec::new())
    }

    /// `intern`, for a node in normal form whose flattened operands are
    /// `operands`.
    fn intern_operands(
        &mut self,
        node: Node,
        future: bool,
        core: Option<NodeId>,
        operands: Vec<NodeId>,
    ) -> NodeId {
        if let Some(&id) = self.interned.get(&node) {
            return id;
        }
        let id = NodeId(self.nodes.len());
        if let Some(held) = &mut self.operands {
            held.push(operands);
        }
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
        if self.operands.is_some() {
            let mut operands = self.operands_of(left, conjunction);
            operands.extend(self.operands_of(right, conjunction));
            return self.fold(operands, conjunction);
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

    /// The flattened operands of `id` read as an `and` (`conjunction`) or an
    /// `or`; `id` itself when it is neither.
    fn operands_of(&self, id: NodeId, conjunction: bool) -> Vec<NodeId> {
        let flat = match self.nodes.get(id.0) {
            Some(Node::And(..)) => conjunction,
            Some(Node::Or(..)) => !conjunction,
            _ => false,
        };
        match self.operands.as_ref().and_then(|held| held.get(id.0)) {
            Some(operands) if flat => operands.clone(),
            _ => vec![id],
        }
    }

    /// The `and` or `or` of `operands` in normal form: sorted by node, without
    /// repeats, folded to the right.
    fn fold(
        &mut self,
        mut operands: Vec<NodeId>,
        conjunction: bool,
    ) -> Result<NodeId, AutomatonError> {
        operands.sort();
        operands.dedup();
        let (mut folded, rest) = operands
            .split_last()
            .map(|(last, rest)| (*last, rest.to_vec()))
            .ok_or_else(|| invariant("connective-has-operands"))?;
        for index in (0..rest.len()).rev() {
            let flattened = operands.get(index..).unwrap_or_default().to_vec();
            let future = flattened.iter().any(|id| self.has_future(*id));
            let node = if conjunction {
                Node::And(rest[index], folded)
            } else {
                Node::Or(rest[index], folded)
            };
            folded = self.intern_operands(node, future, None, flattened);
        }
        Ok(folded)
    }

    /// Where each node of `order`, in ascending order, goes when atom `a`
    /// becomes atom `exchange[a]`. A node's image is built from the images of
    /// its operands, so the image of a state's operators stays in the arena
    /// whenever the exchange fixes the target, which the caller checks.
    ///
    /// # Errors
    ///
    /// An invariant error when `order` is not an arena order.
    pub(super) fn exchanged(
        &mut self,
        order: &[NodeId],
        exchange: &[usize],
    ) -> Result<BTreeMap<NodeId, NodeId>, AutomatonError> {
        let mut images: BTreeMap<NodeId, NodeId> = BTreeMap::new();
        let image = |images: &BTreeMap<NodeId, NodeId>, id: NodeId| {
            images
                .get(&id)
                .copied()
                .ok_or_else(|| invariant("operands-precede-operators"))
        };
        for id in order {
            let node = self.node(*id)?;
            let exchanged = match node {
                Node::Constant(_) => *id,
                Node::Atom(atom) => {
                    let to = exchange.get(atom.index()).copied().unwrap_or(atom.index());
                    self.atom(Atom::new(to)?)
                }
                Node::Not(operand) => {
                    let operand = image(&images, operand)?;
                    self.not(operand)?
                }
                Node::And(..) | Node::Or(..) => {
                    let leaves = self
                        .operands
                        .as_ref()
                        .and_then(|held| held.get(id.0))
                        .cloned()
                        .ok_or_else(|| invariant("normal-arena"))?;
                    let leaves = leaves
                        .into_iter()
                        .map(|leaf| image(&images, leaf))
                        .collect::<Result<Vec<_>, _>>()?;
                    self.fold(leaves, matches!(node, Node::And(..)))?
                }
                Node::Past {
                    kind,
                    window,
                    left,
                    right,
                } => {
                    let left = left.map(|left| image(&images, left)).transpose()?;
                    let right = image(&images, right)?;
                    self.past(kind, window, left, right)?
                }
                Node::Future {
                    kind,
                    window,
                    left,
                    right,
                } => {
                    let left = left.map(|left| image(&images, left)).transpose()?;
                    let right = image(&images, right)?;
                    self.future(kind, window, left, right)
                }
            };
            images.insert(*id, exchanged);
        }
        Ok(images)
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
    /// negation of every state operand of a future operator and of an `or`
    /// that holds a future operator (the translation asserts it when a
    /// counter's operand does not hold, or in the branch of a disjunction that
    /// does not discharge on that operand), in ascending order.
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
            if let (Node::Or(left, right), true) = (node, self.has_future(id)) {
                // The `or` rule asserts the negation of a state operand in
                // the branch that does not discharge on it.
                for operand in [left, right] {
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
    /// Source-occurrence paths for the selected NNF node identities. A node
    /// can have several source occurrences when the checked formula shares a
    /// node at distinct operand paths.
    pub(super) positive_occurrences: BTreeMap<NodeId, Vec<Occurrence>>,
    pub(super) negative_occurrences: BTreeMap<NodeId, Vec<Occurrence>>,
    /// Source past occurrence paths for the NNF recurrence nodes.
    pub(super) positive_past_occurrences: BTreeMap<NodeId, Vec<(Vec<u64>, PastOperator)>>,
    pub(super) negative_past_occurrences: BTreeMap<NodeId, Vec<(Vec<u64>, PastOperator)>>,
    /// Finite future occurrences whose operand contains another interval.
    pub(super) positive_nested_intervals: BTreeSet<NodeId>,
    pub(super) negative_nested_intervals: BTreeSet<NodeId>,
}

/// Lower `formula`: every `holds` atom becomes a letter atom, `implies`
/// becomes `or`, and each node is built with its negation.
pub(super) fn lower(formula: &CheckedFormula, normal: bool) -> Result<Lowered, AutomatonError> {
    let mut ir = if normal { Ir::normal() } else { Ir::default() };
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
            FormulaNode::Aggregate { .. } => {
                return Err(invariant("aggregate-formula-not-emitted"));
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
                let window = Window::of(interval.as_ref())?;
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
                let window = Window::of(interval.as_ref())?;
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
    let (positive_occurrences, positive_past_occurrences) =
        occurrence_nodes(formula, &pairs, &ir, false)?;
    let (negative_occurrences, negative_past_occurrences) =
        occurrence_nodes(formula, &pairs, &ir, true)?;
    let positive_nested_intervals = nested_intervals(formula, &pairs, &ir, false)?;
    let negative_nested_intervals = nested_intervals(formula, &pairs, &ir, true)?;
    Ok(Lowered {
        ir,
        atoms,
        positive,
        negative,
        positive_occurrences,
        negative_occurrences,
        positive_past_occurrences,
        negative_past_occurrences,
        positive_nested_intervals,
        negative_nested_intervals,
    })
}

/// The lower-bound-zero NNF cores of finite future occurrences whose
/// source operand subtree contains another interval.
fn nested_intervals(
    formula: &CheckedFormula,
    pairs: &[Option<(NodeId, NodeId)>],
    ir: &Ir,
    root_negative: bool,
) -> Result<BTreeSet<NodeId>, AutomatonError> {
    let mut contains_interval = vec![false; formula.nodes().len()];
    let mut nested = vec![false; formula.nodes().len()];
    for (index, source) in formula.nodes().iter().enumerate().rev() {
        let child_has_interval = source.operands().any(|child| {
            contains_interval
                .get(child.index())
                .copied()
                .unwrap_or(false)
        });
        let own_interval = matches!(
            source,
            FormulaNode::Unary {
                interval: Some(_),
                ..
            } | FormulaNode::Binary {
                interval: Some(_),
                ..
            }
        );
        if let Some(slot) = nested.get_mut(index) {
            *slot = child_has_interval;
        }
        if let Some(slot) = contains_interval.get_mut(index) {
            *slot = child_has_interval || own_interval;
        }
    }
    let mut cores = BTreeSet::new();
    for (index, source) in formula.nodes().iter().enumerate() {
        let finite_future = matches!(
            source,
            FormulaNode::Unary {
                operator: UnaryTemporal::Eventually | UnaryTemporal::Always,
                interval: Some(_),
                ..
            } | FormulaNode::Binary {
                operator: BinaryTemporal::Until | BinaryTemporal::Release,
                interval: Some(_),
                ..
            }
        );
        if !finite_future || !nested.get(index).copied().unwrap_or(false) {
            continue;
        }
        let (positive, negative) = pairs
            .get(index)
            .copied()
            .flatten()
            .ok_or_else(|| invariant("nested-interval-has-pair"))?;
        let selected = if root_negative { negative } else { positive };
        cores.insert(ir.core(selected));
    }
    Ok(cores)
}

/// Resolve each occurrence path in the checked formula to the node its
/// polarity selects in the NNF arena. The path, not the arena's materialized
/// node index, is the stable identity used by FR-338.
fn occurrence_nodes(
    formula: &CheckedFormula,
    pairs: &[Option<(NodeId, NodeId)>],
    ir: &Ir,
    root_negative: bool,
) -> Result<
    (
        BTreeMap<NodeId, Vec<Occurrence>>,
        BTreeMap<NodeId, Vec<(Vec<u64>, PastOperator)>>,
    ),
    AutomatonError,
> {
    struct Entry {
        path: Vec<u64>,
        node: usize,
        negative: bool,
    }

    let mut pending = vec![Entry {
        path: Vec::new(),
        node: formula.root().index(),
        negative: root_negative,
    }];
    let mut origins: BTreeMap<NodeId, Vec<Occurrence>> = BTreeMap::new();
    let mut past_origins: BTreeMap<NodeId, Vec<(Vec<u64>, PastOperator)>> = BTreeMap::new();
    let mut next = 0;
    while next < pending.len() {
        let entry = pending
            .get(next)
            .ok_or_else(|| invariant("formula-occurrence-entry"))?;
        let source = formula
            .nodes()
            .get(entry.node)
            .ok_or_else(|| invariant("formula-occurrence-node"))?;
        let path = entry.path.clone();
        let source_index = entry.node;
        let is_negative = entry.negative;
        let (positive, negative_node) = pairs
            .get(source_index)
            .copied()
            .flatten()
            .ok_or_else(|| invariant("formula-occurrence-pair"))?;
        let selected = if is_negative { negative_node } else { positive };
        let occurrence = Occurrence::new(
            path.clone(),
            if is_negative {
                Polarity::Negative
            } else {
                Polarity::Positive
            },
        );
        origins
            .entry(selected)
            .or_default()
            .push(occurrence.clone());
        let core = ir.core(selected);
        if core != selected {
            origins.entry(core).or_default().push(occurrence);
        }

        let source_operator = match source {
            FormulaNode::Unary { operator, .. } => match operator {
                UnaryTemporal::Once => Some(PastOperator::Once),
                UnaryTemporal::Historically => Some(PastOperator::Historically),
                UnaryTemporal::Eventually | UnaryTemporal::Always => None,
            },
            FormulaNode::Binary { operator, .. } => match operator {
                BinaryTemporal::Since => Some(PastOperator::Since),
                BinaryTemporal::Triggered => Some(PastOperator::Triggered),
                BinaryTemporal::Until | BinaryTemporal::Release => None,
            },
            _ => None,
        };
        if let Some(operator) = source_operator {
            let mut recurrence = selected;
            loop {
                match ir.node(recurrence)? {
                    Node::Not(child) => recurrence = child,
                    Node::Past { .. } => break,
                    _ => return Err(invariant("past-occurrence-has-recurrence")),
                }
            }
            past_origins
                .entry(recurrence)
                .or_default()
                .push((path.clone(), operator));
        }

        for (position, child) in source.operands().enumerate() {
            let flips = matches!(source, FormulaNode::Not(_))
                || (position == 0 && matches!(source, FormulaNode::Implies(..)));
            let mut child_path = path.clone();
            child_path.push(u64::try_from(position).map_err(|_| invariant("operand-position"))?);
            pending.push(Entry {
                path: child_path,
                node: child.index(),
                negative: is_negative ^ flips,
            });
        }
        next += 1;
    }
    for values in origins.values_mut() {
        values.sort_by(|left, right| {
            left.path
                .cmp(&right.path)
                .then_with(|| match (left.polarity, right.polarity) {
                    (Polarity::Positive, Polarity::Negative) => std::cmp::Ordering::Less,
                    (Polarity::Negative, Polarity::Positive) => std::cmp::Ordering::Greater,
                    _ => std::cmp::Ordering::Equal,
                })
        });
        values.dedup();
    }
    for values in past_origins.values_mut() {
        values.sort_by(|left, right| left.0.cmp(&right.0));
        values.dedup();
    }
    Ok((origins, past_origins))
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

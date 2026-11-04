// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-523 "Counters": what the clause itself costs, read from its formula
//! without recursion (ADR-030 D-1, D-4.2).
//!
//! The formula is read as a graph of [`NodeShape`]s, so a node that two
//! parents share is one node. [`ClauseWork::of`] adapts a
//! [`CheckedFormula`]; a caller that holds the formula as nodes of a checked
//! package (FR-515 `formula()`, where equal terms share one node, FR-337)
//! builds its [`NodeShape`]s and calls [`ClauseWork::of_shapes`].

use qsl_foundation::bound::TemporalUpper;
use qsl_package::checked_views::{FormulaNodeUse, OperatorDirection};
use qsl_package::{CheckedIntervalEnd, CheckedOperationInterval, CheckedOperationRational};
use qsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, FormulaNode, UnaryTemporal,
};

/// Why a formula's reach cannot be measured in ticks.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReachFault {
    /// The sum of interval upper bounds along a nesting path exceeds `u64`.
    #[error("the formula's reach exceeds u64 ticks")]
    Overflow,
    /// An operator has no finite upper bound, which only the infinite-trace
    /// profile admits; a temporal evidence call reads a bounded profile.
    #[error("an operator has no finite upper bound")]
    Unbounded,
    /// An operator carries a timed interval that is not closed and integral,
    /// which counts time and not ticks of a trace clock.
    #[error("an operator carries a timed interval")]
    Timed,
    /// An operand names no node, or the formula contains a cycle.
    #[error("the formula is not an acyclic graph of its own nodes")]
    Malformed,
}

/// The upper bound one node adds to a nesting path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NodeReach {
    /// A connective, a constant or an atom: it adds nothing.
    None,
    /// A future operator (`eventually`, `always`, `until`, `release`) with
    /// this finite upper bound.
    Future(u64),
    /// A past operator (`once`, `historically`, `since`, `triggered`) with
    /// this finite upper bound: it counts toward the history span and not
    /// toward `Hf`.
    Past(u64),
}

/// One node of a formula as the work counters read it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeShape {
    /// The indices of the node's operands.
    pub operands: [Option<usize>; 2],
    /// What the node adds to a nesting path.
    pub reach: NodeReach,
}

/// The work counters a clause's formula fixes, before any instance is
/// assessed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClauseWork {
    /// The distinct nodes reachable from the root: a node two parents share
    /// counts once.
    pub formula_nodes: u64,
    /// The greatest sum of the upper bounds of the operators along any
    /// nesting path, past operators included (FR-326 horizon).
    pub history_span: u64,
    /// `Hf` of FR-501: the same sum over the future operators only
    /// (`eventually`, `always`, `until`, `release`).
    pub future_reach: u64,
}

impl ClauseWork {
    /// Measure `formula`.
    ///
    /// # Errors
    ///
    /// [`ReachFault`] when an interval has no finite tick upper bound, a
    /// reach sum exceeds `u64`, or the arena is malformed.
    pub fn of(formula: &CheckedFormula) -> Result<Self, ReachFault> {
        let shapes = formula
            .nodes()
            .iter()
            .map(shape)
            .collect::<Result<Vec<_>, _>>()?;
        Self::of_shapes(&shapes, formula.root().index())
    }

    /// Measure the formula whose nodes are `shapes`, rooted at `root`.
    ///
    /// # Errors
    ///
    /// [`ReachFault::Overflow`] when a reach sum exceeds `u64` and
    /// [`ReachFault::Malformed`] for an operand or root outside `shapes` or
    /// a cycle.
    pub fn of_shapes(shapes: &[NodeShape], root: usize) -> Result<Self, ReachFault> {
        #[derive(Clone, Copy)]
        enum Mark {
            Unvisited,
            Open,
            Done { span: u64, future: u64 },
        }
        let mut marks = vec![Mark::Unvisited; shapes.len()];
        let mut stack = vec![(root, 0_usize)];
        let mut nodes = 0_u64;
        while let Some((index, next)) = stack.pop() {
            let node = shapes.get(index).ok_or(ReachFault::Malformed)?;
            if next == 0 {
                match marks.get(index).ok_or(ReachFault::Malformed)? {
                    Mark::Done { .. } => continue,
                    Mark::Open => return Err(ReachFault::Malformed),
                    Mark::Unvisited => {}
                }
                *marks.get_mut(index).ok_or(ReachFault::Malformed)? = Mark::Open;
                nodes += 1;
            }
            let operand = node.operands.get(next).copied();
            match operand {
                Some(Some(operand)) => {
                    stack.push((index, next + 1));
                    stack.push((operand, 0));
                }
                Some(None) => stack.push((index, next + 1)),
                None => {
                    let (mut below_span, mut below_future) = (0_u64, 0_u64);
                    for operand in node.operands.iter().flatten() {
                        if let Some(Mark::Done { span, future }) = marks.get(*operand) {
                            below_span = below_span.max(*span);
                            below_future = below_future.max(*future);
                        }
                    }
                    let (own, is_future) = match node.reach {
                        NodeReach::None => (0, false),
                        NodeReach::Future(upper) => (upper, true),
                        NodeReach::Past(upper) => (upper, false),
                    };
                    let span = below_span.checked_add(own).ok_or(ReachFault::Overflow)?;
                    let future = if is_future {
                        below_future.checked_add(own).ok_or(ReachFault::Overflow)?
                    } else {
                        below_future
                    };
                    *marks.get_mut(index).ok_or(ReachFault::Malformed)? =
                        Mark::Done { span, future };
                }
            }
        }
        match marks.get(root) {
            Some(Mark::Done { span, future }) => Ok(Self {
                formula_nodes: nodes,
                history_span: *span,
                future_reach: *future,
            }),
            Some(Mark::Unvisited | Mark::Open) | None => Err(ReachFault::Malformed),
        }
    }
}

/// The shape of one node of a checked formula.
fn shape(node: &FormulaNode) -> Result<NodeShape, ReachFault> {
    let mut operands = node.operands();
    let operands = [
        operands.next().map(|operand| operand.index()),
        operands.next().map(|operand| operand.index()),
    ];
    let (interval, is_future) = match node {
        FormulaNode::Constant(_)
        | FormulaNode::Holds { .. }
        | FormulaNode::Aggregate { .. }
        | FormulaNode::Not(_)
        | FormulaNode::And(..)
        | FormulaNode::Or(..)
        | FormulaNode::Implies(..) => {
            return Ok(NodeShape {
                operands,
                reach: NodeReach::None,
            });
        }
        FormulaNode::Unary {
            operator, interval, ..
        } => (
            interval,
            matches!(operator, UnaryTemporal::Eventually | UnaryTemporal::Always),
        ),
        FormulaNode::Binary {
            operator, interval, ..
        } => (
            interval,
            matches!(operator, BinaryTemporal::Until | BinaryTemporal::Release),
        ),
    };
    let upper = match interval {
        None => return Err(ReachFault::Unbounded),
        // A finite-window clause measures ticks of its clock unit, both ends
        // inclusive (FR-501): closed, integral bounds are a tick count (the predicate of the planner's
        // `Window::of`).
        // Any other timed interval counts rational time and has no tick
        // reach.
        Some(CheckedInterval::Timed(timed)) => {
            if !timed.lower_closed()
                || !timed.upper_closed()
                || !timed.lower().is_integer()
                || !timed.upper().is_integer()
            {
                return Err(ReachFault::Timed);
            }
            timed
                .upper()
                .numerator()
                .to_u64()
                .ok_or(ReachFault::Overflow)?
        }
        Some(CheckedInterval::Steps(steps)) => match steps.upper() {
            TemporalUpper::Finite(upper) => upper,
            TemporalUpper::Open => return Err(ReachFault::Unbounded),
        },
    };
    Ok(NodeShape {
        operands,
        reach: if is_future {
            NodeReach::Future(upper)
        } else {
            NodeReach::Past(upper)
        },
    })
}

/// The formula of a clause as the evidence call measures it: one
/// [`NodeShape`] per node, the first node the root.
///
/// A clause of a checked package builds it from the subject view's
/// `formula_graph()` entries and their `operands` edges (FR-515) with
/// [`Self::of_graph`], one construction for every route to a package, so
/// that the package built from an `Emission` and the one read back from the
/// emitted bytes give the evaluator the same formula by construction.
/// [`Self::of_checked`] reads an in-process checked formula, for a caller
/// that holds no package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseFormula {
    shapes: Result<Vec<NodeShape>, ReachFault>,
    root: usize,
}

impl ClauseFormula {
    /// The formula of `formula`, rooted at its root node.
    pub fn of_checked(formula: &CheckedFormula) -> Self {
        Self {
            shapes: formula.nodes().iter().map(shape).collect(),
            root: formula.root().index(),
        }
    }

    /// The formula whose nodes are `graph`, the first node the root: each
    /// operand position names an entry of `graph`.
    pub fn of_graph(graph: &[FormulaNodeUse<'_>]) -> Self {
        Self {
            shapes: graph.iter().map(graph_shape).collect(),
            root: 0,
        }
    }

    /// The shape of each node.
    ///
    /// # Errors
    ///
    /// [`ReachFault::Unbounded`], [`ReachFault::Timed`] or
    /// [`ReachFault::Overflow`] for the first operator whose interval has no
    /// finite tick upper bound, and [`ReachFault::Malformed`] for a node with
    /// more than two operands.
    pub fn shapes(&self) -> Result<&[NodeShape], ReachFault> {
        self.shapes.as_deref().map_err(|fault| *fault)
    }

    /// The work counters the formula fixes.
    ///
    /// # Errors
    ///
    /// [`ReachFault`] as [`ClauseWork::of_shapes`] and [`Self::shapes`] state.
    pub fn work(&self) -> Result<ClauseWork, ReachFault> {
        ClauseWork::of_shapes(self.shapes()?, self.root)
    }
}

/// The shape of one `formula_graph()` entry.
fn graph_shape(node: &FormulaNodeUse<'_>) -> Result<NodeShape, ReachFault> {
    let mut operands = [None, None];
    if node.operands.len() > operands.len() {
        return Err(ReachFault::Malformed);
    }
    for (slot, operand) in operands.iter_mut().zip(&node.operands) {
        *slot = Some(*operand);
    }
    let Some(direction) = node.direction() else {
        return Ok(NodeShape {
            operands,
            reach: NodeReach::None,
        });
    };
    let upper = match &node.interval {
        None => return Err(ReachFault::Unbounded),
        Some(CheckedOperationInterval::Integer { upper: None, .. })
        | Some(CheckedOperationInterval::Timed { upper: None, .. }) => {
            return Err(ReachFault::Unbounded)
        }
        Some(CheckedOperationInterval::Integer {
            upper: Some(upper), ..
        }) => u64::try_from(upper).map_err(|_| ReachFault::Overflow)?,
        // The same predicate `shape` applies to an in-process timed
        // interval: closed ends and integral bounds count ticks.
        Some(CheckedOperationInterval::Timed {
            lower,
            upper: Some(upper),
            lower_end,
            upper_end,
        }) => {
            let closed = |end: &CheckedIntervalEnd| *end == CheckedIntervalEnd::Closed;
            let integral = |bound: &CheckedOperationRational| {
                u64::try_from(&bound.denominator).is_ok_and(|denominator| denominator == 1)
            };
            if !closed(lower_end) || !closed(upper_end) || !integral(lower) || !integral(upper) {
                return Err(ReachFault::Timed);
            }
            u64::try_from(&upper.numerator).map_err(|_| ReachFault::Overflow)?
        }
    };
    Ok(NodeShape {
        operands,
        reach: match direction {
            OperatorDirection::Future => NodeReach::Future(upper),
            OperatorDirection::Past => NodeReach::Past(upper),
        },
    })
}

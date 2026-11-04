// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-532 "One clause value, from the bytes": the facts the finite-window
//! evaluator reads about a clause, as one value.
//!
//! A [`ClauseFacts`] holds the clause's activation, evaluator profile, clock
//! name, captures, name and formula. A checked package builds it from its own
//! nodes and rows (`formula_graph()`, `temporal_subject`, the lock's
//! `profile_selections`, the `temporal_bindings` and `temporal_clause_names`
//! rows) on every route to a package, so the evaluator reads no source
//! clause. [`ClauseFacts::of_checked`] builds the same value from an
//! in-process [`CheckedTemporalClause`], for the callers that evaluate a
//! checked clause without a package.
//!
//! The formula is a [`ClauseFormula`]: the distinct nodes of the clause in
//! an order that lists an operator before its operands, the shape of each for
//! the work counters (FR-523), the evaluator's plan, and the atom index of
//! each atom node (FR-532 "Atom index").

use std::collections::BTreeMap;

use qsl_package::checked_views::{FormulaNodeUse, OperatorDirection};
use qsl_package::{CheckedIntervalEnd, CheckedOperationInterval, CheckedOperationRational};
use qsl_semantics::check::{
    Activation, CaptureSite, CheckedFormula, CheckedTemporalClause, FormulaNode, FormulaNodeId,
    TemporalProfile,
};
use quire_exact::ValueType;
use quire_semantic_value::catalog::InternalFault;

use super::evidence::ClauseWork;
use super::evidence::{shape, NodeReach, NodeShape, ReachFault};
use super::plan::{Base, Node, Plan, Scan, Window};

/// The operation identities of the atoms of a formula (FR-337): a `holds`
/// application and a window aggregate.
const HOLDS: &str = "quire.op.temporal.holds";
const AGGREGATE: &str = "quire.op.temporal.aggregate";

/// The facts of one clause the evaluator reads (FR-532).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseFacts {
    name: Box<str>,
    activation: Activation,
    profile: TemporalProfile,
    clock: Box<str>,
    captures: Vec<(Box<str>, ValueType)>,
    formula: ClauseFormula,
}

impl ClauseFacts {
    /// The facts of a clause a checked package carries: the values its rows
    /// and nodes give.
    pub fn new(
        name: impl Into<Box<str>>,
        activation: Activation,
        profile: TemporalProfile,
        clock: impl Into<Box<str>>,
        captures: Vec<(Box<str>, ValueType)>,
        formula: ClauseFormula,
    ) -> Self {
        Self {
            name: name.into(),
            activation,
            profile,
            clock: clock.into(),
            captures,
            formula,
        }
    }

    /// The facts of an in-process checked clause, for a caller that holds no
    /// package. The atom index of a node is its arena index.
    pub fn of_checked(clause: &CheckedTemporalClause) -> Self {
        Self {
            name: clause.name().into(),
            activation: clause.activation(),
            profile: clause.profile(),
            clock: clause.clock().into(),
            captures: clause
                .captures()
                .map(|(name, declared)| (name.into(), declared.clone()))
                .collect(),
            formula: ClauseFormula::of_checked(clause.formula()),
        }
    }

    /// The facts with `formula` as the clause's formula.
    #[must_use]
    pub fn with_formula(mut self, formula: ClauseFormula) -> Self {
        self.formula = formula;
        self
    }

    /// The clause's declared name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// How the clause activates.
    pub const fn activation(&self) -> Activation {
        self.activation
    }

    /// The evaluator profile the clause is checked under.
    pub const fn profile(&self) -> TemporalProfile {
        self.profile
    }

    /// The clock binding name.
    pub fn clock(&self) -> &str {
        &self.clock
    }

    /// The captures in authored order, each with its declared value type.
    pub fn captures(&self) -> impl ExactSizeIterator<Item = (&str, &ValueType)> {
        self.captures.iter().map(|(name, ty)| (&**name, ty))
    }

    /// Where the clause fixes its captures.
    pub fn capture_site(&self) -> CaptureSite {
        if self.captures.is_empty() {
            CaptureSite::Never
        } else if matches!(self.activation, Activation::Origin) {
            CaptureSite::Origin
        } else {
            CaptureSite::Activations
        }
    }

    /// The clause's formula.
    pub fn formula(&self) -> &ClauseFormula {
        &self.formula
    }
}

/// The formula of a clause as the evidence call reads it: the distinct nodes
/// in an order in which an operator precedes its operands, the first node the
/// root.
///
/// A clause of a checked package builds it from the subject view's
/// `formula_graph()` entries and their `operands` edges (FR-515) with
/// [`Self::of_graph`], one construction for every route to a package.
/// [`Self::of_checked`] reads an in-process checked formula.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseFormula {
    shapes: Result<Vec<NodeShape>, ReachFault>,
    plan: Result<Plan, InternalFault>,
    ids: Vec<FormulaNodeId>,
    /// The atom index of each node, `None` for a node that is no atom.
    atom_of: Vec<Option<u64>>,
    root: usize,
}

impl ClauseFormula {
    /// The formula of `formula`, rooted at its root node; the atom index of a
    /// node is its arena index.
    pub fn of_checked(formula: &CheckedFormula) -> Self {
        let ids = (0..formula.nodes().len())
            .filter_map(|index| formula.node_id(index))
            .collect();
        let atom_of = formula
            .nodes()
            .iter()
            .enumerate()
            .map(|(index, node)| {
                matches!(
                    node,
                    FormulaNode::Holds { .. } | FormulaNode::Aggregate { .. }
                )
                .then(|| u64::try_from(index).ok())
                .flatten()
            })
            .collect();
        Self {
            shapes: formula.nodes().iter().map(shape).collect(),
            plan: Plan::new(formula),
            ids,
            atom_of,
            root: formula.root().index(),
        }
    }

    /// The formula whose entries are `graph`, the first entry the root: each
    /// operand position names an entry of `graph`. Entries with one `node`
    /// are uses of one formula node and are one node of the formula; the
    /// atom index of an atom node is its number among the distinct atom
    /// nodes in the order of their first use.
    pub fn of_graph(graph: &[FormulaNodeUse<'_>]) -> Self {
        match Built::of(graph) {
            Ok(built) => built.formula(graph),
            Err(fault) => Self {
                shapes: Err(fault),
                plan: Err(InternalFault::new(
                    "temporal.facts",
                    "formula-graph-unreadable",
                )),
                ids: Vec::new(),
                atom_of: Vec::new(),
                root: 0,
            },
        }
    }

    /// The shape of each node.
    ///
    /// # Errors
    ///
    /// [`ReachFault::Unbounded`], [`ReachFault::Timed`] or
    /// [`ReachFault::Overflow`] for the first operator whose interval has no
    /// finite tick upper bound, and [`ReachFault::Malformed`] for a node with
    /// more than two operands, an operand that names no entry or a cycle.
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

    /// The atom index of the node at arena index `node`, `None` for a node
    /// that is no atom.
    pub fn atom_index(&self, node: usize) -> Option<u64> {
        self.atom_of.get(node).copied().flatten()
    }

    /// The number of atoms.
    pub fn atom_count(&self) -> usize {
        self.atom_of.iter().flatten().count()
    }

    pub(super) fn plan(&self) -> Result<&Plan, InternalFault> {
        self.plan.as_ref().map_err(Clone::clone)
    }

    pub(super) fn node_id(&self, index: usize) -> Option<FormulaNodeId> {
        self.ids.get(index).copied()
    }
}

/// The atom index (FR-532 "Atom index") of each entry of `graph`: the number
/// of the entry's node among the distinct atom nodes (`holds` and window
/// aggregate), counted from 0 in the order of first use in pre-order, and
/// `None` for an entry that is no atom.
pub fn atom_numbers(graph: &[FormulaNodeUse<'_>]) -> Vec<Option<u64>> {
    let mut assigned: BTreeMap<&qsl_package::CheckedNodeId, u64> = BTreeMap::new();
    graph
        .iter()
        .map(|entry| {
            if !matches!(entry.operation, HOLDS | AGGREGATE) {
                return None;
            }
            let next = u64::try_from(assigned.len()).ok()?;
            Some(*assigned.entry(entry.node).or_insert(next))
        })
        .collect()
}

/// The distinct nodes of a `formula_graph()` in plan order.
struct Built {
    /// For each node of the plan order, the first position of the graph that
    /// used it.
    order: Vec<usize>,
    /// For each graph position, the position of the first use of its node.
    first: Vec<usize>,
}

impl Built {
    fn of(graph: &[FormulaNodeUse<'_>]) -> Result<Self, ReachFault> {
        let mut seen: BTreeMap<&qsl_package::CheckedNodeId, usize> = BTreeMap::new();
        let first: Vec<usize> = graph
            .iter()
            .enumerate()
            .map(|(position, entry)| *seen.entry(entry.node).or_insert(position))
            .collect();
        // Each distinct node's operands, as positions of first uses.
        let operands = |position: usize| -> Result<Vec<usize>, ReachFault> {
            let entry = graph.get(position).ok_or(ReachFault::Malformed)?;
            if entry.operands.len() > 2 {
                return Err(ReachFault::Malformed);
            }
            entry
                .operands
                .iter()
                .map(|operand| first.get(*operand).copied().ok_or(ReachFault::Malformed))
                .collect()
        };
        // Reverse post-order of an iterative depth-first walk lists every
        // operator before its operands and finds a cycle.
        #[derive(Clone, Copy, PartialEq)]
        enum Mark {
            Open,
            Done,
        }
        if graph.is_empty() {
            return Err(ReachFault::Malformed);
        }
        let mut marks: BTreeMap<usize, Mark> = BTreeMap::new();
        let mut post = Vec::new();
        let mut stack: Vec<(usize, Vec<usize>, usize)> = vec![(0, operands(0)?, 0)];
        marks.insert(0, Mark::Open);
        while let Some((node, below, next)) = stack.pop() {
            match below.get(next) {
                Some(&operand) => {
                    stack.push((node, below, next + 1));
                    match marks.get(&operand) {
                        Some(Mark::Open) => return Err(ReachFault::Malformed),
                        Some(Mark::Done) => {}
                        None => {
                            marks.insert(operand, Mark::Open);
                            stack.push((operand, operands(operand)?, 0));
                        }
                    }
                }
                None => {
                    marks.insert(node, Mark::Done);
                    post.push(node);
                }
            }
        }
        post.reverse();
        Ok(Self { order: post, first })
    }

    fn formula(&self, graph: &[FormulaNodeUse<'_>]) -> ClauseFormula {
        let mut index_of: BTreeMap<usize, usize> = BTreeMap::new();
        for (arena, position) in self.order.iter().enumerate() {
            index_of.insert(*position, arena);
        }
        let operands_of = |position: usize| -> Vec<usize> {
            graph
                .get(position)
                .map(|entry| {
                    entry
                        .operands
                        .iter()
                        .filter_map(|operand| {
                            self.first
                                .get(*operand)
                                .and_then(|first| index_of.get(first))
                                .copied()
                        })
                        .collect()
                })
                .unwrap_or_default()
        };
        let shapes = self
            .order
            .iter()
            .map(|position| {
                graph
                    .get(*position)
                    .ok_or(ReachFault::Malformed)
                    .and_then(|entry| {
                        let mut operands = operands_of(*position).into_iter();
                        graph_shape(entry, [operands.next(), operands.next()])
                    })
            })
            .collect();
        let numbers = atom_numbers(graph);
        let mut atom_of = vec![None; self.order.len()];
        let mut by_number: Vec<(u64, usize)> = Vec::new();
        for (arena, position) in self.order.iter().enumerate() {
            if let (Some(number), Some(slot)) = (
                numbers.get(*position).copied().flatten(),
                atom_of.get_mut(arena),
            ) {
                *slot = Some(number);
                by_number.push((number, arena));
            }
        }
        by_number.sort_unstable();
        let atoms: Vec<usize> = by_number.into_iter().map(|(_, arena)| arena).collect();
        let plan = self
            .order
            .iter()
            .map(|position| {
                graph
                    .get(*position)
                    .ok_or_else(|| InternalFault::new("temporal.facts", "entry-in-graph"))
                    .and_then(|entry| plan_node(entry, &operands_of(*position)))
            })
            .collect::<Result<Vec<_>, _>>()
            .and_then(|nodes| Plan::from_nodes(nodes, 0, atoms));
        let ids = (0..self.order.len())
            .map(FormulaNodeId::from_arena_index)
            .collect();
        ClauseFormula {
            shapes,
            plan,
            ids,
            atom_of,
            root: 0,
        }
    }
}

/// The shape of one `formula_graph()` entry whose operands sit at `operands`.
fn graph_shape(
    node: &FormulaNodeUse<'_>,
    operands: [Option<usize>; 2],
) -> Result<NodeShape, ReachFault> {
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

/// The evaluator's plan node of one entry whose operands sit at `operands`.
fn plan_node(entry: &FormulaNodeUse<'_>, operands: &[usize]) -> Result<Node, InternalFault> {
    let fault = |invariant: &'static str| InternalFault::new("temporal.facts", invariant);
    let one = || {
        operands
            .first()
            .copied()
            .ok_or_else(|| fault("operator-has-an-operand"))
    };
    let two = || match operands {
        [left, right] => Ok((*left, *right)),
        _ => Err(fault("binary-operator-has-two-operands")),
    };
    let window = || window_of(entry.interval.as_ref());
    let unary = |base: Base, dual: bool| -> Result<Node, InternalFault> {
        Ok(Node::Scan(Scan {
            base,
            dual,
            window: window()?,
            guard: None,
            witness: one()?,
        }))
    };
    let binary = |base: Base, dual: bool| -> Result<Node, InternalFault> {
        let (left, right) = two()?;
        Ok(Node::Scan(Scan {
            base,
            dual,
            window: window()?,
            guard: Some(left),
            witness: right,
        }))
    };
    match entry.operation {
        "quire.op.temporal.true" => Ok(Node::Constant(true)),
        "quire.op.temporal.false" => Ok(Node::Constant(false)),
        HOLDS => Ok(Node::Atom),
        "quire.op.temporal.not" => Ok(Node::Not(one()?)),
        "quire.op.temporal.and" => two().map(|(left, right)| Node::And(left, right)),
        "quire.op.temporal.or" => two().map(|(left, right)| Node::Or(left, right)),
        "quire.op.temporal.implies" => two().map(|(left, right)| Node::Implies(left, right)),
        "quire.op.temporal.eventually" => unary(Base::Eventually, false),
        "quire.op.temporal.always" => unary(Base::Eventually, true),
        "quire.op.temporal.once" => unary(Base::Once, false),
        "quire.op.temporal.historically" => unary(Base::Once, true),
        "quire.op.temporal.until" => binary(Base::Until, false),
        "quire.op.temporal.release" => binary(Base::Until, true),
        "quire.op.temporal.since" => binary(Base::Since, false),
        "quire.op.temporal.triggered" => binary(Base::Since, true),
        // A window aggregate atom is gated on its emission and admission
        // (FR-532 "Window aggregates"): no admitted package carries one.
        AGGREGATE => Err(fault("aggregate-formula-not-emitted")),
        _ => Err(fault("formula-operation-is-temporal")),
    }
}

/// The offsets of a package interval, as `Window::of` reads a checked one.
fn window_of(interval: Option<&CheckedOperationInterval>) -> Result<Window, InternalFault> {
    let fault = |invariant: &'static str| InternalFault::new("temporal.facts", invariant);
    fn tick<T>(value: &T) -> Result<u64, InternalFault>
    where
        for<'a> u64: TryFrom<&'a T>,
    {
        u64::try_from(value)
            .map_err(|_| InternalFault::new("temporal.facts", "interval-bound-in-u64"))
    }
    Ok(match interval {
        None => Window::From { lower: 0 },
        Some(CheckedOperationInterval::Integer { lower, upper }) => match upper {
            Some(upper) => Window::Closed {
                lower: tick(lower)?,
                upper: tick(upper)?,
            },
            None => Window::From {
                lower: tick(lower)?,
            },
        },
        Some(CheckedOperationInterval::Timed {
            lower,
            upper,
            lower_end,
            upper_end,
        }) => {
            let integral = |bound: &CheckedOperationRational| {
                u64::try_from(&bound.denominator).is_ok_and(|denominator| denominator == 1)
            };
            match upper {
                Some(upper)
                    if *lower_end == CheckedIntervalEnd::Closed
                        && *upper_end == CheckedIntervalEnd::Closed
                        && integral(lower)
                        && integral(upper) =>
                {
                    Window::Closed {
                        lower: tick(&lower.numerator)?,
                        upper: tick(&upper.numerator)?,
                    }
                }
                _ => return Err(fault("step-interval-has-offsets")),
            }
        }
    })
}

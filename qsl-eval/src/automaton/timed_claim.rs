// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-242's claim automaton for a timed-profile clause with intervals that
//! is not a flat TT-2 window (TT-3, TT-4, and a TT-2 window that nests one
//! interval operator under another): the timed Büchi automaton of the
//! clause's negation, read over positions, whose clocks join the zone of
//! EN-6's liveness product (ADR-026 EZ-9).
//!
//! The negation, with the clause's activation, is put in negation normal
//! form over `until` and `release` (`eventually_I φ` is `true until_I φ`,
//! `always_I φ` is `false release_I φ`). A state of the automaton is the
//! set of `until` and `release` obligations pending from earlier positions,
//! the memory of every past operator, and, per unbounded `until`, whether
//! the last position fulfilled it. Each obligation with an interval owns
//! one clock, reset where the obligation starts, so its value at a position
//! is the stamp difference the reference evaluator reads (FR-231). Reading
//! a position expands the formulas it must satisfy, as a tableau does:
//!
//! - `ψ until_I χ` started now holds `χ` now when `0 ∈ I`, or holds `ψ` now
//!   and is pending; pending, at a position whose clock lies in `I` it may
//!   hold `χ` and be discharged, before `I` or in it it may hold `ψ` and
//!   stay pending, and past `I` the run dies;
//! - `ψ release_I χ` holds `χ` at every position whose clock lies in `I`
//!   until a position holds `ψ`, after which it is discharged; past `I` it
//!   is discharged.
//!
//! Every choice is a branch, so the automaton accepts a timed word exactly
//! when the negation holds on it. An `until` with no interval has an
//! acceptance set (FR-240); an interval's deadline needs none, since a run
//! of EN-6's product takes infinitely many positions and lets time diverge.
//!
//! **Copies, bounded by variability (MightyL).** A clock per pending
//! instance would need unboundedly many clocks; the construction keeps a
//! number bounded by the interval's variability, as MightyL bounds the
//! copies of each location of its one-clock alternating automaton
//! (Brihaye, Geeraerts, Ho and Monmege, CAV 2017). The negation starts an
//! operator at one position, or at many: under the left operand of an
//! `until` or the right operand of a `release` (each read at many
//! positions), or under an operator itself started at many. Then:
//!
//! - started at one position, an operator has one pending instance at most
//!   and one clock, whatever its interval, punctual included;
//! - started at many with no interval, or one from a closed 0, of two
//!   pending instances one implies the other: the older `until`'s window
//!   ends first and holds the newer's remaining positions, and the newer
//!   `release`'s window covers the older's. One clock suffices; the
//!   construction keeps the older `until` and the newer `release`;
//! - started at many with a non-punctual interval `I` from `a` to `b`
//!   otherwise, instances are grouped into *blocks* of consecutive starts,
//!   each with a clock at its first start `s` and one at its last `e`. A
//!   `release` block holds its right operand wherever `s` has reached `a`
//!   and `e` not passed `b`: the union of its starts' windows, exactly,
//!   since a start joins the newest block only when its window meets that
//!   block's (`e <= b - a`); blocks then begin at least `d = b - a` apart,
//!   and all but the oldest pending one within the last `a`, so at most
//!   `m + 1` are pending, `m` the starts that fit `d` apart in `a`
//!   (`spread`). An `until` block is
//!   discharged at one position read by all its starts, `e` at least `a`
//!   and `s` at most `b`; a start joins the newest block or begins a new
//!   one, a nondeterministic choice. A run that discharges each block at
//!   the latest position its first start can use keeps every other block
//!   `d` apart, all but the oldest within the last `a`: at most `2 m + 1`
//!   pending, and every accepted word has such a run, so the bound loses
//!   none.
//!
//! A punctual interval started at many positions is refused
//! [`AutomatonError::RepeatedPunctualInterval`]: no timed automaton reads
//! such a negation in general (Alur and Dill, 1994), and FR-234 admits
//! punctual intervals in TT-2 windows only.
//!
//! **Past operators** are tracked deterministically as memory. One read at
//! position 0 only reads its single offset 0. Otherwise `since_I`
//! remembers, after the last position its guard failed, the witnesses whose
//! window may still hold: with no interval or one from a closed 0, whether
//! one holds and the clock since the latest; otherwise blocks of witnesses
//! whose windows meet, read as a `release` block's are, at most `m + 1`. A punctual interval below a future operator is
//! refused alike. A past operator over a future operator is refused, as in
//! [`translate`](crate::automaton::translate).
//!
//! EN-6 (`qsl-analyze`) and FR-245's checker (`qsl-replay`) both build this
//! one automaton from the clause, so the checker rebuilds what the search
//! read and never trusts a certificate's automaton (ADR-018 LA-5).

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::bound::TimedInterval;
use qsl_semantics::check::{
    Activation, BinaryTemporal, CheckedFormula, CheckedInterval, CheckedTemporalClause, ClockOp,
    FormulaNode, FormulaNodeId, TemporalProfile, UnaryTemporal,
};
use quire_exact::{Integer, NonZeroInteger, Rational};

use crate::automaton::error::AutomatonError;
use crate::automaton::letter::{Atom, Letter};
use crate::automaton::state::StateId;
use crate::automaton::translate::{translate_position_negation, PropertyAutomaton};
use crate::simulation::model::PlaceConstraint;

/// A formula node of the negation in negation normal form.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Node {
    Const(bool),
    /// An atom of the letter, or its negation.
    Lit {
        atom: usize,
        positive: bool,
    },
    /// A past tracker's value, or its negation.
    Past {
        tracker: usize,
        positive: bool,
    },
    And(usize, usize),
    Or(usize, usize),
    /// `left until_I right`.
    Until {
        left: usize,
        right: usize,
        window: Option<TimedInterval>,
    },
    /// `left release_I right`.
    Release {
        left: usize,
        right: usize,
        window: Option<TimedInterval>,
    },
}

/// A past-only formula, read deterministically from the history.
#[derive(Clone, Debug)]
enum PastExpr {
    Const(bool),
    Lit(usize),
    Tracker(usize),
    Not(Box<PastExpr>),
    And(Box<PastExpr>, Box<PastExpr>),
    Or(Box<PastExpr>, Box<PastExpr>),
}

/// How an operator's pending instances, or a tracker's witnesses, are kept.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Shape {
    /// One at most, with one clock when the operator has an interval.
    Single,
    /// Up to `capacity` blocks, two clocks each: block `k` reads clock
    /// `base + 2k` from its first start and `base + 2k + 1` from its last.
    Blocks(usize),
}

/// `guard since_I witness`.
#[derive(Clone, Debug)]
struct Tracker {
    guard: PastExpr,
    witness: PastExpr,
    window: Option<TimedInterval>,
    /// Read at position 0 only.
    origin: bool,
    shape: Shape,
    /// Its first clock.
    clock: Option<usize>,
}

/// One automaton state.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Config {
    /// Whether position 0 has been read.
    started: bool,
    /// Per tracker: whether a witness holds with its guard ever since.
    seen: Vec<bool>,
    /// The single-instance `until` and `release` nodes with a pending
    /// instance.
    pending: BTreeSet<usize>,
    /// Per block-shaped node with a pending block: its blocks' slots,
    /// oldest first.
    blocks: BTreeMap<usize, Vec<usize>>,
    /// Per block-shaped tracker: its blocks' slots, oldest first.
    witnesses: Vec<Vec<usize>>,
    /// Per unbounded `until`: whether the last position fulfilled it.
    fulfilled: Vec<bool>,
}

/// A transition of a claim automaton: its target, a conjunction of clock
/// constraints over the automaton's clocks (each constraint's place is the
/// automaton clock's index), and the automaton clocks it resets.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ClockedEdge {
    /// The target state.
    pub target: StateId,
    /// The clock guard read at the position.
    pub guard: Vec<PlaceConstraint>,
    /// The automaton clocks reset at the position, ascending.
    pub resets: Vec<usize>,
}

/// FR-242's timed claim automaton of one clause's negation.
#[derive(Debug)]
pub struct TimedClaim {
    nodes: Vec<Node>,
    root: usize,
    trackers: Vec<Tracker>,
    /// Each interval or block-shaped node's shape.
    shapes: BTreeMap<usize, Shape>,
    /// Each node's first clock, by node.
    clocks: BTreeMap<usize, usize>,
    clock_count: usize,
    /// The unbounded `until` nodes, one acceptance set each.
    eventualities: Vec<usize>,
    atoms: Vec<FormulaNodeId>,
    when: Option<Atom>,
    states: Vec<Config>,
    index: BTreeMap<Config, usize>,
    expanded: BTreeMap<(usize, Letter), Vec<ClockedEdge>>,
    max_states: u64,
}

/// The lowering of a checked formula into the automaton's nodes.
struct Lowering<'f> {
    formula: &'f CheckedFormula,
    atoms: Vec<FormulaNodeId>,
    nodes: Vec<Node>,
    index: BTreeMap<Node, usize>,
    trackers: Vec<Tracker>,
    shapes: BTreeMap<usize, Shape>,
}

impl Lowering<'_> {
    fn add(&mut self, node: Node) -> usize {
        if let Some(&at) = self.index.get(&node) {
            return at;
        }
        let at = self.nodes.len();
        self.index.insert(node.clone(), at);
        self.nodes.push(node);
        at
    }

    /// An `until` or `release` occurrence of `shape`: never shared with an
    /// equal one elsewhere, whose starts it would mix with its own.
    fn add_temporal(&mut self, node: Node, shape: Shape) -> usize {
        let at = self.nodes.len();
        self.nodes.push(node);
        self.shapes.insert(at, shape);
        at
    }

    fn form(&self, id: FormulaNodeId) -> Result<&FormulaNode, AutomatonError> {
        self.formula
            .nodes()
            .get(id.index())
            .ok_or(AutomatonError::Invariant {
                invariant: "timed-claim-node-in-arena",
            })
    }

    fn atom(&self, id: FormulaNodeId) -> Result<usize, AutomatonError> {
        self.atoms
            .iter()
            .position(|known| *known == id)
            .ok_or(AutomatonError::Invariant {
                invariant: "timed-claim-atom-known",
            })
    }

    /// `id` (negated when `!positive`) in negation normal form; `many`
    /// when the negation starts it at many positions, `deep` when a future
    /// operator lies above it.
    fn lower(
        &mut self,
        id: FormulaNodeId,
        positive: bool,
        many: bool,
        deep: bool,
    ) -> Result<usize, AutomatonError> {
        let form = self.form(id)?.clone();
        Ok(match form {
            FormulaNode::Constant(value) => self.add(Node::Const(value == positive)),
            FormulaNode::Holds { .. } => {
                let atom = self.atom(id)?;
                self.add(Node::Lit { atom, positive })
            }
            FormulaNode::Aggregate { .. } => {
                return Err(AutomatonError::Invariant {
                    invariant: "timed-claim-no-aggregate",
                })
            }
            FormulaNode::Not(inner) => self.lower(inner, !positive, many, deep)?,
            FormulaNode::And(left, right) => {
                let (left, right) = (
                    self.lower(left, positive, many, deep)?,
                    self.lower(right, positive, many, deep)?,
                );
                self.add(if positive {
                    Node::And(left, right)
                } else {
                    Node::Or(left, right)
                })
            }
            FormulaNode::Or(left, right) => {
                let (left, right) = (
                    self.lower(left, positive, many, deep)?,
                    self.lower(right, positive, many, deep)?,
                );
                self.add(if positive {
                    Node::Or(left, right)
                } else {
                    Node::And(left, right)
                })
            }
            FormulaNode::Implies(left, right) => {
                let (left, right) = (
                    self.lower(left, !positive, many, deep)?,
                    self.lower(right, positive, many, deep)?,
                );
                self.add(if positive {
                    Node::Or(left, right)
                } else {
                    Node::And(left, right)
                })
            }
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                let window = window(interval.as_ref())?;
                match operator {
                    UnaryTemporal::Eventually | UnaryTemporal::Always => {
                        // `eventually` is `true until`; `always` is
                        // `false release`; negation swaps them.
                        let eventually = (operator == UnaryTemporal::Eventually) == positive;
                        let shape = future_shape(window.as_ref(), many, eventually)?;
                        // An `until`'s right operand is read once per
                        // instance; a `release`'s at many positions.
                        let operand = self.lower(operand, positive, many || !eventually, true)?;
                        let left = self.add(Node::Const(eventually));
                        self.add_temporal(
                            if eventually {
                                Node::Until {
                                    left,
                                    right: operand,
                                    window,
                                }
                            } else {
                                Node::Release {
                                    left,
                                    right: operand,
                                    window,
                                }
                            },
                            shape,
                        )
                    }
                    UnaryTemporal::Once | UnaryTemporal::Historically => {
                        let witness = self.past(operand, deep)?;
                        // `once φ` is `true since φ`; `historically φ` is
                        // `not (true since not φ)`.
                        let tracker = if operator == UnaryTemporal::Once {
                            self.tracker(PastExpr::Const(true), witness, window, deep)?
                        } else {
                            self.tracker(
                                PastExpr::Const(true),
                                PastExpr::Not(Box::new(witness)),
                                window,
                                deep,
                            )?
                        };
                        let value = (operator == UnaryTemporal::Once) == positive;
                        self.add(Node::Past {
                            tracker,
                            positive: value,
                        })
                    }
                }
            }
            FormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => {
                let window = window(interval.as_ref())?;
                match operator {
                    BinaryTemporal::Until | BinaryTemporal::Release => {
                        let until = (operator == BinaryTemporal::Until) == positive;
                        let shape = future_shape(window.as_ref(), many, until)?;
                        // An `until` reads its left operand at many
                        // positions, a `release` its right.
                        let (left, right) = (
                            self.lower(left, positive, many || until, true)?,
                            self.lower(right, positive, many || !until, true)?,
                        );
                        self.add_temporal(
                            if until {
                                Node::Until {
                                    left,
                                    right,
                                    window,
                                }
                            } else {
                                Node::Release {
                                    left,
                                    right,
                                    window,
                                }
                            },
                            shape,
                        )
                    }
                    BinaryTemporal::Since | BinaryTemporal::Triggered => {
                        let (guard, witness) = (self.past(left, deep)?, self.past(right, deep)?);
                        // `triggered` is `not (not left since not right)`.
                        let tracker = if operator == BinaryTemporal::Since {
                            self.tracker(guard, witness, window, deep)?
                        } else {
                            self.tracker(
                                PastExpr::Not(Box::new(guard)),
                                PastExpr::Not(Box::new(witness)),
                                window,
                                deep,
                            )?
                        };
                        let value = (operator == BinaryTemporal::Since) == positive;
                        self.add(Node::Past {
                            tracker,
                            positive: value,
                        })
                    }
                }
            }
        })
    }

    /// A past-only formula, its inner past operators tracked first;
    /// `deep` when a future operator lies above it.
    fn past(&mut self, id: FormulaNodeId, deep: bool) -> Result<PastExpr, AutomatonError> {
        let form = self.form(id)?.clone();
        Ok(match form {
            FormulaNode::Constant(value) => PastExpr::Const(value),
            FormulaNode::Holds { .. } => PastExpr::Lit(self.atom(id)?),
            FormulaNode::Aggregate { .. } => {
                return Err(AutomatonError::Invariant {
                    invariant: "timed-claim-no-aggregate",
                })
            }
            FormulaNode::Not(inner) => PastExpr::Not(Box::new(self.past(inner, deep)?)),
            FormulaNode::And(left, right) => PastExpr::And(
                Box::new(self.past(left, deep)?),
                Box::new(self.past(right, deep)?),
            ),
            FormulaNode::Or(left, right) => PastExpr::Or(
                Box::new(self.past(left, deep)?),
                Box::new(self.past(right, deep)?),
            ),
            FormulaNode::Implies(left, right) => PastExpr::Or(
                Box::new(PastExpr::Not(Box::new(self.past(left, deep)?))),
                Box::new(self.past(right, deep)?),
            ),
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                let window = window(interval.as_ref())?;
                match operator {
                    UnaryTemporal::Once | UnaryTemporal::Historically => {
                        let witness = self.past(operand, deep)?;
                        if operator == UnaryTemporal::Once {
                            PastExpr::Tracker(self.tracker(
                                PastExpr::Const(true),
                                witness,
                                window,
                                deep,
                            )?)
                        } else {
                            PastExpr::Not(Box::new(PastExpr::Tracker(self.tracker(
                                PastExpr::Const(true),
                                PastExpr::Not(Box::new(witness)),
                                window,
                                deep,
                            )?)))
                        }
                    }
                    UnaryTemporal::Eventually | UnaryTemporal::Always => {
                        return Err(AutomatonError::PastOverFuture)
                    }
                }
            }
            FormulaNode::Binary {
                operator,
                interval,
                left,
                right,
            } => {
                let window = window(interval.as_ref())?;
                match operator {
                    BinaryTemporal::Since | BinaryTemporal::Triggered => {
                        let (guard, witness) = (self.past(left, deep)?, self.past(right, deep)?);
                        if operator == BinaryTemporal::Since {
                            PastExpr::Tracker(self.tracker(guard, witness, window, deep)?)
                        } else {
                            PastExpr::Not(Box::new(PastExpr::Tracker(self.tracker(
                                PastExpr::Not(Box::new(guard)),
                                PastExpr::Not(Box::new(witness)),
                                window,
                                deep,
                            )?)))
                        }
                    }
                    BinaryTemporal::Until | BinaryTemporal::Release => {
                        return Err(AutomatonError::PastOverFuture)
                    }
                }
            }
        })
    }

    /// A tracker of `guard since_I witness`; `deep` when a future operator
    /// lies above it, so that it is read at positions after 0.
    fn tracker(
        &mut self,
        guard: PastExpr,
        witness: PastExpr,
        window: Option<TimedInterval>,
        deep: bool,
    ) -> Result<usize, AutomatonError> {
        let shape = match &window {
            Some(window) if deep && !from_zero(window) => {
                if window.lower() == window.upper() {
                    return Err(AutomatonError::RepeatedPunctualInterval);
                }
                Shape::Blocks(spread(window)? + 1)
            }
            _ => Shape::Single,
        };
        self.trackers.push(Tracker {
            guard,
            witness,
            window,
            origin: !deep,
            shape,
            clock: None,
        });
        Ok(self.trackers.len() - 1)
    }
}

/// The timed interval an operator reads, `None` for an unbounded one.
fn window(interval: Option<&CheckedInterval>) -> Result<Option<TimedInterval>, AutomatonError> {
    match interval {
        None => Ok(None),
        Some(CheckedInterval::Timed(interval)) => Ok(Some(interval.clone())),
        Some(CheckedInterval::Steps(_)) => Err(AutomatonError::Invariant {
            invariant: "timed-claim-reads-timed-intervals",
        }),
    }
}

/// Whether `window` starts at a closed 0.
fn from_zero(window: &TimedInterval) -> bool {
    window.lower() == &zero() && window.lower_closed()
}

/// How the pending instances of an `until` (`until`) or `release` with
/// `window` are kept; `many` when the negation starts it at many positions.
fn future_shape(
    window: Option<&TimedInterval>,
    many: bool,
    until: bool,
) -> Result<Shape, AutomatonError> {
    match window {
        Some(window) if many && !from_zero(window) => {
            if window.lower() == window.upper() {
                return Err(AutomatonError::RepeatedPunctualInterval);
            }
            let spread = spread(window)?;
            Ok(Shape::Blocks(if until {
                2 * spread + 1
            } else {
                spread + 1
            }))
        }
        _ => Ok(Shape::Single),
    }
}

/// The most automaton clocks a claim builds: a zone over them has their
/// square of bounds, so a copy count grown by an interval's variability is
/// refused past it rather than exhausting memory.
const MAX_CLOCKS: usize = 64;

/// The most block starts of a non-punctual `window` from `a` to `b` that
/// lie within the last `a` and at least `d = b - a` apart, more than `d`
/// apart when either end is closed: `max(1, ceil(a / d))` then, and
/// `floor(a / d) + 1` with both ends open. Blocks after the oldest pending
/// one begin within the last `a` (a `release` block, or one parity class
/// of `until` blocks), so this bounds the copies by the interval's
/// variability.
fn spread(window: &TimedInterval) -> Result<usize, AutomatonError> {
    let width = window.upper().sub(window.lower());
    let ratio = window
        .lower()
        .div(&width)
        .ok_or(AutomatonError::Invariant {
            invariant: "timed-claim-non-punctual-width",
        })?;
    let denominator =
        NonZeroInteger::try_from(ratio.denominator()).map_err(|_| AutomatonError::Invariant {
            invariant: "timed-claim-rational-denominator",
        })?;
    let (floor, remainder) = ratio.numerator().div_mod_floor(denominator);
    let limit = AutomatonError::ClockLimit {
        clocks: usize::MAX,
        limit: MAX_CLOCKS,
    };
    let floor = usize::try_from(floor.to_u64().ok_or(limit)?).map_err(|_| limit)?;
    // Each block takes two clocks: past the limit, refuse before counting.
    if floor > MAX_CLOCKS {
        return Err(limit);
    }
    let strict = window.lower_closed() || window.upper_closed();
    Ok(if strict && remainder.is_zero() {
        floor.max(1)
    } else {
        floor + 1
    })
}

/// `b - a` of `window`.
fn width(window: &TimedInterval) -> Rational {
    window.upper().sub(window.lower())
}

/// `clock` has reached `window`'s lower end; empty for a closed 0.
fn lower_ok(window: &TimedInterval, clock: usize) -> Vec<PlaceConstraint> {
    if from_zero(window) {
        return Vec::new();
    }
    let op = if window.lower_closed() {
        ClockOp::GreaterOrEqual
    } else {
        ClockOp::Greater
    };
    vec![PlaceConstraint::new(clock, op, window.lower().clone())]
}

/// `clock` has not reached `window`'s lower end; `None` for a closed 0.
fn lower_fail(window: &TimedInterval, clock: usize) -> Option<Vec<PlaceConstraint>> {
    if from_zero(window) {
        return None;
    }
    let op = if window.lower_closed() {
        ClockOp::Less
    } else {
        ClockOp::LessOrEqual
    };
    Some(vec![PlaceConstraint::new(
        clock,
        op,
        window.lower().clone(),
    )])
}

/// `clock` has not passed `window`'s upper end.
fn upper_ok(window: &TimedInterval, clock: usize) -> Vec<PlaceConstraint> {
    let op = if window.upper_closed() {
        ClockOp::LessOrEqual
    } else {
        ClockOp::Less
    };
    vec![PlaceConstraint::new(clock, op, window.upper().clone())]
}

/// `clock` has passed `window`'s upper end.
fn upper_fail(window: &TimedInterval, clock: usize) -> Vec<PlaceConstraint> {
    let op = if window.upper_closed() {
        ClockOp::Greater
    } else {
        ClockOp::GreaterOrEqual
    };
    vec![PlaceConstraint::new(clock, op, window.upper().clone())]
}

/// A block whose first start is `clock` ago can still be discharged at one
/// position with a start now: some later offset lies in `window` from both.
fn joins(window: &TimedInterval, clock: usize) -> PlaceConstraint {
    let op = if window.lower_closed() && window.upper_closed() {
        ClockOp::LessOrEqual
    } else {
        ClockOp::Less
    };
    PlaceConstraint::new(clock, op, width(window))
}

/// A start now has a window that meets, or touches at a covered point, the
/// window of a start `clock` ago (`meets`), or does not (`!meets`).
fn meets(window: &TimedInterval, clock: usize, meets: bool) -> PlaceConstraint {
    let touching = window.lower_closed() || window.upper_closed();
    let op = match (meets, touching) {
        (true, true) => ClockOp::LessOrEqual,
        (true, false) => ClockOp::Less,
        (false, true) => ClockOp::Greater,
        (false, false) => ClockOp::GreaterOrEqual,
    };
    PlaceConstraint::new(clock, op, width(window))
}

/// The lowest slot of `capacity` not in `taken`.
fn free_slot(taken: &[usize], capacity: usize) -> Option<usize> {
    (0..capacity).find(|slot| !taken.contains(slot))
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// Where a clock lies against an interval.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Region {
    Before,
    In,
    After,
}

/// The clock constraints of `region` of `window` on `clock`; `None` when
/// the region is empty.
fn region(window: &TimedInterval, clock: usize, region: Region) -> Option<Vec<PlaceConstraint>> {
    let lower = window.lower().clone();
    let upper = window.upper().clone();
    match region {
        Region::Before => {
            if lower == zero() && window.lower_closed() {
                return None;
            }
            let op = if window.lower_closed() {
                ClockOp::Less
            } else {
                ClockOp::LessOrEqual
            };
            Some(vec![PlaceConstraint::new(clock, op, lower)])
        }
        Region::In => {
            let low = if window.lower_closed() {
                ClockOp::GreaterOrEqual
            } else {
                ClockOp::Greater
            };
            let high = if window.upper_closed() {
                ClockOp::LessOrEqual
            } else {
                ClockOp::Less
            };
            let mut constraints = Vec::new();
            if !(lower == zero() && window.lower_closed()) {
                constraints.push(PlaceConstraint::new(clock, low, lower));
            }
            constraints.push(PlaceConstraint::new(clock, high, upper));
            Some(constraints)
        }
        Region::After => {
            let op = if window.upper_closed() {
                ClockOp::Greater
            } else {
                ClockOp::GreaterOrEqual
            };
            Some(vec![PlaceConstraint::new(clock, op, upper)])
        }
    }
}

/// One past-tracker case of a position: its clock guard, each tracker's
/// value and memory after the position, and the tracker clocks reset.
#[derive(Clone, Debug)]
struct PastCase {
    guard: Vec<PlaceConstraint>,
    values: Vec<bool>,
    seen: Vec<bool>,
    /// Per tracker, its witness blocks after the position.
    witnesses: Vec<Vec<usize>>,
    resets: BTreeSet<usize>,
}

/// One branch of a position's expansion.
#[derive(Clone, Debug)]
struct Branch {
    guard: Vec<PlaceConstraint>,
    resets: BTreeSet<usize>,
    /// The pending instances kept from the state read.
    kept: BTreeSet<usize>,
    /// The pending instances after the position.
    pending: BTreeSet<usize>,
    /// The blocks after the position, by block-shaped node.
    blocks: BTreeMap<usize, Vec<usize>>,
    /// The block-shaped nodes started at the position.
    started: BTreeSet<usize>,
    discharged: BTreeSet<usize>,
    /// The formulas the position must satisfy, still to expand.
    now: Vec<usize>,
}

impl TimedClaim {
    /// The automaton of `clause`'s negation, with its activation, numbering
    /// at most `max_states` states as they are reached.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::ProfileNotTranslated`] for a clause not under the
    /// timed profile; [`AutomatonError::RepeatedPunctualInterval`] for a
    /// punctual interval started or read at many positions;
    /// [`AutomatonError::ClockLimit`] past the clocks it builds;
    /// [`AutomatonError::PastOverFuture`]; [`AutomatonError::AtomOutOfRange`]
    /// when the letter needs more atoms than it holds.
    pub fn new(clause: &CheckedTemporalClause, max_states: u64) -> Result<Self, AutomatonError> {
        if clause.profile() != TemporalProfile::Timed {
            return Err(AutomatonError::ProfileNotTranslated {
                profile: clause.profile(),
            });
        }
        let formula = clause.formula();
        let atoms: Vec<FormulaNodeId> = formula
            .nodes()
            .iter()
            .enumerate()
            .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
            .filter_map(|(index, _)| formula.node_id(index))
            .collect();
        let mut lowering = Lowering {
            formula,
            atoms,
            nodes: Vec::new(),
            index: BTreeMap::new(),
            trackers: Vec::new(),
            shapes: BTreeMap::new(),
        };
        let each = matches!(clause.activation(), Activation::Each { .. });
        // Under `on each` the negation is read once, at the activating
        // position the `until` below chooses, which is not position 0.
        let negation = lowering.lower(formula.root(), false, false, each)?;
        let when = match clause.activation() {
            Activation::Each { guarded: true } => Some(Atom::new(lowering.atoms.len())?),
            _ => None,
        };
        let root = if each {
            // `on each`: some activating position violates the body.
            let violation = match when {
                Some(when) => {
                    let lit = lowering.add(Node::Lit {
                        atom: when.index(),
                        positive: true,
                    });
                    lowering.add(Node::And(lit, negation))
                }
                None => negation,
            };
            let left = lowering.add(Node::Const(true));
            lowering.add_temporal(
                Node::Until {
                    left,
                    right: violation,
                    window: None,
                },
                Shape::Single,
            )
        } else {
            negation
        };
        let Lowering {
            nodes,
            mut trackers,
            atoms,
            shapes,
            ..
        } = lowering;
        // One clock for a single instance with an interval, two per block.
        let span = |shape: Shape| match shape {
            Shape::Single => 1,
            Shape::Blocks(capacity) => 2 * capacity,
        };
        let mut clock_count = 0_usize;
        let mut clocks = BTreeMap::new();
        for (at, node) in nodes.iter().enumerate() {
            if let Node::Until {
                window: Some(_), ..
            }
            | Node::Release {
                window: Some(_), ..
            } = node
            {
                clocks.insert(at, clock_count);
                clock_count = clock_count
                    .saturating_add(span(shapes.get(&at).copied().unwrap_or(Shape::Single)));
            }
        }
        for tracker in &mut trackers {
            if tracker.window.is_some() {
                tracker.clock = Some(clock_count);
                clock_count = clock_count.saturating_add(span(tracker.shape));
            }
        }
        if clock_count > MAX_CLOCKS {
            return Err(AutomatonError::ClockLimit {
                clocks: clock_count,
                limit: MAX_CLOCKS,
            });
        }
        let eventualities = nodes
            .iter()
            .enumerate()
            .filter(|(_, node)| matches!(node, Node::Until { window: None, .. }))
            .map(|(at, _)| at)
            .collect();
        // Every atom fits the letter.
        if let Some(last) = atoms.len().checked_sub(1) {
            Atom::new(last)?;
        }
        let mut claim = Self {
            nodes,
            root,
            trackers,
            shapes,
            clocks,
            clock_count,
            eventualities,
            atoms,
            when,
            states: Vec::new(),
            index: BTreeMap::new(),
            expanded: BTreeMap::new(),
            max_states,
        };
        claim.initial()?;
        Ok(claim)
    }

    /// The `holds` atoms of the clause: atom `i` of the letter is
    /// `atoms()[i]`.
    pub fn atoms(&self) -> &[FormulaNodeId] {
        &self.atoms
    }

    /// The atom that holds where an `on each` activation's `when` holds.
    pub fn when(&self) -> Option<Atom> {
        self.when
    }

    /// The number of automaton clocks.
    pub fn clock_count(&self) -> usize {
        self.clock_count
    }

    /// The greatest constant each automaton clock is compared with, by
    /// clock: its operator's interval's upper end. Each joins that clock's
    /// `L` and `U` and the scale (FR-242).
    pub fn constants(&self) -> Vec<Rational> {
        let mut constants = vec![zero(); self.clock_count];
        let mut set = |first: usize, shape: Shape, window: &TimedInterval| {
            let count = match shape {
                Shape::Single => 1,
                Shape::Blocks(capacity) => 2 * capacity,
            };
            for clock in first..first + count {
                if let Some(constant) = constants.get_mut(clock) {
                    *constant = window.upper().clone();
                }
            }
        };
        for (&node, &clock) in &self.clocks {
            if let Some(
                Node::Until {
                    window: Some(window),
                    ..
                }
                | Node::Release {
                    window: Some(window),
                    ..
                },
            ) = self.nodes.get(node)
            {
                set(clock, self.shape(node), window);
            }
        }
        for tracker in &self.trackers {
            if let (Some(clock), Some(window)) = (tracker.clock, &tracker.window) {
                set(clock, tracker.shape, window);
            }
        }
        constants
    }

    fn shape(&self, node: usize) -> Shape {
        self.shapes.get(&node).copied().unwrap_or(Shape::Single)
    }

    /// Every constant the automaton's guards compare a clock with, which
    /// joins the scale.
    pub fn all_constants(&self) -> Vec<Rational> {
        let mut all = Vec::new();
        let windows = self
            .nodes
            .iter()
            .filter_map(|node| match node {
                Node::Until { window, .. } | Node::Release { window, .. } => window.as_ref(),
                _ => None,
            })
            .chain(
                self.trackers
                    .iter()
                    .filter_map(|tracker| tracker.window.as_ref()),
            );
        for window in windows {
            all.push(window.lower().clone());
            all.push(window.upper().clone());
            // Blocks join and meet by the width.
            all.push(width(window));
        }
        all.sort();
        all.dedup();
        all
    }

    /// The initial state, read before position 0. It is state 0.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::StateLimit`] when the limit admits no state.
    pub fn initial(&mut self) -> Result<StateId, AutomatonError> {
        let config = Config {
            started: false,
            seen: vec![false; self.trackers.len()],
            pending: BTreeSet::new(),
            blocks: BTreeMap::new(),
            witnesses: vec![Vec::new(); self.trackers.len()],
            fulfilled: vec![true; self.eventualities.len()],
        };
        self.intern(config)
    }

    /// The number of acceptance sets: one per unbounded `until`.
    pub fn acceptance_set_count(&self) -> usize {
        self.eventualities.len()
    }

    /// Whether `state` is in acceptance set `set`: the last position
    /// fulfilled that `until`.
    pub fn is_accepting_in(&self, set: usize, state: StateId) -> bool {
        self.states
            .get(state.index())
            .and_then(|config| config.fulfilled.get(set))
            .copied()
            .unwrap_or(false)
    }

    /// The state numbered `index`, once reached.
    pub fn reached_state(&self, index: usize) -> Option<StateId> {
        (index < self.states.len()).then(|| StateId::new(index))
    }

    fn intern(&mut self, config: Config) -> Result<StateId, AutomatonError> {
        if let Some(&at) = self.index.get(&config) {
            return Ok(StateId::new(at));
        }
        if u64::try_from(self.states.len()).unwrap_or(u64::MAX) >= self.max_states {
            return Err(AutomatonError::StateLimit {
                limit: self.max_states,
            });
        }
        let at = self.states.len();
        self.index.insert(config.clone(), at);
        self.states.push(config);
        Ok(StateId::new(at))
    }

    /// The transitions from `from` reading `letter` at a position, ordered
    /// and without repeats.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] for a state not reached;
    /// [`AutomatonError::StateLimit`] past the state limit.
    pub fn successors(
        &mut self,
        from: StateId,
        letter: Letter,
    ) -> Result<Vec<ClockedEdge>, AutomatonError> {
        if let Some(known) = self.expanded.get(&(from.index(), letter)) {
            return Ok(known.clone());
        }
        let config = self
            .states
            .get(from.index())
            .cloned()
            .ok_or(AutomatonError::UnknownState { state: from })?;
        let mut found: Vec<(Config, Vec<PlaceConstraint>, Vec<usize>)> = Vec::new();
        for case in self.past_cases(&config, letter)? {
            for branch in self.branches(&config, &case, letter)? {
                let fulfilled = self
                    .eventualities
                    .iter()
                    .map(|node| !config.pending.contains(node) || branch.discharged.contains(node))
                    .collect();
                let mut blocks = branch.blocks.clone();
                blocks.retain(|_, slots| !slots.is_empty());
                let target = Config {
                    started: true,
                    seen: case.seen.clone(),
                    pending: branch.pending.clone(),
                    blocks,
                    witnesses: case.witnesses.clone(),
                    fulfilled,
                };
                // Clocks of no pending instance, block or remembered
                // witness read nothing until reset: reset them now, so
                // equal obligations meet in equal zones.
                let mut resets = branch.resets.clone();
                for (&node, &clock) in &self.clocks {
                    match self.shape(node) {
                        Shape::Single => {
                            if !target.pending.contains(&node) {
                                resets.insert(clock);
                            }
                        }
                        Shape::Blocks(capacity) => {
                            let taken = target.blocks.get(&node).map_or(&[][..], Vec::as_slice);
                            for slot in (0..capacity).filter(|slot| !taken.contains(slot)) {
                                resets.insert(clock + 2 * slot);
                                resets.insert(clock + 2 * slot + 1);
                            }
                        }
                    }
                }
                for (at, tracker) in self.trackers.iter().enumerate() {
                    let Some(clock) = tracker.clock else {
                        continue;
                    };
                    match tracker.shape {
                        Shape::Single => {
                            if !target.seen.get(at).copied().unwrap_or(false) {
                                resets.insert(clock);
                            }
                        }
                        Shape::Blocks(capacity) => {
                            let taken = target.witnesses.get(at).map_or(&[][..], Vec::as_slice);
                            for slot in (0..capacity).filter(|slot| !taken.contains(slot)) {
                                resets.insert(clock + 2 * slot);
                                resets.insert(clock + 2 * slot + 1);
                            }
                        }
                    }
                }
                let mut guard = branch.guard.clone();
                guard.sort();
                guard.dedup();
                if !satisfiable(&guard) {
                    continue;
                }
                found.push((target, guard, resets.into_iter().collect()));
            }
        }
        found.sort();
        found.dedup();
        let mut edges = Vec::with_capacity(found.len());
        for (target, guard, resets) in found {
            let target = self.intern(target)?;
            edges.push(ClockedEdge {
                target,
                guard,
                resets,
            });
        }
        edges.sort();
        edges.dedup();
        self.expanded.insert((from.index(), letter), edges.clone());
        Ok(edges)
    }

    /// The past trackers' cases at a position reading `letter`.
    fn past_cases(&self, config: &Config, letter: Letter) -> Result<Vec<PastCase>, AutomatonError> {
        let mut cases = vec![PastCase {
            guard: Vec::new(),
            values: Vec::with_capacity(self.trackers.len()),
            seen: Vec::with_capacity(self.trackers.len()),
            witnesses: Vec::with_capacity(self.trackers.len()),
            resets: BTreeSet::new(),
        }];
        for (at, tracker) in self.trackers.iter().enumerate() {
            let mut next = Vec::with_capacity(cases.len());
            for case in cases {
                if let (Shape::Blocks(capacity), Some(window), Some(clock)) =
                    (tracker.shape, &tracker.window, tracker.clock)
                {
                    let witness = past_value(&tracker.witness, letter, &case.values)?;
                    let guard = past_value(&tracker.guard, letter, &case.values)?;
                    let previous = match (config.started && guard, config.witnesses.get(at)) {
                        (true, Some(slots)) => slots.clone(),
                        _ => Vec::new(),
                    };
                    for (mut case, slots, value) in
                        witness_blocks(window, clock, capacity, &previous, witness, case)
                    {
                        case.values.push(value);
                        case.seen.push(false);
                        case.witnesses.push(slots);
                        next.push(case);
                    }
                    continue;
                }
                let mut case = case;
                case.witnesses.push(Vec::new());
                if tracker.origin && config.started {
                    // Read at position 0 only.
                    let mut case = case;
                    case.values.push(false);
                    case.seen.push(false);
                    next.push(case);
                    continue;
                }
                let witness = past_value(&tracker.witness, letter, &case.values)?;
                let guard = past_value(&tracker.guard, letter, &case.values)?;
                let previous = config.started && config.seen.get(at).copied().unwrap_or(false);
                if witness {
                    let mut case = case;
                    case.values
                        .push(tracker.window.as_ref().is_none_or(|w| w.contains(&zero())));
                    case.seen.push(true);
                    if let Some(clock) = tracker.clock {
                        case.resets.insert(clock);
                    }
                    next.push(case);
                } else if !guard || !previous {
                    let mut case = case;
                    case.values.push(false);
                    case.seen.push(false);
                    next.push(case);
                } else {
                    match (&tracker.window, tracker.clock) {
                        (Some(window), Some(clock)) => {
                            for (inside, part) in [(true, Region::In), (false, Region::After)] {
                                let Some(constraints) = region(window, clock, part) else {
                                    continue;
                                };
                                let mut case = case.clone();
                                case.guard.extend(constraints);
                                case.values.push(inside);
                                // An expired witness stays expired.
                                case.seen.push(inside);
                                next.push(case);
                            }
                        }
                        _ => {
                            let mut case = case;
                            case.values.push(true);
                            case.seen.push(true);
                            next.push(case);
                        }
                    }
                }
            }
            cases = next;
        }
        Ok(cases)
    }

    /// The branches of a position from `config` under the past `case`.
    fn branches(
        &self,
        config: &Config,
        case: &PastCase,
        letter: Letter,
    ) -> Result<Vec<Branch>, AutomatonError> {
        let start = Branch {
            guard: case.guard.clone(),
            resets: case.resets.clone(),
            kept: BTreeSet::new(),
            pending: BTreeSet::new(),
            blocks: BTreeMap::new(),
            started: BTreeSet::new(),
            discharged: BTreeSet::new(),
            now: if config.started {
                Vec::new()
            } else {
                vec![self.root]
            },
        };
        let mut branches = vec![start];
        // The pending instances, each in its clock's region.
        for &node in &config.pending {
            let (left, right, window, until) = match self.nodes.get(node) {
                Some(Node::Until {
                    left,
                    right,
                    window,
                }) => (*left, *right, window.as_ref(), true),
                Some(Node::Release {
                    left,
                    right,
                    window,
                }) => (*left, *right, window.as_ref(), false),
                _ => {
                    return Err(AutomatonError::Invariant {
                        invariant: "timed-claim-pending-is-temporal",
                    })
                }
            };
            let regions: Vec<(Region, Vec<PlaceConstraint>)> =
                match (window, self.clocks.get(&node)) {
                    (Some(window), Some(&clock)) => [Region::Before, Region::In, Region::After]
                        .into_iter()
                        .filter_map(|part| region(window, clock, part).map(|c| (part, c)))
                        .collect(),
                    _ => vec![(Region::In, Vec::new())],
                };
            let mut next = Vec::new();
            for branch in branches {
                for (part, constraints) in &regions {
                    let mut base = branch.clone();
                    base.guard.extend(constraints.iter().cloned());
                    match (until, part) {
                        (true, Region::After) => {}
                        (true, Region::Before) => {
                            let mut keep = base;
                            keep.now.push(left);
                            keep.kept.insert(node);
                            keep.pending.insert(node);
                            next.push(keep);
                        }
                        (true, Region::In) => {
                            let mut discharge = base.clone();
                            discharge.now.push(right);
                            discharge.discharged.insert(node);
                            next.push(discharge);
                            let mut keep = base;
                            keep.now.push(left);
                            keep.kept.insert(node);
                            keep.pending.insert(node);
                            next.push(keep);
                        }
                        (false, Region::After) => next.push(base),
                        (false, before_or_in) => {
                            if *before_or_in == Region::In {
                                base.now.push(right);
                            }
                            let mut escape = base.clone();
                            escape.now.push(left);
                            next.push(escape);
                            let mut keep = base;
                            keep.kept.insert(node);
                            keep.pending.insert(node);
                            next.push(keep);
                        }
                    }
                }
            }
            branches = next;
        }
        // The pending blocks, oldest first.
        for (&node, slots) in &config.blocks {
            let (left, right, window, until, base) = self.blocked(node)?;
            for &slot in slots {
                let (first, last) = (base + 2 * slot, base + 2 * slot + 1);
                let mut next = Vec::new();
                for branch in branches {
                    let keep = |mut branch: Branch| {
                        branch.blocks.entry(node).or_default().push(slot);
                        branch
                    };
                    if until {
                        // Alive while its first start's window lasts; it
                        // may be discharged where its last start's window
                        // has begun.
                        let mut alive = branch;
                        alive.guard.extend(upper_ok(window, first));
                        let mut discharge = alive.clone();
                        discharge.guard.extend(lower_ok(window, last));
                        push_now(&mut discharge, right);
                        next.push(discharge);
                        push_now(&mut alive, left);
                        next.push(keep(alive));
                    } else {
                        let before = lower_fail(window, first);
                        let mut inside = lower_ok(window, first);
                        inside.extend(upper_ok(window, last));
                        for (guard, holds) in before
                            .map(|guard| (guard, false))
                            .into_iter()
                            .chain([(inside, true)])
                        {
                            let mut base = branch.clone();
                            base.guard.extend(guard);
                            if holds {
                                push_now(&mut base, right);
                            }
                            let mut escape = base.clone();
                            push_now(&mut escape, left);
                            next.push(escape);
                            next.push(keep(base));
                        }
                        // Past every start's window: done.
                        let mut after = branch;
                        after.guard.extend(lower_ok(window, first));
                        after.guard.extend(upper_fail(window, last));
                        next.push(after);
                    }
                }
                branches = next;
            }
        }
        // The formulas the position must satisfy.
        let mut done = Vec::new();
        while let Some(mut branch) = branches.pop() {
            let Some(node) = branch.now.pop() else {
                done.push(branch);
                continue;
            };
            match self.nodes.get(node) {
                Some(Node::Const(true)) => branches.push(branch),
                Some(Node::Const(false)) | None => {}
                Some(Node::Lit { atom, positive }) => {
                    if letter.holds(Atom::new(*atom)?) == *positive {
                        branches.push(branch);
                    }
                }
                Some(Node::Past { tracker, positive }) => {
                    if case.values.get(*tracker).copied() == Some(*positive) {
                        branches.push(branch);
                    }
                }
                Some(Node::And(left, right)) => {
                    branch.now.push(*left);
                    branch.now.push(*right);
                    branches.push(branch);
                }
                Some(Node::Or(left, right)) => {
                    let mut other = branch.clone();
                    branch.now.push(*left);
                    other.now.push(*right);
                    branches.push(branch);
                    branches.push(other);
                }
                Some(Node::Until { left, .. } | Node::Release { left, .. })
                    if matches!(self.shape(node), Shape::Blocks(_)) =>
                {
                    // A block-shaped start, once per position.
                    if !branch.started.insert(node) {
                        branches.push(branch);
                        continue;
                    }
                    let left = *left;
                    let (_, _, window, until, base) = self.blocked(node)?;
                    let Shape::Blocks(capacity) = self.shape(node) else {
                        continue;
                    };
                    let slots = branch.blocks.get(&node).cloned().unwrap_or_default();
                    // A start's window misses 0, so it reads nothing now.
                    if !until {
                        let mut escape = branch.clone();
                        escape.now.push(left);
                        branches.push(escape);
                    } else {
                        branch.now.push(left);
                    }
                    let newest = slots
                        .last()
                        .map(|slot| (base + 2 * slot, base + 2 * slot + 1));
                    let mut fresh = branch.clone();
                    if let Some((first, last)) = newest {
                        let mut join = branch;
                        join.guard.push(if until {
                            joins(window, first)
                        } else {
                            meets(window, last, true)
                        });
                        join.resets.insert(last);
                        branches.push(join);
                        if !until {
                            fresh.guard.push(meets(window, last, false));
                        }
                    }
                    if let Some(slot) = free_slot(&slots, capacity) {
                        fresh.blocks.entry(node).or_default().push(slot);
                        fresh.resets.insert(base + 2 * slot);
                        fresh.resets.insert(base + 2 * slot + 1);
                        branches.push(fresh);
                    }
                }
                Some(Node::Until {
                    left,
                    right,
                    window,
                }) => {
                    if window.as_ref().is_none_or(|w| w.contains(&zero())) {
                        let mut now = branch.clone();
                        now.now.push(*right);
                        branches.push(now);
                    }
                    branch.now.push(*left);
                    // The older instance implies the one started now.
                    if !branch.pending.contains(&node) {
                        branch.pending.insert(node);
                        if let Some(&clock) = self.clocks.get(&node) {
                            branch.resets.insert(clock);
                        }
                    }
                    branches.push(branch);
                }
                Some(Node::Release {
                    left,
                    right,
                    window,
                }) => {
                    if window.as_ref().is_none_or(|w| w.contains(&zero())) {
                        branch.now.push(*right);
                    }
                    let mut escape = branch.clone();
                    escape.now.push(*left);
                    branches.push(escape);
                    // The instance started now implies an older one.
                    branch.pending.insert(node);
                    if let Some(&clock) = self.clocks.get(&node) {
                        branch.resets.insert(clock);
                    }
                    branches.push(branch);
                }
            }
        }
        Ok(done)
    }
}

impl TimedClaim {
    /// A block-shaped node's operands, interval, kind (`until` or not) and
    /// first clock.
    fn blocked(
        &self,
        node: usize,
    ) -> Result<(usize, usize, &TimedInterval, bool, usize), AutomatonError> {
        let broken = AutomatonError::Invariant {
            invariant: "timed-claim-block-has-interval-and-clock",
        };
        let base = *self.clocks.get(&node).ok_or(broken)?;
        match self.nodes.get(node) {
            Some(Node::Until {
                left,
                right,
                window: Some(window),
            }) => Ok((*left, *right, window, true, base)),
            Some(Node::Release {
                left,
                right,
                window: Some(window),
            }) => Ok((*left, *right, window, false, base)),
            _ => Err(broken),
        }
    }
}

/// Adds `node` to what the position must satisfy, once.
fn push_now(branch: &mut Branch, node: usize) {
    if !branch.now.contains(&node) {
        branch.now.push(node);
    }
}

/// A block-shaped tracker's cases at a position: from its blocks
/// `previous` (none when its guard failed now), each block before its
/// window (kept), in it (kept, the tracker holds) or past it (dropped);
/// then a witness now joins the newest block when their windows meet, or
/// begins a new one. Each case is the past case extended, the blocks after
/// the position and the tracker's value.
fn witness_blocks(
    window: &TimedInterval,
    base: usize,
    capacity: usize,
    previous: &[usize],
    witness: bool,
    case: PastCase,
) -> Vec<(PastCase, Vec<usize>, bool)> {
    let mut partial = vec![(case, Vec::new(), false)];
    for &slot in previous {
        let (first, last) = (base + 2 * slot, base + 2 * slot + 1);
        let mut next = Vec::new();
        for (case, slots, value) in partial {
            if let Some(guard) = lower_fail(window, first) {
                let mut before = (case.clone(), slots.clone(), value);
                before.0.guard.extend(guard);
                before.1.push(slot);
                next.push(before);
            }
            let mut inside = (case.clone(), slots.clone(), true);
            inside.0.guard.extend(lower_ok(window, first));
            inside.0.guard.extend(upper_ok(window, last));
            inside.1.push(slot);
            next.push(inside);
            let mut after = (case, slots, value);
            after.0.guard.extend(lower_ok(window, first));
            after.0.guard.extend(upper_fail(window, last));
            next.push(after);
        }
        partial = next;
    }
    if !witness {
        return partial;
    }
    let mut found = Vec::new();
    for (case, slots, value) in partial {
        let mut fresh = (case.clone(), slots.clone(), value);
        if let Some(&newest) = slots.last() {
            let last = base + 2 * newest + 1;
            let mut join = (case, slots, value);
            join.0.guard.push(meets(window, last, true));
            join.0.resets.insert(last);
            found.push(join);
            fresh.0.guard.push(meets(window, last, false));
        }
        if let Some(slot) = free_slot(&fresh.1, capacity) {
            fresh.1.push(slot);
            fresh.0.resets.insert(base + 2 * slot);
            fresh.0.resets.insert(base + 2 * slot + 1);
            found.push(fresh);
        }
    }
    found
}

/// Whether a conjunction of single-clock constraints has a solution.
fn satisfiable(guard: &[PlaceConstraint]) -> bool {
    let mut by_clock: BTreeMap<usize, Vec<&PlaceConstraint>> = BTreeMap::new();
    for constraint in guard {
        by_clock
            .entry(constraint.place())
            .or_default()
            .push(constraint);
    }
    by_clock.values().all(|constraints| {
        // The interval of values: lower end and whether it is open, upper.
        let mut low = (zero(), false);
        let mut high: Option<(Rational, bool)> = None;
        for constraint in constraints {
            let value = constraint.bound().clone();
            let tighten_low = |low: &mut (Rational, bool), open: bool| {
                if value > low.0 || (value == low.0 && open) {
                    *low = (value.clone(), open);
                }
            };
            let tighten_high = |high: &mut Option<(Rational, bool)>, open: bool| match high {
                Some((at, was_open)) if value > *at || (value == *at && *was_open) => {}
                _ => *high = Some((value.clone(), open)),
            };
            match constraint.op() {
                ClockOp::Less => tighten_high(&mut high, true),
                ClockOp::LessOrEqual => tighten_high(&mut high, false),
                ClockOp::Equal => {
                    tighten_low(&mut low, false);
                    tighten_high(&mut high, false);
                }
                ClockOp::GreaterOrEqual => tighten_low(&mut low, false),
                ClockOp::Greater => tighten_low(&mut low, true),
            }
        }
        match high {
            None => true,
            Some((upper, open)) => low.0 < upper || (low.0 == upper && !open && !low.1),
        }
    })
}

/// A past formula's value at the position, the trackers before it valued.
fn past_value(expr: &PastExpr, letter: Letter, values: &[bool]) -> Result<bool, AutomatonError> {
    Ok(match expr {
        PastExpr::Const(value) => *value,
        PastExpr::Lit(atom) => letter.holds(Atom::new(*atom)?),
        PastExpr::Tracker(at) => *values.get(*at).ok_or(AutomatonError::Invariant {
            invariant: "timed-claim-inner-tracker-first",
        })?,
        PastExpr::Not(inner) => !past_value(inner, letter, values)?,
        PastExpr::And(left, right) => {
            past_value(left, letter, values)? && past_value(right, letter, values)?
        }
        PastExpr::Or(left, right) => {
            past_value(left, letter, values)? || past_value(right, letter, values)?
        }
    })
}

/// Whether `clause` has a timed interval, which [`TimedClaim`] reads.
pub fn has_timed_interval(clause: &CheckedTemporalClause) -> bool {
    clause.profile() == TemporalProfile::Timed
        && clause.formula().nodes().iter().any(|node| {
            matches!(
                node,
                FormulaNode::Unary {
                    interval: Some(_),
                    ..
                } | FormulaNode::Binary {
                    interval: Some(_),
                    ..
                }
            )
        })
}

/// The claim automaton of EN-6's liveness product for a clause (FR-240,
/// FR-242): the negation automaton read over positions, untimed for a
/// clause with no timed interval and FR-242's timed automaton otherwise.
#[derive(Debug)]
pub enum PositionAutomaton {
    /// [`translate_position_negation`]'s automaton, with no clock.
    Untimed(Box<PropertyAutomaton>),
    /// [`TimedClaim`].
    Timed(Box<TimedClaim>),
}

/// The claim automaton of `clause` for EN-6's liveness product.
///
/// # Errors
///
/// As [`translate_position_negation`] and [`TimedClaim::new`].
pub fn position_automaton(
    clause: &CheckedTemporalClause,
    max_states: u64,
) -> Result<PositionAutomaton, AutomatonError> {
    if has_timed_interval(clause) {
        Ok(PositionAutomaton::Timed(Box::new(TimedClaim::new(
            clause, max_states,
        )?)))
    } else {
        Ok(PositionAutomaton::Untimed(Box::new(
            translate_position_negation(clause, max_states)?,
        )))
    }
}

impl PositionAutomaton {
    /// The initial state, read before position 0.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::StateLimit`] when the limit admits no state.
    pub fn initial(&mut self) -> Result<StateId, AutomatonError> {
        match self {
            Self::Untimed(automaton) => automaton.initial(),
            Self::Timed(claim) => claim.initial(),
        }
    }

    /// The transitions from `from` reading `letter`.
    ///
    /// # Errors
    ///
    /// As the automaton's own.
    pub fn successors(
        &mut self,
        from: StateId,
        letter: Letter,
    ) -> Result<Vec<ClockedEdge>, AutomatonError> {
        match self {
            Self::Untimed(automaton) => Ok(automaton
                .successors(from, letter)?
                .into_iter()
                .map(|target| ClockedEdge {
                    target,
                    guard: Vec::new(),
                    resets: Vec::new(),
                })
                .collect()),
            Self::Timed(claim) => claim.successors(from, letter),
        }
    }

    /// The `holds` atoms the letter reads.
    pub fn atoms(&self) -> &[FormulaNodeId] {
        match self {
            Self::Untimed(automaton) => automaton.atoms(),
            Self::Timed(claim) => claim.atoms(),
        }
    }

    /// The `when` atom of an `on each` activation.
    pub fn when(&self) -> Option<Atom> {
        match self {
            Self::Untimed(automaton) => automaton.when(),
            Self::Timed(claim) => claim.when(),
        }
    }

    /// The number of acceptance sets.
    pub fn acceptance_set_count(&self) -> usize {
        match self {
            Self::Untimed(automaton) => automaton.acceptance_set_count(),
            Self::Timed(claim) => claim.acceptance_set_count(),
        }
    }

    /// Whether `state` is in acceptance set `set`.
    pub fn is_accepting_in(&self, set: usize, state: StateId) -> bool {
        match self {
            Self::Untimed(automaton) => automaton.is_accepting_in(set, state),
            Self::Timed(claim) => claim.is_accepting_in(set, state),
        }
    }

    /// The state numbered `index`, once reached.
    pub fn reached_state(&self, index: usize) -> Option<StateId> {
        match self {
            Self::Untimed(automaton) => automaton.reached_state(index),
            Self::Timed(claim) => claim.reached_state(index),
        }
    }

    /// The number of automaton clocks.
    pub fn clock_count(&self) -> usize {
        match self {
            Self::Untimed(_) => 0,
            Self::Timed(claim) => claim.clock_count(),
        }
    }

    /// The greatest constant each automaton clock is compared with.
    pub fn constants(&self) -> Vec<Rational> {
        match self {
            Self::Untimed(_) => Vec::new(),
            Self::Timed(claim) => claim.constants(),
        }
    }

    /// Every constant the automaton's guards compare a clock with.
    pub fn all_constants(&self) -> Vec<Rational> {
        match self {
            Self::Untimed(_) => Vec::new(),
            Self::Timed(claim) => claim.all_constants(),
        }
    }
}

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
//! **One clock per operator.** An operator that starts at one position only
//! (no future operator above it, activation `on origin`) has one pending
//! instance at most, whatever its interval. One nested under a future
//! operator starts at many positions; with an interval whose lower end is a
//! closed 0, of two pending instances one implies the other, so one clock
//! suffices: the older `until`'s window ends first and holds the newer's
//! remaining positions, and the newer `release`'s window covers the older's
//! remaining positions, both reading the same guard from the newer start
//! on. The construction keeps the older `until` and the newer `release`.
//! A nested operator whose interval starts elsewhere has no such bound and
//! is refused [`AutomatonError::NestedIntervalShape`] (spec gap STD-493:
//! MightyL's general non-punctual construction).
//!
//! **Past operators** are tracked as memory: `since_I` remembers whether a
//! witness holds with its guard ever since, and the clock since the latest
//! such witness, which is the one an interval starting at a closed 0 reads.
//! A past operator nested under a future one with another interval is
//! refused alike; one read at position 0 only reads its single offset 0.
//! A past operator over a future operator is refused, as in
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
use quire_exact::{Integer, Rational};

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

/// `guard since_I witness`.
#[derive(Clone, Debug)]
struct Tracker {
    guard: PastExpr,
    witness: PastExpr,
    window: Option<TimedInterval>,
    /// Read at position 0 only.
    origin: bool,
    clock: Option<usize>,
}

/// One automaton state.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct Config {
    /// Whether position 0 has been read.
    started: bool,
    /// Per tracker: whether a witness holds with its guard ever since.
    seen: Vec<bool>,
    /// The `until` and `release` nodes with a pending instance.
    pending: BTreeSet<usize>,
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
    /// Each node's clock, by node.
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

    /// `id` (negated when `!positive`) in negation normal form; `nested`
    /// when a future operator lies above it.
    fn lower(
        &mut self,
        id: FormulaNodeId,
        positive: bool,
        nested: bool,
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
            FormulaNode::Not(inner) => self.lower(inner, !positive, nested)?,
            FormulaNode::And(left, right) => {
                let (left, right) = (
                    self.lower(left, positive, nested)?,
                    self.lower(right, positive, nested)?,
                );
                self.add(if positive {
                    Node::And(left, right)
                } else {
                    Node::Or(left, right)
                })
            }
            FormulaNode::Or(left, right) => {
                let (left, right) = (
                    self.lower(left, positive, nested)?,
                    self.lower(right, positive, nested)?,
                );
                self.add(if positive {
                    Node::Or(left, right)
                } else {
                    Node::And(left, right)
                })
            }
            FormulaNode::Implies(left, right) => {
                let (left, right) = (
                    self.lower(left, !positive, nested)?,
                    self.lower(right, positive, nested)?,
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
                        shaped(window.as_ref(), nested)?;
                        let operand = self.lower(operand, positive, true)?;
                        // `eventually` is `true until`; `always` is
                        // `false release`; negation swaps them.
                        let eventually = (operator == UnaryTemporal::Eventually) == positive;
                        let left = self.add(Node::Const(eventually));
                        self.add(if eventually {
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
                        })
                    }
                    UnaryTemporal::Once | UnaryTemporal::Historically => {
                        shaped(window.as_ref(), nested)?;
                        let witness = self.past(operand, nested)?;
                        // `once φ` is `true since φ`; `historically φ` is
                        // `not (true since not φ)`.
                        let tracker = if operator == UnaryTemporal::Once {
                            self.tracker(PastExpr::Const(true), witness, window, !nested)
                        } else {
                            self.tracker(
                                PastExpr::Const(true),
                                PastExpr::Not(Box::new(witness)),
                                window,
                                !nested,
                            )
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
                        shaped(window.as_ref(), nested)?;
                        let (left, right) = (
                            self.lower(left, positive, true)?,
                            self.lower(right, positive, true)?,
                        );
                        let until = (operator == BinaryTemporal::Until) == positive;
                        self.add(if until {
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
                        })
                    }
                    BinaryTemporal::Since | BinaryTemporal::Triggered => {
                        shaped(window.as_ref(), nested)?;
                        let (guard, witness) =
                            (self.past(left, nested)?, self.past(right, nested)?);
                        // `triggered` is `not (not left since not right)`.
                        let tracker = if operator == BinaryTemporal::Since {
                            self.tracker(guard, witness, window, !nested)
                        } else {
                            self.tracker(
                                PastExpr::Not(Box::new(guard)),
                                PastExpr::Not(Box::new(witness)),
                                window,
                                !nested,
                            )
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
    /// `nested` when a future operator lies above it.
    fn past(&mut self, id: FormulaNodeId, nested: bool) -> Result<PastExpr, AutomatonError> {
        let form = self.form(id)?.clone();
        Ok(match form {
            FormulaNode::Constant(value) => PastExpr::Const(value),
            FormulaNode::Holds { .. } => PastExpr::Lit(self.atom(id)?),
            FormulaNode::Aggregate { .. } => {
                return Err(AutomatonError::Invariant {
                    invariant: "timed-claim-no-aggregate",
                })
            }
            FormulaNode::Not(inner) => PastExpr::Not(Box::new(self.past(inner, nested)?)),
            FormulaNode::And(left, right) => PastExpr::And(
                Box::new(self.past(left, nested)?),
                Box::new(self.past(right, nested)?),
            ),
            FormulaNode::Or(left, right) => PastExpr::Or(
                Box::new(self.past(left, nested)?),
                Box::new(self.past(right, nested)?),
            ),
            FormulaNode::Implies(left, right) => PastExpr::Or(
                Box::new(PastExpr::Not(Box::new(self.past(left, nested)?))),
                Box::new(self.past(right, nested)?),
            ),
            FormulaNode::Unary {
                operator,
                interval,
                operand,
            } => {
                let window = window(interval.as_ref())?;
                match operator {
                    UnaryTemporal::Once | UnaryTemporal::Historically => {
                        shaped(window.as_ref(), nested)?;
                        let witness = self.past(operand, nested)?;
                        let origin = !nested;
                        if operator == UnaryTemporal::Once {
                            PastExpr::Tracker(self.tracker(
                                PastExpr::Const(true),
                                witness,
                                window,
                                origin,
                            ))
                        } else {
                            PastExpr::Not(Box::new(PastExpr::Tracker(self.tracker(
                                PastExpr::Const(true),
                                PastExpr::Not(Box::new(witness)),
                                window,
                                origin,
                            ))))
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
                        shaped(window.as_ref(), nested)?;
                        let (guard, witness) =
                            (self.past(left, nested)?, self.past(right, nested)?);
                        let origin = !nested;
                        if operator == BinaryTemporal::Since {
                            PastExpr::Tracker(self.tracker(guard, witness, window, origin))
                        } else {
                            PastExpr::Not(Box::new(PastExpr::Tracker(self.tracker(
                                PastExpr::Not(Box::new(guard)),
                                PastExpr::Not(Box::new(witness)),
                                window,
                                origin,
                            ))))
                        }
                    }
                    BinaryTemporal::Until | BinaryTemporal::Release => {
                        return Err(AutomatonError::PastOverFuture)
                    }
                }
            }
        })
    }

    fn tracker(
        &mut self,
        guard: PastExpr,
        witness: PastExpr,
        window: Option<TimedInterval>,
        origin: bool,
    ) -> usize {
        self.trackers.push(Tracker {
            guard,
            witness,
            window,
            origin,
            clock: None,
        });
        self.trackers.len() - 1
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

/// A future operator's interval: any for one read once, one starting at a
/// closed 0 for one nested under a future operator.
fn shaped(window: Option<&TimedInterval>, nested: bool) -> Result<(), AutomatonError> {
    match window {
        Some(window) if nested && !from_zero(window) => Err(AutomatonError::NestedIntervalShape),
        _ => Ok(()),
    }
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
    /// timed profile; [`AutomatonError::NestedIntervalShape`] for a nested
    /// operator whose interval does not start at a closed 0;
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
        };
        let each = matches!(clause.activation(), Activation::Each { .. });
        // Under `on each` every node is read at many positions.
        let negation = lowering.lower(formula.root(), false, each)?;
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
            lowering.add(Node::Until {
                left,
                right: violation,
                window: None,
            })
        } else {
            negation
        };
        let Lowering {
            nodes,
            mut trackers,
            atoms,
            ..
        } = lowering;
        let mut clock_count = 0;
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
                clock_count += 1;
            }
        }
        for tracker in &mut trackers {
            if tracker.window.is_some() {
                tracker.clock = Some(clock_count);
                clock_count += 1;
            }
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
                constants[clock] = window.upper().clone();
            }
        }
        for tracker in &self.trackers {
            if let (Some(clock), Some(window)) = (tracker.clock, &tracker.window) {
                constants[clock] = window.upper().clone();
            }
        }
        constants
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
                let target = Config {
                    started: true,
                    seen: case.seen.clone(),
                    pending: branch.pending.clone(),
                    fulfilled,
                };
                // Clocks of no pending instance and no remembered witness
                // read nothing until reset: reset them now, so equal
                // obligations meet in equal zones.
                let mut resets = branch.resets.clone();
                for (&node, &clock) in &self.clocks {
                    if !target.pending.contains(&node) {
                        resets.insert(clock);
                    }
                }
                for (tracker, seen) in self.trackers.iter().zip(&target.seen) {
                    if let (Some(clock), false) = (tracker.clock, seen) {
                        resets.insert(clock);
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
            resets: BTreeSet::new(),
        }];
        for (at, tracker) in self.trackers.iter().enumerate() {
            let mut next = Vec::with_capacity(cases.len());
            for case in cases {
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

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-242 for a TT-2 `BoundedWindow` clause: a claim automaton read at each
//! position's time stamp (ADR-026 EZ-9). Under a dense source the product
//! reads the stamp from one added horizon clock `h`, reset at position 0 and
//! never again; under a tick source the stamp is the period times the ticks
//! so far (FR-231 "Digital time"), which the automaton state keeps.
//!
//! The clause is read on origin. Each interval operator there is an
//! obligation whose operands are state predicates: `eventually_I φ` is
//! `true until_I φ`, `always_I φ` is `not (true until_I not φ)`, and
//! `p release_I q` is `not (not p until_I not q)`. An obligation is decided
//! as the reference evaluator's future scan decides it, in Kleene logic:
//! at each position with its stamp in `I` the witness, guarded by every
//! earlier `p`, may decide it true; a position past `I`, or a false guard,
//! decides it with what the scan found. An atom outside every interval
//! operator, and a past operator on origin, reads position 0 only. Every
//! atom is read at every position, as the evaluator reads the whole trace:
//! an undefined reading makes the clause undefined there, and one that
//! lacks a window's data makes it unknown and keeps the instance from a
//! `false` verdict. The automaton state is each instance's status of each
//! obligation, and the clause is violated where its value is false.
//!
//! Positions are the subject's steps. A behaviour that stays at a quiescent
//! state reads as the stutter step repeated after positive delays, and
//! under a tick source a terminal state's stutter step is a tick (FR-231),
//! so such a state also takes stutter positions, after which the behaviour
//! is in its idle tail and takes no further step.
//!
//! An interval operator under another temporal operator needs a clock per
//! activation: such a clause is refused (spec gap STD-493; QSL-583).

use std::collections::BTreeMap;

use qsl_eval::simulation::model::PlaceConstraint;
use qsl_eval::temporal::{
    AtomReading, InsufficientDataEvaluation, TemporalOutcome, TemporalPosition, UndefinedCause,
    UndefinedEvaluation,
};
use qsl_foundation::bound::TimedInterval;
use qsl_semantics::check::{
    BinaryTemporal, CheckedFormula, CheckedInterval, CheckedTemporalClause, ClockOp, FormulaNode,
    FormulaNodeId, TimedPropertyForm, UnaryTemporal,
};
use quire_exact::{Integer, Rational};

use super::ZoneCheckRefusal;

/// A boolean combination over a formula's atoms and obligations.
#[derive(Clone, Debug)]
enum Expr {
    Constant(bool),
    Atom(FormulaNodeId),
    Not(Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    /// An obligation's status, by index.
    Obligation(usize),
}

/// One obligation from origin.
#[derive(Clone, Debug)]
enum Obligation {
    /// `left until_I right`.
    Until {
        left: Expr,
        right: Expr,
        interval: Box<TimedInterval>,
    },
    /// A state predicate read at position 0 only: an atom outside every
    /// interval operator, or a past operator on origin.
    Origin(Expr),
}

/// A Kleene truth value, ordered `False < Unknown < True`.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Tri {
    False,
    Unknown,
    True,
}

impl Tri {
    fn of(value: bool) -> Self {
        if value {
            Self::True
        } else {
            Self::False
        }
    }

    fn negate(self) -> Self {
        match self {
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
            Self::True => Self::False,
        }
    }
}

/// An obligation's status at a position.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Status {
    /// Still open: the scan's value so far and its guard.
    Open { acc: Tri, held: Tri },
    /// Decided.
    Done(Tri),
}

/// One automaton state: whether the behaviour is in its idle tail; under a
/// tick source, the stamp, held at one value past the last interval end;
/// the status of each instance's obligations, instance-major; which
/// instances lacked a window's data; and the first instance whose reading
/// was undefined, if any.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct WindowState {
    pub tail: bool,
    stamp: Option<Rational>,
    statuses: Vec<Status>,
    insufficient: Vec<bool>,
    undefined: Option<usize>,
}

/// An undefined reading: the instance's index, the atom and its cause.
#[derive(Clone, Debug)]
pub(super) struct Undefined {
    pub instance: usize,
    pub atom: FormulaNodeId,
    pub cause: UndefinedCause,
}

/// Where a position lies on the time axis.
#[derive(Clone, Copy, Debug)]
pub(super) enum Stamp<'a> {
    /// At this time stamp: a dense source's representative of a piece of
    /// the `h` axis.
    At(&'a Rational),
    /// This much later than the state's own stamp: under a tick source, a
    /// tick step's period, or 0.
    After(&'a Rational),
}

/// What a position reads: its automaton state, or a stop.
pub(super) enum Read {
    State(usize),
    Fault(&'static str),
    Atom(String),
    AutomatonStates,
}

/// The reading of atom `atom` for the instance of index `instance` at the
/// position being read.
pub(super) type Atoms<'f> = dyn FnMut(usize, FormulaNodeId) -> Result<AtomReading, String> + 'f;

/// FR-242's TT-2 automaton for one clause, with the states it has reached.
pub(super) struct WindowClaim {
    instances: usize,
    /// The clause's value over its obligations.
    root: Expr,
    obligations: Vec<Obligation>,
    /// The formula's atoms, in clause-node order.
    atoms: Vec<FormulaNodeId>,
    /// Every distinct interval end, ascending.
    breakpoints: Vec<Rational>,
    /// Under a tick source, the period; `None` under a dense source.
    period: Option<Rational>,
    states: Vec<WindowState>,
    index: BTreeMap<WindowState, usize>,
    undefined: Vec<Option<Undefined>>,
    max_states: u64,
}

impl WindowClaim {
    /// The automaton of `formula` over `instances` instances; `period` is a
    /// tick source's.
    ///
    /// # Errors
    ///
    /// [`ZoneCheckRefusal::NestedWindow`] for an interval operator under
    /// another temporal operator.
    pub(super) fn new(
        formula: &CheckedFormula,
        instances: usize,
        period: Option<Rational>,
        max_states: u64,
    ) -> Result<Self, ZoneCheckRefusal> {
        let mut obligations = Vec::new();
        let root = top(formula, formula.root(), &mut obligations)?;
        let mut breakpoints: Vec<Rational> = obligations
            .iter()
            .filter_map(|obligation| match obligation {
                Obligation::Until { interval, .. } => {
                    Some([interval.lower().clone(), interval.upper().clone()])
                }
                Obligation::Origin(_) => None,
            })
            .flatten()
            .collect();
        breakpoints.sort();
        breakpoints.dedup();
        let atoms = formula
            .nodes()
            .iter()
            .enumerate()
            .filter(|(_, node)| matches!(node, FormulaNode::Holds { .. }))
            .filter_map(|(index, _)| formula.node_id(index))
            .collect();
        Ok(Self {
            instances,
            root,
            obligations,
            atoms,
            breakpoints,
            period,
            states: Vec::new(),
            index: BTreeMap::new(),
            undefined: Vec::new(),
            max_states,
        })
    }

    /// Every constant the automaton compares `h` with, which joins the
    /// scale and `h`'s `L` and `U`.
    pub(super) fn constants(&self) -> &[Rational] {
        &self.breakpoints
    }

    /// The automaton states reached.
    pub(super) fn state_count(&self) -> usize {
        self.states.len()
    }

    /// Automaton state `q`.
    pub(super) fn state(&self, q: usize) -> Option<&WindowState> {
        self.states.get(q)
    }

    /// The undefined reading that made `q` bad, if any.
    pub(super) fn undefined(&self, q: usize) -> Option<&Undefined> {
        self.undefined.get(q).and_then(Option::as_ref)
    }

    /// The index of the first instance for which `q` falsifies the clause:
    /// its value is false and it read every window's data.
    pub(super) fn violated(&self, q: usize) -> Option<usize> {
        let state = self.states.get(q)?;
        (0..self.instances).find(|&at| {
            self.value(state, at) == Tri::False
                && !state.insufficient.get(at).copied().unwrap_or(true)
        })
    }

    /// The index of the first instance that read a window short of data at
    /// or before `q` (ADR-024 WA-4): the evaluator reports such a trace
    /// `InsufficientData` whatever the clause's value, unless a reading is
    /// undefined.
    pub(super) fn insufficient(&self, q: usize) -> Option<usize> {
        let state = self.states.get(q)?;
        if state.undefined.is_some() {
            return None;
        }
        state.insufficient.iter().position(|&short| short)
    }

    /// Whether every obligation of every instance is decided at `q`, so no
    /// later position changes the clause: FR-242 stops the path there.
    pub(super) fn decided(&self, q: usize) -> bool {
        self.states.get(q).is_some_and(|state| {
            state.undefined.is_some()
                || state
                    .statuses
                    .iter()
                    .all(|status| matches!(status, Status::Done(_)))
        })
    }

    /// The clause's value for instance `at` in `state`; an open obligation
    /// reads unknown.
    fn value(&self, state: &WindowState, at: usize) -> Tri {
        let count = self.obligations.len();
        state
            .statuses
            .get(at * count..(at + 1) * count)
            .map_or(Tri::Unknown, |statuses| read_root(&self.root, statuses))
    }

    /// The pieces of the `h` axis on which every obligation decides alike:
    /// each breakpoint, and each open stretch between and around them, as
    /// constraints on the place `h` with a representative time stamp.
    pub(super) fn pieces(&self, h: usize) -> Vec<(Vec<PlaceConstraint>, Rational)> {
        let zero = zero();
        let two = Rational::from_integer(Integer::one().add(&Integer::one()));
        let half = |value: &Rational| value.div(&two).unwrap_or_else(|| value.clone());
        let mut pieces = Vec::new();
        let mut previous: Option<&Rational> = None;
        for point in &self.breakpoints {
            match previous {
                None if point > &zero => pieces.push((
                    vec![PlaceConstraint::new(h, ClockOp::Less, point.clone())],
                    half(point),
                )),
                None => {}
                Some(low) => pieces.push((
                    vec![
                        PlaceConstraint::new(h, ClockOp::Greater, low.clone()),
                        PlaceConstraint::new(h, ClockOp::Less, point.clone()),
                    ],
                    half(&low.add(point)),
                )),
            }
            pieces.push((
                vec![PlaceConstraint::new(h, ClockOp::Equal, point.clone())],
                point.clone(),
            ));
            previous = Some(point);
        }
        match previous {
            Some(last) => pieces.push((
                vec![PlaceConstraint::new(h, ClockOp::Greater, last.clone())],
                last.add(&Rational::from_integer(Integer::one())),
            )),
            None => pieces.push((Vec::new(), zero)),
        }
        pieces
    }

    /// The state position 0 reads, at time 0.
    pub(super) fn initial(&mut self, atoms: &mut Atoms<'_>) -> Read {
        let open = WindowState {
            tail: false,
            stamp: self.period.as_ref().map(|_| zero()),
            statuses: vec![
                Status::Open {
                    acc: Tri::False,
                    held: Tri::True,
                };
                self.obligations.len() * self.instances
            ],
            insufficient: vec![false; self.instances],
            undefined: None,
        };
        self.position(open, &zero(), true, false, atoms)
    }

    /// The state a position at `stamp` reads from automaton state `from`;
    /// `stutter` for an idle-tail position.
    pub(super) fn step(
        &mut self,
        from: usize,
        stamp: Stamp<'_>,
        stutter: bool,
        atoms: &mut Atoms<'_>,
    ) -> Read {
        let Some(mut previous) = self.states.get(from).cloned() else {
            return Read::Fault("window-state");
        };
        let at = match (stamp, &previous.stamp) {
            (Stamp::At(stamp), None) => stamp.clone(),
            (Stamp::After(advance), Some(own)) => {
                let next = own.add(advance);
                // Past the last interval end every stamp reads alike, so
                // the state keeps one: the last end plus the period.
                let ceiling = match (self.breakpoints.last(), &self.period) {
                    (Some(last), Some(period)) => Some(last.add(period)),
                    _ => None,
                };
                match ceiling {
                    Some(ceiling) if next > ceiling => ceiling,
                    _ => next,
                }
            }
            _ => return Read::Fault("window-stamp"),
        };
        if previous.stamp.is_some() {
            previous.stamp = Some(at.clone());
        }
        self.position(previous, &at, false, stutter, atoms)
    }

    fn position(
        &mut self,
        previous: WindowState,
        stamp: &Rational,
        first: bool,
        stutter: bool,
        atoms: &mut Atoms<'_>,
    ) -> Read {
        let count = self.obligations.len();
        let mut next = WindowState {
            tail: previous.tail || stutter,
            ..previous
        };
        let mut undefined = None;
        if next.undefined.is_none() {
            for at in 0..self.instances {
                // Every atom at this position, in clause-node order, as the
                // evaluator reads them; the first undefined one stops.
                let mut values = BTreeMap::new();
                for &atom in &self.atoms {
                    let value = match atoms(at, atom) {
                        Ok(AtomReading::True) => Tri::True,
                        Ok(AtomReading::False) => Tri::False,
                        Ok(AtomReading::Undefined(UndefinedCause::InsufficientData(_))) => {
                            next.insufficient[at] = true;
                            Tri::Unknown
                        }
                        Ok(AtomReading::Undefined(cause)) => {
                            undefined = Some(Undefined {
                                instance: at,
                                atom,
                                cause,
                            });
                            break;
                        }
                        Err(stop) => return Read::Atom(stop),
                    };
                    values.insert(atom, value);
                }
                if undefined.is_some() {
                    next.undefined = Some(at);
                    break;
                }
                for (index, obligation) in self.obligations.iter().enumerate() {
                    let slot = at * count + index;
                    let Status::Open { mut acc, mut held } = next.statuses[slot] else {
                        continue;
                    };
                    next.statuses[slot] = match obligation {
                        Obligation::Origin(expr) if first => Status::Done(read(expr, &values)),
                        Obligation::Origin(_) => next.statuses[slot],
                        Obligation::Until {
                            left,
                            right,
                            interval,
                        } => {
                            if !interval.reaches(stamp) {
                                Status::Done(acc)
                            } else {
                                if interval.contains(stamp) {
                                    acc = acc.max(read(right, &values).min(held));
                                }
                                held = held.min(read(left, &values));
                                if acc == Tri::True || held == Tri::False {
                                    Status::Done(acc)
                                } else {
                                    Status::Open { acc, held }
                                }
                            }
                        }
                    };
                }
            }
        }
        if let Some(&q) = self.index.get(&next) {
            return Read::State(q);
        }
        if u64::try_from(self.states.len()).unwrap_or(u64::MAX) >= self.max_states {
            return Read::AutomatonStates;
        }
        let q = self.states.len();
        self.index.insert(next.clone(), q);
        self.states.push(next);
        self.undefined.push(undefined);
        Read::State(q)
    }
}

/// FR-242's TT-2 translation read on one finite timed trace, as the
/// reference evaluator `evaluate_timed_at` reads it at the origin: the
/// positions have the time stamps `stamps` (non-decreasing) and
/// `atoms(atom, position)` gives each atom's reading. Every atom is read at
/// every position. A trace that ends while an obligation is open, its
/// window still reachable, leaves it unknown, as the evaluator's scan past
/// the trace does.
///
/// # Errors
///
/// [`ZoneCheckRefusal::UnsupportedForm`] for a clause that is not a
/// `BoundedWindow`, [`ZoneCheckRefusal::NestedWindow`] as the engine
/// refuses it.
#[doc(hidden)]
pub fn bounded_window_verdict(
    clause: &CheckedTemporalClause,
    stamps: &[Rational],
    atoms: &mut dyn FnMut(FormulaNodeId, usize) -> AtomReading,
) -> Result<TemporalOutcome, ZoneCheckRefusal> {
    if !matches!(
        clause.timed_form(),
        Some(TimedPropertyForm::BoundedWindow { .. })
    ) {
        return Err(ZoneCheckRefusal::UnsupportedForm);
    }
    let mut window = WindowClaim::new(clause.formula(), 1, None, u64::MAX)?;
    let mut q = None;
    let mut undefined = None;
    let mut insufficient = None;
    for (position, stamp) in stamps.iter().enumerate() {
        let at = TemporalPosition::new(u64::try_from(position).unwrap_or(u64::MAX));
        let mut read = |_: usize, atom: FormulaNodeId| {
            let reading = atoms(atom, position);
            if let AtomReading::Undefined(UndefinedCause::InsufficientData(data)) = &reading {
                if insufficient.is_none() {
                    insufficient = Some(InsufficientDataEvaluation {
                        at,
                        atom,
                        data: *data,
                    });
                }
            }
            Ok(reading)
        };
        let read = match q {
            None => window.initial(&mut read),
            Some(from) => window.step(from, Stamp::At(stamp), false, &mut read),
        };
        q = Some(match read {
            Read::State(next) => next,
            Read::Fault(invariant) => return Err(ZoneCheckRefusal::Fault(invariant)),
            Read::Atom(stop) => return Err(ZoneCheckRefusal::Atom(stop)),
            Read::AutomatonStates => return Err(ZoneCheckRefusal::Fault("automaton-states")),
        });
        if let Some(found) = q.and_then(|q| window.undefined(q)) {
            undefined.get_or_insert_with(|| UndefinedEvaluation {
                at,
                cause: found.cause.clone(),
            });
            break;
        }
    }
    let Some(q) = q else {
        return Ok(TemporalOutcome::Pending);
    };
    if let Some(undefined) = undefined {
        return Ok(TemporalOutcome::Undefined(undefined));
    }
    let state = window
        .state(q)
        .ok_or(ZoneCheckRefusal::Fault("window-state"))?;
    Ok(match (window.value(state, 0), insufficient) {
        (_, Some(insufficient)) => TemporalOutcome::InsufficientData(insufficient),
        (Tri::False, None) => TemporalOutcome::Violated {
            at: TemporalPosition::new(0),
        },
        (Tri::True, None) => TemporalOutcome::Holds,
        (Tri::Unknown, None) => TemporalOutcome::Pending,
    })
}

/// A state predicate's value under the position's atom `values`.
fn read(expr: &Expr, values: &BTreeMap<FormulaNodeId, Tri>) -> Tri {
    match expr {
        Expr::Constant(value) => Tri::of(*value),
        Expr::Atom(atom) => values.get(atom).copied().unwrap_or(Tri::Unknown),
        Expr::Not(inner) => read(inner, values).negate(),
        Expr::And(left, right) => read(left, values).min(read(right, values)),
        Expr::Or(left, right) => read(left, values).max(read(right, values)),
        Expr::Obligation(_) => Tri::Unknown,
    }
}

/// The root's value under one instance's `statuses`; an open obligation
/// reads unknown.
fn read_root(expr: &Expr, statuses: &[Status]) -> Tri {
    match expr {
        Expr::Constant(value) => Tri::of(*value),
        Expr::Atom(_) => Tri::Unknown,
        Expr::Not(inner) => read_root(inner, statuses).negate(),
        Expr::And(left, right) => read_root(left, statuses).min(read_root(right, statuses)),
        Expr::Or(left, right) => read_root(left, statuses).max(read_root(right, statuses)),
        Expr::Obligation(index) => match statuses.get(*index) {
            Some(Status::Done(value)) => *value,
            _ => Tri::Unknown,
        },
    }
}

/// The root of a `BoundedWindow` formula on origin, with its interval
/// operators gathered as obligations.
fn top(
    formula: &CheckedFormula,
    node: FormulaNodeId,
    obligations: &mut Vec<Obligation>,
) -> Result<Expr, ZoneCheckRefusal> {
    let Some(form) = formula.nodes().get(node.index()) else {
        return Err(ZoneCheckRefusal::Fault("formula-node"));
    };
    let mut recurse = |child: FormulaNodeId| top(formula, child, obligations);
    Ok(match form {
        FormulaNode::Constant(value) => Expr::Constant(*value),
        FormulaNode::Holds { .. } => origin(obligations, Expr::Atom(node)),
        FormulaNode::Aggregate { .. } => return Err(ZoneCheckRefusal::UnsupportedForm),
        FormulaNode::Not(inner) => Expr::Not(Box::new(recurse(*inner)?)),
        FormulaNode::And(left, right) => {
            Expr::And(Box::new(recurse(*left)?), Box::new(recurse(*right)?))
        }
        FormulaNode::Or(left, right) => {
            Expr::Or(Box::new(recurse(*left)?), Box::new(recurse(*right)?))
        }
        FormulaNode::Implies(left, right) => Expr::Or(
            Box::new(Expr::Not(Box::new(recurse(*left)?))),
            Box::new(recurse(*right)?),
        ),
        FormulaNode::Unary {
            operator,
            interval,
            operand,
        } => {
            let interval = timed(interval.as_ref())?;
            let operand = state(formula, *operand)?;
            let truth = Expr::Constant(true);
            match operator {
                UnaryTemporal::Eventually => obligation(obligations, truth, operand, interval),
                UnaryTemporal::Always => Expr::Not(Box::new(obligation(
                    obligations,
                    truth,
                    Expr::Not(Box::new(operand)),
                    interval,
                ))),
                // On origin a past operator reads position 0, at offset 0.
                UnaryTemporal::Once => {
                    if interval.contains(&zero()) {
                        origin(obligations, operand)
                    } else {
                        Expr::Constant(false)
                    }
                }
                UnaryTemporal::Historically => {
                    if interval.contains(&zero()) {
                        origin(obligations, operand)
                    } else {
                        Expr::Constant(true)
                    }
                }
            }
        }
        FormulaNode::Binary {
            operator,
            interval,
            left,
            right,
        } => {
            let interval = timed(interval.as_ref())?;
            let left = state(formula, *left)?;
            let right = state(formula, *right)?;
            match operator {
                BinaryTemporal::Until => obligation(obligations, left, right, interval),
                BinaryTemporal::Release => Expr::Not(Box::new(obligation(
                    obligations,
                    Expr::Not(Box::new(left)),
                    Expr::Not(Box::new(right)),
                    interval,
                ))),
                BinaryTemporal::Since => {
                    if interval.contains(&zero()) {
                        origin(obligations, right)
                    } else {
                        Expr::Constant(false)
                    }
                }
                BinaryTemporal::Triggered => {
                    if interval.contains(&zero()) {
                        origin(obligations, right)
                    } else {
                        Expr::Constant(true)
                    }
                }
            }
        }
    })
}

fn obligation(
    obligations: &mut Vec<Obligation>,
    left: Expr,
    right: Expr,
    interval: TimedInterval,
) -> Expr {
    obligations.push(Obligation::Until {
        left,
        right,
        interval: Box::new(interval),
    });
    Expr::Obligation(obligations.len() - 1)
}

fn origin(obligations: &mut Vec<Obligation>, expr: Expr) -> Expr {
    obligations.push(Obligation::Origin(expr));
    Expr::Obligation(obligations.len() - 1)
}

/// A state predicate: a boolean combination of atoms with no temporal
/// operator.
fn state(formula: &CheckedFormula, node: FormulaNodeId) -> Result<Expr, ZoneCheckRefusal> {
    let Some(form) = formula.nodes().get(node.index()) else {
        return Err(ZoneCheckRefusal::Fault("formula-node"));
    };
    let recurse = |child: FormulaNodeId| state(formula, child);
    Ok(match form {
        FormulaNode::Constant(value) => Expr::Constant(*value),
        FormulaNode::Holds { .. } => Expr::Atom(node),
        FormulaNode::Aggregate { .. } => return Err(ZoneCheckRefusal::UnsupportedForm),
        FormulaNode::Not(inner) => Expr::Not(Box::new(recurse(*inner)?)),
        FormulaNode::And(left, right) => {
            Expr::And(Box::new(recurse(*left)?), Box::new(recurse(*right)?))
        }
        FormulaNode::Or(left, right) => {
            Expr::Or(Box::new(recurse(*left)?), Box::new(recurse(*right)?))
        }
        FormulaNode::Implies(left, right) => Expr::Or(
            Box::new(Expr::Not(Box::new(recurse(*left)?))),
            Box::new(recurse(*right)?),
        ),
        FormulaNode::Unary { .. } | FormulaNode::Binary { .. } => {
            return Err(ZoneCheckRefusal::NestedWindow)
        }
    })
}

fn timed(interval: Option<&CheckedInterval>) -> Result<TimedInterval, ZoneCheckRefusal> {
    match interval {
        Some(CheckedInterval::Timed(interval)) => Ok(interval.clone()),
        _ => Err(ZoneCheckRefusal::UnsupportedForm),
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

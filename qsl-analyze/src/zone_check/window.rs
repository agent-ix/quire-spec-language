// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-242 for a TT-2 `BoundedWindow` clause: a claim automaton with one
//! added horizon clock `h`, reset at position 0 and never again, so `h`
//! reads each position's time stamp (ADR-026 EZ-9).
//!
//! The clause is read on origin. Each interval operator there is an
//! obligation whose operands are state predicates: `eventually_I φ` is
//! `true until_I φ`, `always_I φ` is `not (true until_I not φ)`, and
//! `p release_I q` is `not (not p until_I not q)`. `p until_I q` stays open
//! while `p` holds and no position with `h` in `I` has had `q`; a position
//! with `h` in `I` and `q` decides it true, and one with `not p`, or past
//! `I`, false. An atom outside every interval operator, and a past operator
//! on origin, reads position 0 only. The automaton state is each instance's
//! status of each obligation, and the clause is violated where its boolean
//! combination of them is false.
//!
//! Positions are the subject's steps; a behaviour that stays at a quiescent
//! state reads as the stutter step repeated after positive delays (FR-231),
//! so a quiescent state also takes stutter positions, after which the
//! behaviour is in its idle tail and takes no further step.
//!
//! An interval operator under another temporal operator needs a clock per
//! activation, which one horizon clock does not give: such a clause is
//! refused (spec gap STD-493).

use std::collections::BTreeMap;

use qsl_eval::simulation::model::PlaceConstraint;
use qsl_eval::temporal::{AtomReading, UndefinedCause};
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

/// An obligation's status at a position.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum Status {
    Open,
    True,
    False,
}

/// One automaton state: whether the behaviour is in its idle tail, the
/// status of each instance's obligations, instance-major, and the first
/// instance whose reading was undefined, if any.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct WindowState {
    pub tail: bool,
    statuses: Vec<Status>,
    undefined: Option<usize>,
}

/// An undefined reading: the instance's index, the atom and its cause.
#[derive(Clone, Debug)]
pub(super) struct Undefined {
    pub instance: usize,
    pub atom: FormulaNodeId,
    pub cause: UndefinedCause,
}

/// Which position the automaton reads.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Position {
    /// Position 0.
    First,
    /// A step's position.
    Step,
    /// An idle-tail stutter position.
    Stutter,
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
    /// Every distinct interval end, ascending.
    breakpoints: Vec<Rational>,
    states: Vec<WindowState>,
    index: BTreeMap<WindowState, usize>,
    undefined: Vec<Option<Undefined>>,
    max_states: u64,
}

impl WindowClaim {
    /// The automaton of `formula` over `instances` instances.
    ///
    /// # Errors
    ///
    /// [`ZoneCheckRefusal::NestedWindow`] for an interval operator under
    /// another temporal operator.
    pub(super) fn new(
        formula: &CheckedFormula,
        instances: usize,
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
        Ok(Self {
            instances,
            root,
            obligations,
            breakpoints,
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

    /// The index of the first instance for which `q` falsifies the clause.
    pub(super) fn violated(&self, q: usize) -> Option<usize> {
        self.value(q).iter().position(|value| *value == Some(false))
    }

    /// The clause's value for each instance at `q`, `None` while open.
    fn value(&self, q: usize) -> Vec<Option<bool>> {
        let count = self.obligations.len();
        let Some(state) = self.states.get(q) else {
            return Vec::new();
        };
        (0..self.instances)
            .map(|at| {
                state
                    .statuses
                    .get(at * count..(at + 1) * count)
                    .and_then(|statuses| read_root(&self.root, statuses))
            })
            .collect()
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
            statuses: vec![Status::Open; self.obligations.len() * self.instances],
            undefined: None,
        };
        self.position(open, &zero(), Position::First, atoms)
    }

    /// The state a position at time stamp `stamp` reads from automaton
    /// state `from`; `stutter` for an idle-tail position.
    pub(super) fn step(
        &mut self,
        from: usize,
        stamp: &Rational,
        stutter: bool,
        atoms: &mut Atoms<'_>,
    ) -> Read {
        let Some(previous) = self.states.get(from).cloned() else {
            return Read::Fault("window-state");
        };
        let kind = if stutter {
            Position::Stutter
        } else {
            Position::Step
        };
        self.position(previous, stamp, kind, atoms)
    }

    fn position(
        &mut self,
        previous: WindowState,
        stamp: &Rational,
        kind: Position,
        atoms: &mut Atoms<'_>,
    ) -> Read {
        let count = self.obligations.len();
        let mut next = WindowState {
            tail: previous.tail || kind == Position::Stutter,
            statuses: previous.statuses,
            undefined: previous.undefined,
        };
        let mut undefined = None;
        if next.undefined.is_none() {
            for at in 0..self.instances {
                let mut reader = Reader {
                    atoms: &mut *atoms,
                    instance: at,
                    undefined: None,
                    stop: None,
                };
                for (index, obligation) in self.obligations.iter().enumerate() {
                    let slot = at * count + index;
                    if next.statuses[slot] != Status::Open {
                        continue;
                    }
                    next.statuses[slot] = match obligation {
                        Obligation::Origin(expr) if kind == Position::First => {
                            match reader.read(expr) {
                                Some(true) => Status::True,
                                Some(false) => Status::False,
                                None => Status::Open,
                            }
                        }
                        Obligation::Origin(_) => Status::Open,
                        Obligation::Until {
                            left,
                            right,
                            interval,
                        } => {
                            let right = reader.read(right);
                            let left = reader.read(left);
                            if interval.contains(stamp) && right == Some(true) {
                                Status::True
                            } else if !interval.reaches(stamp) || left == Some(false) {
                                Status::False
                            } else {
                                Status::Open
                            }
                        }
                    };
                }
                if let Some(stop) = reader.stop {
                    return Read::Atom(stop);
                }
                if let Some((atom, cause)) = reader.undefined {
                    next.undefined = Some(at);
                    undefined = Some(Undefined {
                        instance: at,
                        atom,
                        cause,
                    });
                    break;
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

/// FR-242's TT-2 translation read on one finite timed trace: the clause's
/// value on origin, decided by the automaton over the positions with time
/// stamps `stamps` (non-decreasing), `atoms(atom, position)` giving each
/// atom's reading. `Some(false)` exactly where the automaton accepts, that
/// is, rejects the clause; `None` while the trace ends before every
/// obligation is decided. A trace that reaches past every interval decides
/// the clause.
///
/// # Errors
///
/// [`ZoneCheckRefusal::UnsupportedForm`] for a clause that is not a
/// `BoundedWindow`, [`ZoneCheckRefusal::NestedWindow`] as the engine
/// refuses it, and [`ZoneCheckRefusal::Atom`] for an undefined or stopped
/// atom.
pub fn bounded_window_verdict(
    clause: &CheckedTemporalClause,
    stamps: &[Rational],
    atoms: &mut dyn FnMut(FormulaNodeId, usize) -> AtomReading,
) -> Result<Option<bool>, ZoneCheckRefusal> {
    if !matches!(
        clause.timed_form(),
        Some(TimedPropertyForm::BoundedWindow { .. })
    ) {
        return Err(ZoneCheckRefusal::UnsupportedForm);
    }
    let mut window = WindowClaim::new(clause.formula(), 1, u64::MAX)?;
    let mut q = None;
    for (position, stamp) in stamps.iter().enumerate() {
        let mut read = |_: usize, atom: FormulaNodeId| Ok(atoms(atom, position));
        let read = match q {
            None => window.initial(&mut read),
            Some(from) => window.step(from, stamp, false, &mut read),
        };
        q = Some(match read {
            Read::State(next) => next,
            Read::Fault(invariant) => return Err(ZoneCheckRefusal::Fault(invariant)),
            Read::Atom(stop) => return Err(ZoneCheckRefusal::Atom(stop)),
            Read::AutomatonStates => return Err(ZoneCheckRefusal::Fault("automaton-states")),
        });
    }
    let Some(q) = q else {
        return Ok(None);
    };
    if let Some(undefined) = window.undefined(q) {
        return Err(ZoneCheckRefusal::Atom(format!("{:?}", undefined.cause)));
    }
    Ok(window.value(q).first().copied().flatten())
}

/// Reads state predicates at one instance and position.
struct Reader<'a, 'f> {
    atoms: &'a mut Atoms<'f>,
    instance: usize,
    undefined: Option<(FormulaNodeId, UndefinedCause)>,
    stop: Option<String>,
}

impl Reader<'_, '_> {
    /// The value of a state predicate; `None` once undefined or stopped.
    fn read(&mut self, expr: &Expr) -> Option<bool> {
        match expr {
            Expr::Constant(value) => Some(*value),
            Expr::Atom(atom) => self.atom(*atom),
            Expr::Not(inner) => self.read(inner).map(|value| !value),
            Expr::And(left, right) => {
                let left = self.read(left);
                let right = self.read(right);
                kleene_and(left, right)
            }
            Expr::Or(left, right) => {
                let left = self.read(left);
                let right = self.read(right);
                kleene_or(left, right)
            }
            Expr::Obligation(_) => None,
        }
    }

    fn atom(&mut self, atom: FormulaNodeId) -> Option<bool> {
        if self.undefined.is_some() || self.stop.is_some() {
            return None;
        }
        match (self.atoms)(self.instance, atom) {
            Ok(AtomReading::True) => Some(true),
            Ok(AtomReading::False) => Some(false),
            Ok(AtomReading::Undefined(cause)) => {
                self.undefined = Some((atom, cause));
                None
            }
            Err(stop) => {
                self.stop = Some(stop);
                None
            }
        }
    }
}

/// The root's value under one instance's `statuses`, `None` while open.
fn read_root(expr: &Expr, statuses: &[Status]) -> Option<bool> {
    match expr {
        Expr::Constant(value) => Some(*value),
        Expr::Atom(_) => None,
        Expr::Not(inner) => read_root(inner, statuses).map(|value| !value),
        Expr::And(left, right) => kleene_and(read_root(left, statuses), read_root(right, statuses)),
        Expr::Or(left, right) => kleene_or(read_root(left, statuses), read_root(right, statuses)),
        Expr::Obligation(index) => match statuses.get(*index) {
            Some(Status::True) => Some(true),
            Some(Status::False) => Some(false),
            _ => None,
        },
    }
}

fn kleene_and(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(false), _) | (_, Some(false)) => Some(false),
        (Some(true), Some(true)) => Some(true),
        _ => None,
    }
}

fn kleene_or(left: Option<bool>, right: Option<bool>) -> Option<bool> {
    match (left, right) {
        (Some(true), _) | (_, Some(true)) => Some(true),
        (Some(false), Some(false)) => Some(false),
        _ => None,
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

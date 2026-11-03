// SPDX-License-Identifier: AGPL-3.0-or-later
//! The finite-window evaluator (FR-501): demand each activation's node
//! visits, charging the meter and reading the letters, then lift the formula
//! over the possibility sets and classify the result.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use qsl_semantics::check::{Activation, CaptureSite, CheckedTemporalClause, TemporalProfile};
use quire_exact::{Charge, ChargePoint, Incomplete, Meter};
use quire_semantic_value::catalog::InternalFault;

use super::identity::Preimage;
use super::lift::{Candidate, Cell, Lifter, Part, Poss, ScanFrame, Variant};
use super::outcome::{
    Assessment, Basis, IndeterminateReason, Premises, SupportPair, Valuation, WindowEvaluation,
    WindowVerdict,
};
use super::trace::{LowerBoundary, TimestampedTrace};
use crate::temporal::letters::{
    AtomReading, CaptureReading, LetterStop, TemporalLetters, UndefinedCause,
};
use crate::temporal::outcome::{TemporalRefusal, UndefinedEvaluation};
use crate::temporal::plan::{fault, slot, Direction, Node, Plan, Window};
use crate::temporal::position::TemporalPosition;

const VISIT: ChargePoint = ChargePoint::TemporalVisit;

/// Why a run stopped before it finished every activation.
enum Stop {
    Incomplete(Incomplete),
    Undefined(UndefinedEvaluation),
    Refused(TemporalRefusal),
}

impl From<Incomplete> for Stop {
    fn from(incomplete: Incomplete) -> Self {
        Self::Incomplete(incomplete)
    }
}

impl From<InternalFault> for Stop {
    fn from(internal: InternalFault) -> Self {
        Self::Refused(TemporalRefusal::Fault(internal))
    }
}

impl From<LetterStop> for Stop {
    fn from(stop: LetterStop) -> Self {
        match stop {
            LetterStop::Incomplete(incomplete) => Self::Incomplete(incomplete),
            LetterStop::Window(refusal) => Self::Refused(TemporalRefusal::Window(refusal)),
            LetterStop::WindowLimit(_) => {
                Self::Refused(TemporalRefusal::Fault(fault("window-aggregate-not-read")))
            }
            LetterStop::Fault(internal) => Self::Refused(TemporalRefusal::Fault(internal)),
        }
    }
}

/// How an activation ended.
#[derive(Clone, Debug, Eq, PartialEq)]
enum Ending {
    True,
    False,
    Pending { deadline: u64 },
    Indeterminate(IndeterminateReason),
}

struct Finished {
    anchor: usize,
    result: Ending,
    closure: Vec<(usize, usize)>,
    established: bool,
    poss: Poss,
}

/// What ended the activations early, if anything did.
enum Tail {
    Done,
    Undefined(UndefinedEvaluation),
    Incomplete(Incomplete),
}

struct Run<'a, L: TemporalLetters> {
    clause: &'a CheckedTemporalClause,
    trace: &'a TimestampedTrace,
    plan: Plan,
    /// Each node's future reach, in ticks: the greatest sum of the upper
    /// bounds of future operators along a nesting path below it.
    reach: Vec<i128>,
    stamps: Vec<i128>,
    margin: i128,
    watermark: i128,
    letters: &'a mut L,
    meter: &'a mut Meter,
    visited: HashSet<(usize, usize)>,
    cells: HashMap<(usize, usize), Cell>,
    undefined: BTreeMap<(usize, usize), UndefinedCause>,
    frames: HashMap<(usize, usize), ScanFrame>,
}

/// Evaluate `clause`, selected under
/// `quire.temporal.timestamped-event.finite-window/v1`, over the timestamped
/// `trace` (FR-501).
///
/// A definite `Holds` or `Violated` is returned whenever the window is
/// closed and the evidence through it is complete, or a decisive witness or
/// counterexample every admissible continuation preserves exists;
/// `Pending` and `Indeterminate` only for an obligation that is genuinely
/// unresolved. The charge count depends on the trace and its progress, not on
/// the valuations.
///
/// # Errors
///
/// A [`TemporalRefusal`] for a clause under another profile, equal stamps of
/// positions that take part in an order-sensitive operator without a
/// comparable order key, or a letter that fails.
pub fn evaluate_finite_window<L: TemporalLetters>(
    clause: &CheckedTemporalClause,
    trace: &TimestampedTrace,
    letters: &mut L,
    meter: &mut Meter,
) -> Result<WindowEvaluation, TemporalRefusal> {
    if clause.profile() != TemporalProfile::TimestampedEventFiniteWindow {
        return Err(TemporalRefusal::NotFiniteWindowProfile {
            profile: clause.profile(),
        });
    }
    let plan = Plan::new(clause.formula()).map_err(TemporalRefusal::Fault)?;
    let reach = reaches(&plan).map_err(TemporalRefusal::Fault)?;
    let mut run = Run {
        clause,
        trace,
        plan,
        reach,
        stamps: trace
            .positions
            .iter()
            .map(|position| i128::from(position.stamp))
            .collect(),
        margin: trace.clock.cover.margin(),
        watermark: trace.progress.watermark.inclusive(),
        letters,
        meter,
        visited: HashSet::new(),
        cells: HashMap::new(),
        undefined: BTreeMap::new(),
        frames: HashMap::new(),
    };
    let mut done = Vec::new();
    let tail = match run.activations(&mut done) {
        Ok(()) => Tail::Done,
        Err(Stop::Incomplete(incomplete)) => Tail::Incomplete(incomplete),
        Err(Stop::Undefined(undefined)) => Tail::Undefined(undefined),
        Err(Stop::Refused(refusal)) => return Err(refusal),
    };
    run.settle(done, tail).map_err(TemporalRefusal::Fault)
}

/// The future reach of every node.
fn reaches(plan: &Plan) -> Result<Vec<i128>, InternalFault> {
    let mut reach = vec![0_i128; plan.nodes.len()];
    for index in (0..plan.nodes.len()).rev() {
        let node = slot(&plan.nodes, index)?;
        let operands: [Option<usize>; 2] = match node {
            Node::Constant(_) | Node::Atom => [None, None],
            Node::Not(operand) => [Some(operand), None],
            Node::And(left, right) | Node::Or(left, right) | Node::Implies(left, right) => {
                [Some(left), Some(right)]
            }
            Node::Scan(scan) => [scan.guard, Some(scan.witness)],
        };
        let mut deepest = 0_i128;
        for operand in operands.into_iter().flatten() {
            if operand <= index {
                return Err(fault("operands-follow-operator"));
            }
            deepest = deepest.max(
                *reach
                    .get(operand)
                    .ok_or_else(|| fault("plan-index-in-arena"))?,
            );
        }
        let own = match node {
            Node::Scan(scan) => match (scan.base.direction(), scan.window) {
                (Direction::Future, Window::Closed { upper, .. }) => i128::from(upper),
                (Direction::Past, Window::Closed { .. }) => 0,
                (_, Window::From { .. }) => return Err(fault("finite-window-interval-is-bounded")),
            },
            Node::Constant(_)
            | Node::Atom
            | Node::Not(_)
            | Node::And(..)
            | Node::Or(..)
            | Node::Implies(..) => 0,
        };
        *reach
            .get_mut(index)
            .ok_or_else(|| fault("plan-index-in-arena"))? = deepest.saturating_add(own);
    }
    Ok(reach)
}

fn position_of(index: usize) -> Result<TemporalPosition, InternalFault> {
    u64::try_from(index)
        .map(TemporalPosition::new)
        .map_err(|_| fault("position-in-u64"))
}

impl<L: TemporalLetters> Run<'_, L> {
    fn charge(&mut self) -> Result<(), Stop> {
        self.meter.charge(Charge::new(VISIT))?;
        Ok(())
    }

    fn stamp(&self, position: usize) -> Result<i128, InternalFault> {
        slot(&self.stamps, position)
    }

    /// Whether the window of an activation (or sub-instance) at `tick`
    /// reaching `reach` ticks ahead is established (FR-501 "Established
    /// window").
    fn established(&self, tick: i128, reach: i128) -> bool {
        let need = tick.saturating_add(reach).saturating_add(self.margin);
        self.trace.progress.completeness.covers(need) && self.watermark >= need
    }

    /// Every activation in position order. `done` receives each activation
    /// that ended before the run stopped.
    fn activations(&mut self, done: &mut Vec<Finished>) -> Result<(), Stop> {
        let (positions, guarded) = match self.clause.activation() {
            Activation::Origin => (1, false),
            Activation::Each { guarded } => (self.stamps.len(), guarded),
        };
        for anchor in 0..positions {
            let at = position_of(anchor)?;
            if guarded {
                match self.letters.activation(at, &mut *self.meter)? {
                    AtomReading::True => {}
                    AtomReading::False => continue,
                    AtomReading::Undefined(cause) => {
                        return Err(Stop::Undefined(UndefinedEvaluation { at, cause }))
                    }
                }
            }
            let captured = match self.clause.capture_site() {
                CaptureSite::Never => false,
                CaptureSite::Origin => anchor == 0,
                CaptureSite::Activations => true,
            };
            if captured {
                if let CaptureReading::Undefined { cause, .. } =
                    self.letters.captures(at, &mut *self.meter)?
                {
                    return Err(Stop::Undefined(UndefinedEvaluation { at, cause }));
                }
            }
            let closure = self.demand(anchor)?;
            if let Some((&(position, _), cause)) = self
                .undefined
                .iter()
                .find(|(key, _)| closure.contains(&(key.1, key.0)))
            {
                return Err(Stop::Undefined(UndefinedEvaluation {
                    at: position_of(position)?,
                    cause: cause.clone(),
                }));
            }
            done.push(self.classify(anchor, closure)?);
        }
        Ok(())
    }

    /// Visit every node the activation at `anchor` depends on, charging each
    /// temporal visit once; the visits already made by an earlier activation
    /// are not repeated. Returns the dependency closure, operators before
    /// operands.
    fn demand(&mut self, anchor: usize) -> Result<Vec<(usize, usize)>, Stop> {
        let root = (self.plan.root, anchor);
        let mut queue = VecDeque::from([root]);
        let mut seen: HashSet<(usize, usize)> = HashSet::from([root]);
        let mut closure = Vec::new();
        while let Some((node, position)) = queue.pop_front() {
            closure.push((node, position));
            let mut next: Vec<(usize, usize)> = Vec::new();
            match slot(&self.plan.nodes, node)? {
                Node::Constant(_) => {}
                Node::Atom => {
                    if self.visited.insert((node, position)) {
                        self.read_atom(node, position)?;
                    }
                }
                Node::Not(operand) => next.push((operand, position)),
                Node::And(left, right) | Node::Or(left, right) | Node::Implies(left, right) => {
                    next.push((left, position));
                    next.push((right, position));
                }
                Node::Scan(scan) => {
                    if self.visited.insert((node, position)) {
                        let frame = self.scan(node, position)?;
                        self.frames.insert((node, position), frame);
                    }
                    let frame = self
                        .frames
                        .get(&(node, position))
                        .ok_or_else(|| fault("scan-frame-visited"))?;
                    for candidate in &frame.candidates {
                        next.push((scan.witness, candidate.position));
                        if let Some(guard) = scan.guard {
                            next.push((guard, candidate.position));
                        }
                    }
                }
            }
            for key in next {
                if seen.insert(key) {
                    queue.push_back(key);
                }
            }
        }
        closure.sort_by_key(|key| std::cmp::Reverse(key.0));
        Ok(closure)
    }

    fn read_atom(&mut self, node: usize, position: usize) -> Result<(), Stop> {
        self.charge()?;
        let id = self
            .clause
            .formula()
            .node_id(node)
            .ok_or_else(|| fault("atom-has-node-id"))?;
        let at = position_of(position)?;
        match self.letters.valuation(id, at, &mut *self.meter)? {
            Valuation::True => {
                self.cells.insert((position, node), Cell::True);
            }
            Valuation::False => {
                self.cells.insert((position, node), Cell::False);
            }
            Valuation::Missing => {
                self.cells.insert((position, node), Cell::Missing);
            }
            Valuation::Conflicting => {
                self.cells.insert((position, node), Cell::Conflicting);
            }
            Valuation::Undefined(cause) => {
                self.cells.insert((position, node), Cell::Missing);
                self.undefined.insert((position, node), cause);
            }
        }
        Ok(())
    }

    /// One visit of the scan `node` at `position`: scan every position in the
    /// scan's direction (one charge each), keep the ones whose stamp lies in
    /// the interval, and read what a continuation could still add.
    fn scan(&mut self, node: usize, position: usize) -> Result<ScanFrame, Stop> {
        self.charge()?;
        let Node::Scan(scan) = slot(&self.plan.nodes, node)? else {
            return Err(fault("scan-node").into());
        };
        let Window::Closed { lower, upper } = scan.window else {
            return Err(fault("finite-window-interval-is-bounded").into());
        };
        let forward = scan.base.direction() == Direction::Future;
        let order: Vec<usize> = if forward {
            (position..self.stamps.len()).collect()
        } else {
            (0..=position).rev().collect()
        };
        let anchor = self.stamp(position)?;
        let cover = self.trace.clock.cover;
        let mut candidates: Vec<Candidate> = Vec::new();
        for candidate in order {
            self.charge()?;
            let stamp = u64::try_from(self.stamp(candidate)?).map_err(|_| fault("stamp-in-u64"))?;
            let anchor_stamp = u64::try_from(anchor).map_err(|_| fault("stamp-in-u64"))?;
            let part = match cover.eligible(
                anchor_stamp,
                stamp,
                candidate == position,
                forward,
                lower,
                upper,
            ) {
                super::cover::Containment::True => Part::Sure,
                super::cover::Containment::Indeterminate => Part::Maybe,
                super::cover::Containment::False => continue,
            };
            candidates.push(Candidate {
                position: candidate,
                part,
            });
        }
        if scan.guard.is_some() {
            self.require_order(&candidates)?;
        }
        let first = self.stamp(0)?;
        let (unestablished, history) = if forward {
            let reach = *self
                .reach
                .get(node)
                .ok_or_else(|| fault("plan-index-in-arena"))?;
            (!self.established(anchor, reach), false)
        } else {
            (
                candidates.is_empty()
                    && !self
                        .trace
                        .progress
                        .completeness
                        .covers(anchor - i128::from(lower) + self.margin),
                self.trace.progress.lower_boundary == LowerBoundary::HistoryCutoff
                    && anchor - i128::from(upper) - self.margin < first,
            )
        };
        Ok(ScanFrame {
            candidates,
            unestablished,
            history,
        })
    }

    /// QSpec FR-090-AC-4: two positions of equal stamp that both take part in
    /// an order-sensitive operator need comparable order keys; the refusal
    /// names the later position.
    fn require_order(&self, candidates: &[Candidate]) -> Result<(), Stop> {
        for pair in candidates.windows(2) {
            let [before, after] = pair else { continue };
            if self.stamp(before.position)? != self.stamp(after.position)? {
                continue;
            }
            let key = |position: usize| {
                self.trace
                    .positions
                    .get(position)
                    .and_then(|p| p.order_key.as_ref())
            };
            let comparable = matches!(
                (key(before.position), key(after.position)),
                (Some(left), Some(right)) if left.authority == right.authority
            );
            if !comparable {
                return Err(Stop::Refused(TemporalRefusal::UnorderedTie {
                    position: position_of(before.position.max(after.position))?,
                }));
            }
        }
        Ok(())
    }

    fn lifter(&self) -> Lifter<'_> {
        Lifter {
            plan: &self.plan,
            cells: &self.cells,
            frames: &self.frames,
        }
    }

    /// Lift the activation at `anchor` and classify it.
    fn classify(&self, anchor: usize, closure: Vec<(usize, usize)>) -> Result<Finished, Stop> {
        let root = (self.plan.root, anchor);
        let poss = self
            .lifter()
            .exact(&closure, root, &Variant::default())
            .ok_or_else(|| fault("root-lifts"))?;
        let tick = self.stamp(anchor)?;
        let reach = slot(&self.reach, self.plan.root)?;
        let established = self.established(tick, reach);
        let result = if poss.can_true && !poss.can_false {
            Ending::True
        } else if poss.can_false && !poss.can_true {
            Ending::False
        } else if !poss.can_true && !poss.can_false {
            return Err(Stop::Refused(TemporalRefusal::Fault(fault(
                "possibility-set-is-not-empty",
            ))));
        } else {
            let need = tick.saturating_add(reach).saturating_add(self.margin);
            if self.watermark < need {
                Ending::Pending {
                    deadline: u64::try_from(need).map_err(|_| fault("deadline-in-u64"))?,
                }
            } else {
                Ending::Indeterminate(self.reason(&closure, root)?)
            }
        };
        Ok(Finished {
            anchor,
            result,
            closure,
            established,
            poss,
        })
    }

    /// Why a closed window's two-valued result stays open: the cause whose
    /// repair would settle it, history first, then completeness, then the
    /// first valuation or straddled bound in position order (FR-501
    /// "Verdicts").
    fn reason(
        &self,
        closure: &[(usize, usize)],
        root: (usize, usize),
    ) -> Result<IndeterminateReason, InternalFault> {
        let lifter = self.lifter();
        let settles = |variant: &Variant| {
            lifter
                .exact(closure, root, variant)
                .is_some_and(Poss::is_singleton)
        };
        let frames = || {
            closure
                .iter()
                .filter_map(|key| self.frames.get(key))
                .collect::<Vec<_>>()
        };
        let any_history = frames().iter().any(|frame| frame.history);
        let any_unestablished = frames().iter().any(|frame| frame.unestablished);
        let repair = |no_history, no_unestablished| Variant {
            no_history,
            no_unestablished,
            ..Variant::default()
        };
        if any_history && settles(&repair(true, false)) {
            return Ok(IndeterminateReason::MissingHistory);
        }
        if any_unestablished && settles(&repair(false, true)) {
            return Ok(IndeterminateReason::IncompleteInput);
        }
        // Valuation sources and straddled positions, in position order.
        let mut cells: Vec<(usize, usize, Cell)> = closure
            .iter()
            .filter(|(node, _)| matches!(self.plan.nodes.get(*node), Some(Node::Atom)))
            .filter_map(|(node, position)| {
                self.cells
                    .get(&(*position, *node))
                    .filter(|cell| cell.ambiguous())
                    .map(|cell| (*position, *node, *cell))
            })
            .collect();
        cells.sort_unstable_by_key(|(position, node, _)| (*position, *node));
        cells.dedup_by_key(|(position, node, _)| (*position, *node));
        let mut straddled: Vec<usize> = frames()
            .iter()
            .flat_map(|frame| frame.candidates.iter())
            .filter(|candidate| candidate.part == Part::Maybe)
            .map(|candidate| candidate.position)
            .collect();
        straddled.sort_unstable();
        straddled.dedup();
        let mut sources: Vec<(usize, Option<(usize, Cell)>)> = cells
            .iter()
            .map(|(position, node, cell)| (*position, Some((*node, *cell))))
            .chain(straddled.iter().map(|position| (*position, None)))
            .collect();
        sources.sort_by_key(|(position, source)| (*position, source.map(|(node, _)| node)));
        let describe = |position: usize, source: Option<(usize, Cell)>| match source {
            Some((node, Cell::Conflicting)) => {
                let atom = self
                    .clause
                    .formula()
                    .node_id(node)
                    .ok_or_else(|| fault("atom-has-node-id"))?;
                Ok(IndeterminateReason::ConflictingValuation {
                    position: position_of(position)?,
                    atom,
                })
            }
            Some((node, _)) => {
                let atom = self
                    .clause
                    .formula()
                    .node_id(node)
                    .ok_or_else(|| fault("atom-has-node-id"))?;
                Ok(IndeterminateReason::MissingValuation {
                    position: position_of(position)?,
                    atom,
                })
            }
            None => Ok(IndeterminateReason::StraddlesBound),
        };
        for (position, source) in &sources {
            let deciding = match source {
                Some((node, _)) => [Poss::TRUE, Poss::FALSE].into_iter().any(|poss| {
                    settles(&Variant {
                        fixed: vec![((*position, *node), poss)],
                        ..Variant::default()
                    })
                }),
                None => [true, false].into_iter().any(|takes_part| {
                    settles(&Variant {
                        settled: Some((*position, takes_part)),
                        ..Variant::default()
                    })
                }),
            };
            if deciding {
                return describe(*position, *source);
            }
        }
        // No single repair settles it: report the first cause present.
        if any_history {
            return Ok(IndeterminateReason::MissingHistory);
        }
        if any_unestablished {
            return Ok(IndeterminateReason::IncompleteInput);
        }
        match sources.first() {
            Some((position, source)) => describe(*position, *source),
            None => Err(fault("two-valued-result-has-a-cause")),
        }
    }

    /// The decision support of `activation`: the `(position, atom)` pairs
    /// whose single valuation, flipped or settled, or whose position, other
    /// than the anchor, removed, changes the lifted result of the activation.
    fn support(&self, activation: &Finished) -> Result<Vec<SupportPair>, InternalFault> {
        let lifter = self.lifter();
        let root = (self.plan.root, activation.anchor);
        let closure = &activation.closure;
        let definite = activation.poss.is_singleton();
        let changes = |variant: &Variant| {
            lifter
                .exact(closure, root, variant)
                .is_some_and(|poss| poss != activation.poss)
        };
        let mut atoms: Vec<(usize, usize)> = closure
            .iter()
            .filter(|(node, _)| matches!(self.plan.nodes.get(*node), Some(Node::Atom)))
            .map(|(node, position)| (*position, *node))
            .collect();
        atoms.sort_unstable();
        atoms.dedup();
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for &(position, node) in &atoms {
            let Some(cell) = self.cells.get(&(position, node)) else {
                continue;
            };
            let perturbed: &[Poss] = match cell {
                Cell::True => &[Poss::FALSE],
                Cell::False => &[Poss::TRUE],
                Cell::Missing | Cell::Conflicting => &[Poss::TRUE, Poss::FALSE],
            };
            let valuation = perturbed.iter().any(|poss| {
                changes(&Variant {
                    fixed: vec![((position, node), *poss)],
                    ..Variant::default()
                })
            });
            let removal = definite
                && position != activation.anchor
                && changes(&Variant {
                    removed: Some(position),
                    ..Variant::default()
                });
            if valuation || removal {
                pairs.push((position, node));
            }
        }
        let mut support = Vec::new();
        for (position, node) in pairs {
            support.push(SupportPair {
                position: position_of(position)?,
                atom: self
                    .clause
                    .formula()
                    .node_id(node)
                    .ok_or_else(|| fault("atom-has-node-id"))?,
            });
        }
        Ok(support)
    }

    /// Assemble the verdict and report from the activations that ended and
    /// what stopped the run.
    fn settle(self, done: Vec<Finished>, tail: Tail) -> Result<WindowEvaluation, InternalFault> {
        let first_false = done.iter().find(|a| a.result == Ending::False);
        let first_open = done
            .iter()
            .find(|a| matches!(a.result, Ending::Pending { .. } | Ending::Indeterminate(_)));
        let scope_closed = self.trace.progress.scope == super::trace::Closure::Closed;
        let (verdict, basis, support) = if let Some(activation) = first_false {
            let basis = if activation.established && scope_closed {
                Basis::ClosedScope
            } else {
                Basis::DecisiveCounterexample
            };
            (
                WindowVerdict::Violated {
                    at: position_of(activation.anchor)?,
                },
                basis,
                self.support(activation)?,
            )
        } else {
            match tail {
                Tail::Undefined(undefined) => (
                    WindowVerdict::Undefined(undefined),
                    Basis::Unavailable,
                    Vec::new(),
                ),
                Tail::Incomplete(incomplete) => (
                    WindowVerdict::Incomplete(incomplete),
                    Basis::Unavailable,
                    Vec::new(),
                ),
                Tail::Done => match first_open {
                    Some(activation) => {
                        let at = position_of(activation.anchor)?;
                        let (verdict, basis) = match &activation.result {
                            Ending::Pending { deadline } => (
                                WindowVerdict::Pending {
                                    at,
                                    deadline: *deadline,
                                },
                                Basis::Unsettled,
                            ),
                            Ending::Indeterminate(reason) => (
                                WindowVerdict::Indeterminate {
                                    at,
                                    reason: reason.clone(),
                                },
                                Basis::Unavailable,
                            ),
                            Ending::True | Ending::False => {
                                return Err(fault("open-activation-is-open"))
                            }
                        };
                        (verdict, basis, self.support(activation)?)
                    }
                    None => {
                        let established = done.iter().all(|a| a.established);
                        let basis = if established && scope_closed {
                            Basis::ClosedScope
                        } else {
                            Basis::DecisiveWitness
                        };
                        let mut support = Vec::new();
                        for activation in &done {
                            support.extend(self.support(activation)?);
                        }
                        support.sort();
                        support.dedup();
                        (WindowVerdict::Holds, basis, support)
                    }
                },
            }
        };
        let assessment = match verdict {
            WindowVerdict::Incomplete(_) => Assessment::Failed,
            WindowVerdict::Holds
            | WindowVerdict::Violated { .. }
            | WindowVerdict::Pending { .. }
            | WindowVerdict::Indeterminate { .. }
            | WindowVerdict::Undefined(_) => Assessment::Completed,
        };
        let progress = self.trace.progress;
        let mut readings: Vec<(usize, usize, Cell)> = self
            .cells
            .iter()
            .map(|((position, node), cell)| (*position, *node, *cell))
            .collect();
        readings.sort_unstable_by_key(|(position, node, _)| (*position, *node));
        let identity = Preimage {
            trace: self.trace,
            profile: TemporalProfile::TimestampedEventFiniteWindow.identity(),
            readings: &readings,
            ceilings: self.meter.limits(),
        }
        .identity()?;
        Ok(WindowEvaluation {
            verdict,
            basis,
            support,
            premises: Premises {
                scope: progress.scope,
                execution: progress.execution,
                assessment,
                completeness: progress.completeness,
                watermark: self.watermark,
                lower_boundary: progress.lower_boundary,
                cover: self.trace.clock.cover,
                unit: self.trace.clock.unit.clone(),
            },
            identity,
        })
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! The finite-window evaluator (FR-501): demand each activation's node
//! visits, charging the meter and reading the letters, then lift the formula
//! over the possibility sets and classify the result.

use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::temporal::facts::ClauseFacts;
use qsl_semantics::check::{Activation, CaptureSite, TemporalProfile};
use quire_exact::{Charge, ChargePoint, Incomplete, Meter};
use quire_semantic_value::catalog::InternalFault;

use super::identity::Preimage;
use super::lift::{Candidate, Cell, Lifter, Part, Poss, ScanFrame, Variant};
use super::outcome::{
    Assessment, Basis, IndeterminateReason, Premises, SupportPair, Valuation, WindowEvaluation,
    WindowVerdict,
};
use super::trace::{covers, LowerBoundary, TimestampedTrace};
use crate::temporal::ledger::ProgressClosure;
use crate::temporal::letters::{
    AtomReading, CaptureReading, LetterStop, TemporalLetters, UndefinedCause,
};
use crate::temporal::outcome::{TemporalRefusal, UndefinedEvaluation};
use crate::temporal::plan::{fault, slot, Direction, Node, Plan, Window};
use crate::temporal::position::TemporalPosition;

const VISIT: ChargePoint = ChargePoint::TemporalVisit;

/// One unit of the work of a finite-window evaluation at the
/// `temporal.visit` charge point, named for the FR-523 counters it spends.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowUnit {
    /// A temporal operator node visited at its activation.
    Step,
    /// One position scanned by an operator node.
    Scan,
    /// One atomic `holds` read at an observed position.
    Lookup,
    /// One atomic `holds` read at a silent sample (FR-504), which is no
    /// observation and no evaluated position visit.
    SilentLookup,
    /// A temporal operator node visited at a silent sample.
    SilentStep,
    /// One silent sample scanned by an operator node.
    SilentScan,
}

/// Where an evaluation charges its `temporal.visit` units instead of the
/// bare meter (FR-523): the temporal evidence call's counters, which refuse
/// a unit before the meter sees it.
pub trait WindowCharge {
    /// Charge one unit of work.
    ///
    /// # Errors
    ///
    /// The `Incomplete` that stops the evaluation; a refused unit is not
    /// performed.
    fn charge(&mut self, unit: WindowUnit) -> Result<(), Incomplete>;
}

/// Why a run stopped before it finished every activation.
enum Stop {
    Incomplete(Incomplete),
    /// An `on each` guard that cannot be established at a position.
    GuardUnresolved(TemporalPosition),
    Undefined(UndefinedEvaluation),
    /// A `holds` condition, an `on each` guard or a capture was refused at
    /// this position (FR-327 "Refused condition").
    ConditionRefused(TemporalPosition),
    Refused(TemporalRefusal),
}

impl Stop {
    /// The stop of a letter that failed at `at`: its refusal is the refused
    /// condition at `at`.
    fn at(stop: LetterStop, at: TemporalPosition) -> Self {
        match stop {
            LetterStop::Refused => Self::ConditionRefused(at),
            other => other.into(),
        }
    }
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
            // Every read of a letter goes through `Stop::at`, which knows
            // the position the refusal is returned at.
            LetterStop::Refused => Self::Refused(TemporalRefusal::Fault(fault(
                "refused-condition-has-its-position",
            ))),
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
    GuardUnresolved(TemporalPosition),
    Undefined(UndefinedEvaluation),
    Incomplete(Incomplete),
}

struct Run<'a, L: TemporalLetters> {
    clause: &'a ClauseFacts,
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
    /// The caller's own charging of the visits, when it has one.
    units: Option<&'a mut dyn WindowCharge>,
    /// The one activation anchor the run evaluates, when the caller names it.
    only: Option<usize>,
    visited: HashSet<(usize, usize)>,
    /// The atom each node stands for: nodes whose conditions are the same
    /// atom share one valuation at a position (QSpec FR-160).
    classes: Vec<usize>,
    /// The valuations read, by (position, atom class).
    cells: HashMap<(usize, usize), Cell>,
    undefined: BTreeMap<(usize, usize), UndefinedCause>,
    frames: HashMap<(usize, usize), ScanFrame>,
    /// The `temporal.visit` charges admitted so far.
    visits: u64,
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
    clause: &ClauseFacts,
    trace: &TimestampedTrace,
    letters: &mut L,
    meter: &mut Meter,
) -> Result<WindowEvaluation, TemporalRefusal> {
    run_window(clause, trace, letters, meter, None, None)
}

/// [`evaluate_finite_window`] for the one activation anchored at the
/// admitted position `anchor` (an `on each` clause activates once per
/// instance, FR-516), with every `temporal.visit` unit charged through
/// `units` before it reaches the meter (FR-523). An `on origin` clause
/// activates at position 0 whatever `anchor` names.
///
/// # Errors
///
/// As [`evaluate_finite_window`].
pub fn evaluate_finite_window_at<L: TemporalLetters>(
    clause: &ClauseFacts,
    trace: &TimestampedTrace,
    anchor: usize,
    letters: &mut L,
    units: &mut dyn WindowCharge,
    meter: &mut Meter,
) -> Result<WindowEvaluation, TemporalRefusal> {
    run_window(clause, trace, letters, meter, Some(units), Some(anchor))
}

fn run_window<'a, L: TemporalLetters>(
    clause: &'a ClauseFacts,
    trace: &'a TimestampedTrace,
    letters: &'a mut L,
    meter: &'a mut Meter,
    units: Option<&'a mut dyn WindowCharge>,
    only: Option<usize>,
) -> Result<WindowEvaluation, TemporalRefusal> {
    if !matches!(
        clause.profile(),
        TemporalProfile::TimestampedEventFiniteWindow
            | TemporalProfile::EventPositionFalseExtension
            | TemporalProfile::FixedSampleFalseExtension
    ) {
        return Err(TemporalRefusal::NotFiniteWindowProfile {
            profile: clause.profile(),
        });
    }
    let plan = clause
        .formula()
        .plan()
        .map_err(TemporalRefusal::Fault)?
        .clone();
    let reach = reaches(&plan).map_err(TemporalRefusal::Fault)?;
    let classes = (0..plan.nodes.len())
        .map(|index| {
            let atom = matches!(plan.nodes.get(index), Some(Node::Atom));
            match clause.formula().node_id(index) {
                Some(id) if atom => letters.atom_class(id),
                _ => index,
            }
        })
        .collect();
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
        units,
        only,
        visited: HashSet::new(),
        classes,
        cells: HashMap::new(),
        undefined: BTreeMap::new(),
        frames: HashMap::new(),
        visits: 0,
    };
    let mut done = Vec::new();
    let tail = match run.activations(&mut done) {
        Ok(()) => Tail::Done,
        Err(Stop::Incomplete(incomplete)) => Tail::Incomplete(incomplete),
        Err(Stop::GuardUnresolved(at)) => Tail::GuardUnresolved(at),
        Err(Stop::Undefined(undefined)) => Tail::Undefined(undefined),
        // In evaluation order: an atom undefined at an earlier position, or
        // an activation that ended false before the refused condition, is
        // returned instead of it.
        Err(Stop::ConditionRefused(at)) => {
            let refused = usize::try_from(at.index()).unwrap_or(usize::MAX);
            if let Some(((position, _), cause)) = run
                .undefined
                .iter()
                .find(|((position, _), _)| *position < refused)
            {
                Tail::Undefined(UndefinedEvaluation {
                    at: position_of(*position).map_err(TemporalRefusal::Fault)?,
                    cause: cause.clone(),
                })
            } else if done
                .iter()
                .any(|activation| activation.result == Ending::False)
            {
                Tail::Done
            } else {
                return Err(TemporalRefusal::ConditionRefused { at });
            }
        }
        Err(Stop::Refused(refusal)) => return Err(refusal),
    };
    run.settle(done, tail).map_err(TemporalRefusal::Fault)
}

/// The future reach of every node.
pub(super) fn reaches(plan: &Plan) -> Result<Vec<i128>, InternalFault> {
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
    fn charge(&mut self, unit: WindowUnit) -> Result<(), Stop> {
        match self.units.as_deref_mut() {
            Some(units) => units.charge(unit)?,
            None => self.meter.charge(Charge::new(VISIT))?,
        }
        self.visits = self.visits.saturating_add(1);
        Ok(())
    }

    /// The atom class of `node`: itself unless another node is the same atom.
    fn class(&self, node: usize) -> usize {
        self.classes.get(node).copied().unwrap_or(node)
    }

    fn stamp(&self, position: usize) -> Result<i128, InternalFault> {
        slot(&self.stamps, position)
    }

    /// Whether the window of an activation (or sub-instance) at `tick`
    /// reaching `reach` ticks ahead is established (FR-501 "Established
    /// window").
    fn established(&self, tick: i128, reach: i128) -> bool {
        let need = tick.saturating_add(reach).saturating_add(self.margin);
        covers(self.trace.progress.completeness, need) && self.watermark >= need
    }

    /// Every activation in position order. `done` receives each activation
    /// that ended before the run stopped.
    fn activations(&mut self, done: &mut Vec<Finished>) -> Result<(), Stop> {
        let (positions, guarded) = match self.clause.activation() {
            Activation::Origin => (1, false),
            // Silent samples are no observation: no `on each` activation
            // anchors on one (FR-504).
            Activation::Each { guarded } => (self.trace.observed, guarded),
        };
        // An anchored run is one instance's single activation: its guard is
        // the instance's, established when the instance was created.
        let (anchors, guarded) = match (self.clause.activation(), self.only) {
            (Activation::Each { .. }, Some(anchor)) => (anchor..anchor.saturating_add(1), false),
            _ => (0..positions, guarded),
        };
        for anchor in anchors {
            let at = position_of(anchor)?;
            if guarded {
                match self
                    .letters
                    .activation(at, &mut *self.meter)
                    .map_err(|stop| Stop::at(stop, at))?
                {
                    AtomReading::True => {}
                    AtomReading::False => continue,
                    // A guard that cannot be established is an incomplete
                    // activation, never a false guard (FR-044-AC-7).
                    AtomReading::Undefined(_) => return Err(Stop::GuardUnresolved(at)),
                }
            }
            let captured = match self.clause.capture_site() {
                CaptureSite::Never => false,
                CaptureSite::Origin => anchor == 0,
                CaptureSite::Activations => true,
            };
            if captured {
                if let CaptureReading::Undefined { cause, .. } = self
                    .letters
                    .captures(at, &mut *self.meter)
                    .map_err(|stop| Stop::at(stop, at))?
                {
                    return Err(Stop::Undefined(UndefinedEvaluation { at, cause }));
                }
            }
            let closure = self.demand(anchor)?;
            if let Some((&(position, _), cause)) = self.undefined.iter().find(|(key, _)| {
                closure
                    .iter()
                    .any(|(node, at)| *at == key.0 && self.class(*node) == key.1)
            }) {
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

    /// Read the atom `node` at `position`. Observations that share a stamp
    /// are readings of one instant, so every position of the stamp is read:
    /// when two of them give differing valuations of the atom, it is
    /// `ConflictingValuation` at every position of that stamp (FR-501
    /// "Verdicts"), whatever order they were listed in.
    fn read_atom(&mut self, node: usize, position: usize) -> Result<(), Stop> {
        let silent = position >= self.trace.observed;
        self.charge(if silent {
            WindowUnit::SilentLookup
        } else {
            WindowUnit::Lookup
        })?;
        let class = self.class(node);
        let id = self
            .clause
            .formula()
            .node_id(class)
            .ok_or_else(|| fault("atom-has-node-id"))?;
        let stamp = self.stamp(position)?;
        let mut group = Vec::new();
        for (index, candidate) in self.stamps.iter().enumerate() {
            if *candidate == stamp {
                group.push(index);
            }
        }
        for member in &group {
            if self.cells.contains_key(&(*member, class)) {
                continue;
            }
            let at = position_of(*member)?;
            match self
                .letters
                .valuation(id, at, &mut *self.meter)
                .map_err(|stop| Stop::at(stop, at))?
            {
                Valuation::True => {
                    self.cells.insert((*member, class), Cell::True);
                }
                Valuation::False => {
                    self.cells.insert((*member, class), Cell::False);
                }
                Valuation::Missing => {
                    self.cells.insert((*member, class), Cell::Missing);
                }
                Valuation::Undefined(cause) => {
                    self.cells.insert((*member, class), Cell::Missing);
                    self.undefined.insert((*member, class), cause);
                }
            }
        }
        let held = |cell: Option<&Cell>, want: Cell| cell == Some(&want);
        let differ = group
            .iter()
            .any(|member| held(self.cells.get(&(*member, class)), Cell::True))
            && group
                .iter()
                .any(|member| held(self.cells.get(&(*member, class)), Cell::False));
        if differ {
            for member in &group {
                if let Some(cell) = self.cells.get_mut(&(*member, class)) {
                    if matches!(cell, Cell::True | Cell::False) {
                        *cell = Cell::Conflicting;
                    }
                }
            }
        }
        Ok(())
    }

    /// One visit of the scan `node` at `position`: scan every position in the
    /// scan's direction (one charge each), keep the ones whose stamp lies in
    /// the interval, and read what a continuation could still add.
    fn scan(&mut self, node: usize, position: usize) -> Result<ScanFrame, Stop> {
        let silent = position >= self.trace.observed;
        self.charge(if silent {
            WindowUnit::SilentStep
        } else {
            WindowUnit::Step
        })?;
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
            self.charge(if candidate >= self.trace.observed {
                WindowUnit::SilentScan
            } else {
                WindowUnit::Scan
            })?;
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
                    && !covers(
                        self.trace.progress.completeness,
                        anchor - i128::from(lower) + self.margin,
                    ),
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
    /// an order-sensitive operator need a usable order (FR-501): distinct keys
    /// of one order authority. A key repeated within its authority orders
    /// nothing, and the observation identity never breaks the tie. The refusal
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
            let ordered = matches!(
                (key(before.position), key(after.position)),
                (Some(left), Some(right))
                    if left.authority == right.authority && left.key != right.key
            );
            if !ordered {
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
            classes: &self.classes,
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
                    .get(&(*position, self.class(*node)))
                    .filter(|cell| cell.ambiguous())
                    .map(|cell| (*position, self.class(*node), *cell))
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
            let Some(cell) = self.cells.get(&(position, self.class(node))) else {
                continue;
            };
            let perturbed: &[Poss] = match cell {
                Cell::True => &[Poss::FALSE],
                Cell::False => &[Poss::TRUE],
                Cell::Missing | Cell::Conflicting => &[Poss::TRUE, Poss::FALSE],
            };
            let valuation = perturbed.iter().any(|poss| {
                changes(&Variant {
                    fixed: vec![((position, self.class(node)), *poss)],
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
        let scope_closed = self.trace.asserted.scope == ProgressClosure::Closed;
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
                Tail::GuardUnresolved(at) => (
                    WindowVerdict::ActivationIncomplete { at },
                    Basis::Unavailable,
                    Vec::new(),
                ),
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
                                if self.clause.profile()
                                    == TemporalProfile::EventPositionFalseExtension
                                {
                                    WindowVerdict::PendingNextEvent { at }
                                } else {
                                    WindowVerdict::Pending {
                                        at,
                                        deadline: *deadline,
                                    }
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
            WindowVerdict::Incomplete(_) | WindowVerdict::ActivationIncomplete { .. } => {
                Assessment::Failed
            }
            WindowVerdict::Holds
            | WindowVerdict::Violated { .. }
            | WindowVerdict::Pending { .. }
            | WindowVerdict::PendingNextEvent { .. }
            | WindowVerdict::Indeterminate { .. }
            | WindowVerdict::Undefined(_) => Assessment::Completed,
        };
        let progress = self.trace.asserted;
        let mut readings: Vec<(usize, usize, Cell)> = self
            .cells
            .iter()
            .map(|((position, node), cell)| (*position, *node, *cell))
            .collect();
        readings.sort_unstable_by_key(|(position, node, _)| (*position, *node));
        let identity = Preimage {
            trace: self.trace,
            profile: self.clause.profile().identity(),
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
                watermark: progress.watermark.inclusive(),
                lower_boundary: progress.lower_boundary,
                cover: self.trace.clock.cover,
                unit: self.trace.clock.unit.clone(),
            },
            identity,
            visits: self.visits,
        })
    }
}

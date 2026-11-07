// SPDX-License-Identifier: AGPL-3.0-or-later
//! The TemporalTrace evaluator: one definition of a temporal formula's truth
//! on a closed trace, a finite prefix and a lasso (ADR-018 SM-1, SM-8).
//!
//! **One word model.** Every trace is read as one infinite word of positions.
//! A lasso of `n` prefix and `L` loop positions is that word. A finite trace
//! is the same word with a one-position loop after its last position, a
//! sentinel that carries no atoms: false under a bounded profile's
//! false-extension (QSpec FR-090), unknown for a prefix of an unknown
//! continuation (FR-328). Positions before 0 read every atom false and every
//! constant unchanged (QSpec FR-092).
//!
//! **Three values.** Truth is Kleene's three values, so a position the
//! prefix does not supply is `Unknown` and a formula is decided only where
//! the supplied positions decide it. A closed trace and a lasso never reach
//! `Unknown` (unless an atom is undefined, below).
//!
//! **SM-8 unrolling.** A subformula of past reach `R` is periodic from
//! `n + R`, so the word is unrolled `m = ceil(R / L) + 1` loop copies, to
//! `N = n + m * L` positions, and a position `q >= N` reads the position of
//! the last copy congruent to it modulo `L`.
//!
//! **Work.** One work unit is charged for each (operator node, position)
//! first visit, before the visit runs, and one for each (atom, position) the
//! letter reads (ADR-014 TR-5). A visit is memoized, so it is charged once.
//! Evaluation keeps an explicit frame stack: a formula of any depth, and any
//! window length, evaluates without recursion.

use std::collections::{HashMap, HashSet};

use qsl_semantics::check::{
    Activation, CaptureSite, CheckedFormula, CheckedTemporalClause, FormulaNodeId, TemporalProfile,
};
use quire_exact::{Charge, ChargePoint, Incomplete, Meter};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::fairness::FairnessConstraint;

use super::input::TemporalTrace;
use super::letters::{AtomReading, CaptureReading, LetterStop, TemporalLetters, UndefinedCause};
use super::outcome::{
    InsufficientDataEvaluation, TemporalOutcome, TemporalRefusal, UndefinedEvaluation,
};
use super::plan::{fault, slot, Direction, Node, Plan, Scan, Window};
use super::position::TemporalPosition;
use super::steps::{LoopShape, ReadStop, Steps};
use super::window::{WindowLimitReached, WindowRefusal};

/// The named point every visit charges, one work unit each: `temporal.visit`
/// (`quire.value.accounting/v1`), charged once per visit this evaluator makes.
/// `graph.expand` is reserved for `reaches` and is never charged here.
const VISIT: ChargePoint = ChargePoint::TemporalVisit;

/// What a trace's positions past its represented ones are, and how a clause
/// settles on it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Mode {
    /// A closed finite trace under a bounded profile: false-extension.
    Closed,
    /// A finite prefix under the infinite-trace profile.
    Prefix,
    /// A lasso under the infinite-trace profile.
    Lasso,
}

/// Kleene's three values, ordered `False < Unknown < True`.
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

    fn and(self, other: Self) -> Self {
        self.min(other)
    }

    fn or(self, other: Self) -> Self {
        self.max(other)
    }

    fn flip(self, dual: bool) -> Self {
        if dual {
            self.negate()
        } else {
            self
        }
    }
}

/// Why evaluation stopped before a value.
enum Stop {
    Incomplete(Incomplete),
    /// A condition the clause reads was refused at this position.
    ConditionRefused(TemporalPosition),
    WindowLimit(WindowLimitReached),
    Window(WindowRefusal),
    Fault(InternalFault),
}

impl From<LetterStop> for Stop {
    fn from(stop: LetterStop) -> Self {
        match stop {
            LetterStop::Incomplete(incomplete) => Self::Incomplete(incomplete),
            LetterStop::WindowLimit(limit) => Self::WindowLimit(limit),
            LetterStop::Window(refusal) => Self::Window(refusal),
            LetterStop::Fault(fault) => Self::Fault(fault),
            // Only `read_letters`, which knows the position, receives a
            // refused condition; any other read of a letter is a fault.
            LetterStop::Refused => Self::Fault(fault("condition-refused-at-a-read-position")),
        }
    }
}

impl From<Incomplete> for Stop {
    fn from(incomplete: Incomplete) -> Self {
        Self::Incomplete(incomplete)
    }
}

impl From<InternalFault> for Stop {
    fn from(fault: InternalFault) -> Self {
        Self::Fault(fault)
    }
}

/// How many unrolled positions a formula is evaluated over (ADR-018 SM-8,
/// FR-125 "Reading a lasso").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Unrolling {
    /// The formula's past reach `R`.
    pub past_reach: u128,
    /// The loop copies `m = ceil(R / L) + 1`.
    pub copies: u128,
    /// The unrolled positions `n + m * L`.
    pub positions: u128,
}

/// A trace validated against its clause's profile.
#[derive(Clone, Copy, Debug)]
struct Shape {
    mode: Mode,
    /// `n`: the positions before the loop (a finite trace's whole length).
    prefix: u64,
    /// `L` of a lasso; 0 for a finite trace.
    real_loop: u64,
    /// The positions the trace represents.
    represented: u64,
}

impl Shape {
    /// The shape of `trace` under `profile`. `no_progress` says a lasso's
    /// loop has no counted step (ADR-027 PB-3): its positions stay inside
    /// every bounded horizon, which the false extension never reaches, so a
    /// bounded profile reads it as the infinite trace it is. Remaining work:
    /// STD-254 settles that exception against FR-329's refusal.
    fn new(
        profile: TemporalProfile,
        trace: TemporalTrace,
        no_progress: bool,
    ) -> Result<Self, TemporalRefusal> {
        let (prefix, real_loop) = match trace {
            TemporalTrace::Finite { len: 0 } => return Err(TemporalRefusal::EmptyTrace),
            TemporalTrace::Finite { len } => (len, 0),
            TemporalTrace::Lasso { loop_len: 0, .. } => return Err(TemporalRefusal::EmptyLoop),
            TemporalTrace::Lasso { prefix, loop_len } => (prefix, loop_len),
        };
        let represented = trace.represented().ok_or(TemporalRefusal::TraceTooLong)?;
        let mode = match (profile, trace) {
            // The finite-window profile reads time stamps, which a position
            // trace does not carry: `evaluate_finite_window` is its evaluator
            // (FR-501), and a `Finite` or `Lasso` trace is refused.
            (TemporalProfile::TimestampedEventFiniteWindow, _) => {
                return Err(TemporalRefusal::TraceCarriesNoStamps { profile })
            }
            // The timed profile reads time stamps: `evaluate_timed_at` is its
            // evaluator (FR-234).
            (TemporalProfile::Timed, _) => {
                return Err(TemporalRefusal::ProfileNotEvaluated { profile })
            }
            (
                TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension,
                TemporalTrace::Finite { .. },
            ) => Mode::Closed,
            (
                TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension,
                TemporalTrace::Lasso { .. },
            ) if no_progress => Mode::Lasso,
            (
                TemporalProfile::EventPositionFalseExtension
                | TemporalProfile::FixedSampleFalseExtension,
                TemporalTrace::Lasso { .. },
            ) => return Err(TemporalRefusal::LassoUnderBoundedProfile { profile }),
            (TemporalProfile::InfiniteTrace, TemporalTrace::Finite { .. }) => Mode::Prefix,
            (TemporalProfile::InfiniteTrace, TemporalTrace::Lasso { .. }) => Mode::Lasso,
        };
        Ok(Self {
            mode,
            prefix,
            real_loop,
            represented,
        })
    }

    /// `L` of the word: a finite trace's sentinel loop is one position.
    fn period(self) -> u64 {
        self.real_loop.max(1)
    }
}

/// The word's geometry (ADR-018 SM-8).
#[derive(Clone, Copy, Debug)]
struct Geometry {
    prefix: i128,
    period: i128,
    real_loop: i128,
    unrolled: i128,
}

/// Where a canonical position's letter comes from.
enum Site {
    Real(u64),
    Sentinel,
}

impl Geometry {
    /// A position `q >= 0` read at the position of the last unrolled loop
    /// copy congruent to it modulo `L` once it passes the unrolled word.
    fn canon(&self, q: i128) -> i128 {
        if q < self.unrolled {
            q
        } else {
            self.unrolled - self.period + (q - self.unrolled).rem_euclid(self.period)
        }
    }

    /// The represented position, or the sentinel, canonical `q` reads.
    fn site(&self, q: i128) -> Result<Site, InternalFault> {
        let position = if q < self.prefix {
            q
        } else if self.real_loop == 0 {
            return Ok(Site::Sentinel);
        } else {
            self.prefix + (q - self.prefix) % self.real_loop
        };
        u64::try_from(position)
            .map(Site::Real)
            .map_err(|_| fault("position-in-u64"))
    }
}

fn convert(value: u128) -> Result<i128, InternalFault> {
    i128::try_from(value).map_err(|_| fault("unrolled-positions-in-i128"))
}

/// The unrolling of `plan` over a word of `prefix` positions and loops of
/// `period` positions, whose steps count `per_pass` (ADR-027 PB-3) in one
/// pass of the loop when some do not.
fn unroll(
    plan: &Plan,
    prefix: u64,
    period: u64,
    per_pass: Option<i128>,
) -> Result<Unrolling, InternalFault> {
    let per_pass = per_pass.map(|steps| u128::try_from(steps).unwrap_or(0));
    let reach = plan.past_reach(u128::from(period), per_pass)?;
    let past_reach = slot(&reach, plan.root)?;
    let copies = past_reach.div_ceil(u128::from(period)).saturating_add(1);
    let positions = u128::from(prefix).saturating_add(copies.saturating_mul(u128::from(period)));
    Ok(Unrolling {
        past_reach,
        copies,
        positions,
    })
}

/// What the evaluator reads of a claim: a temporal clause's formula,
/// profile, activation, horizon and fairness set, or a hyper body's formula
/// and profile with the activation `on origin` (FR-175).
#[derive(Clone, Copy)]
pub(super) struct Subject<'a> {
    pub(super) formula: &'a CheckedFormula,
    pub(super) profile: TemporalProfile,
    pub(super) activation: Activation,
    pub(super) horizon: Option<u64>,
    pub(super) captures: CaptureSite,
    pub(super) fairness: &'a [FairnessConstraint],
}

impl<'a> Subject<'a> {
    /// A temporal clause's claim.
    pub(super) fn of_clause(clause: &'a CheckedTemporalClause) -> Self {
        Self {
            formula: clause.formula(),
            profile: clause.profile(),
            activation: clause.activation(),
            horizon: clause.horizon(),
            captures: clause.capture_site(),
            fairness: clause.fairness(),
        }
    }
}

/// The unrolling `clause` is evaluated over on `trace` (ADR-018 SM-8).
pub fn unrolling(
    clause: &CheckedTemporalClause,
    trace: TemporalTrace,
) -> Result<Unrolling, TemporalRefusal> {
    let subject = Subject::of_clause(clause);
    let shape = Shape::new(subject.profile, trace, false)?;
    let plan = Plan::new(subject.formula).map_err(TemporalRefusal::Fault)?;
    unroll(&plan, shape.prefix, shape.period(), None).map_err(TemporalRefusal::Fault)
}

/// How many positions of `trace` a prefix must hold to show every `false` of
/// `subject`'s formula on the unrolled word: the unrolled positions
/// (ADR-018 SM-8) plus the formula's future reach, since a bounded future
/// operator at the last unrolled position reads beyond it.
pub(super) fn bad_prefix_bound(
    subject: Subject<'_>,
    trace: TemporalTrace,
) -> Result<u128, TemporalRefusal> {
    let shape = Shape::new(subject.profile, trace, false)?;
    let plan = Plan::new(subject.formula).map_err(TemporalRefusal::Fault)?;
    let unrolled =
        unroll(&plan, shape.prefix, shape.period(), None).map_err(TemporalRefusal::Fault)?;
    // `left_floor` counts the atom itself as one position.
    let future = slot(&plan.left_floor, plan.root)
        .map_err(TemporalRefusal::Fault)?
        .saturating_sub(1);
    Ok(unrolled.positions.saturating_add(future))
}

/// What a run settles: the whole clause on its activations, or the root
/// formula at one position.
#[derive(Clone, Copy, Debug)]
pub(super) enum Goal {
    Clause,
    At(TemporalPosition),
}

/// Whether the loop of the lasso `trace` has no counted step, its closing step
/// included (ADR-027 PB-3).
fn loop_without_progress<L: TemporalLetters>(trace: TemporalTrace, letters: &L) -> bool {
    let TemporalTrace::Lasso { prefix, loop_len } = trace else {
        return false;
    };
    let Some(end) = prefix.checked_add(loop_len) else {
        return false;
    };
    !letters.counts_loop_edge()
        && (prefix.saturating_add(1)..end)
            .all(|index| !letters.counts_step(TemporalPosition::new(index)))
}

/// Evaluate `clause` on `trace` (FR-327, FR-328, FR-329).
pub(super) fn run<L: TemporalLetters>(
    clause: Subject<'_>,
    trace: TemporalTrace,
    goal: Goal,
    source: &mut L,
    meter: &mut Meter,
) -> Result<TemporalOutcome, TemporalRefusal> {
    run_seeking(clause, trace, goal, false, source, meter).0
}

/// The decisive atom of a violation (FR-330 "The decisive atom"), found by
/// the descent over the values the run itself computed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DecisiveAtom {
    /// The `holds` atom's formula node.
    pub atom: FormulaNodeId,
    /// The position the atom is read at.
    pub position: TemporalPosition,
}

/// Why the descent to the decisive atom stopped before it found one.
#[derive(Debug)]
pub enum DecisiveStop {
    /// The descent's visits exhausted the run's meter.
    Incomplete(Incomplete),
    /// The descent broke an evaluator invariant.
    Fault(InternalFault),
}

/// What the descent found: the atom, or none when the violation has no
/// decisive atom (FR-330).
pub type DecisiveSearch = Result<Option<DecisiveAtom>, DecisiveStop>;

/// [`run`], which, when `seek` is set and the clause is violated, also
/// descends to the violation's decisive atom with the same evaluator and
/// meter, so the descent's visits are the run's work.
pub(super) fn run_seeking<L: TemporalLetters>(
    clause: Subject<'_>,
    trace: TemporalTrace,
    goal: Goal,
    seek: bool,
    source: &mut L,
    meter: &mut Meter,
) -> (
    Result<TemporalOutcome, TemporalRefusal>,
    Option<DecisiveSearch>,
) {
    let mut decisive = None;
    let outcome = run_goal(
        clause,
        trace,
        goal,
        seek.then_some(&mut decisive),
        source,
        meter,
    );
    (outcome, decisive)
}

fn run_goal<L: TemporalLetters>(
    clause: Subject<'_>,
    trace: TemporalTrace,
    goal: Goal,
    decisive: Option<&mut Option<DecisiveSearch>>,
    source: &mut L,
    meter: &mut Meter,
) -> Result<TemporalOutcome, TemporalRefusal> {
    let no_progress = clause.profile.is_bounded()
        && matches!(trace, TemporalTrace::Lasso { .. })
        && loop_without_progress(trace, &*source);
    let shape = Shape::new(clause.profile, trace, no_progress)?;
    if let Goal::At(position) = goal {
        trace.admit(position)?;
    }
    // A supplied trace carries no enabledness, so a fairness set over it has
    // no premise to check against: nothing is evaluated (FR-328, FR-329).
    if matches!(goal, Goal::Clause) && !clause.profile.is_bounded() {
        if let Some(constraint) = clause.fairness.first() {
            return Ok(TemporalOutcome::MissingFairnessPremise {
                constraint: constraint.clone(),
            });
        }
    }
    let plan = Plan::new(clause.formula).map_err(TemporalRefusal::Fault)?;
    // ADR-027 PB-3: only an interval measures distance. A lasso's loop is
    // read in counted steps too, its closing step included.
    let steps = if plan.measures_distance() {
        let lasso = (shape.mode == Mode::Lasso).then(|| LoopShape {
            prefix: shape.prefix,
            loop_len: shape.real_loop,
            edge_counts: source.counts_loop_edge(),
        });
        match Steps::read(shape.represented, lasso, &*source, meter) {
            Ok(steps) => steps,
            Err(ReadStop::Incomplete(incomplete)) => {
                return Ok(TemporalOutcome::Incomplete(incomplete))
            }
            Err(ReadStop::Fault(internal)) => return Err(TemporalRefusal::Fault(internal)),
        }
    } else {
        None
    };
    let seek = decisive.is_some();
    let evaluation = Evaluator::new(clause, plan, shape, steps, meter).and_then(|mut evaluator| {
        evaluator.seek = seek;
        let outcome = evaluator.execute(clause, goal, source);
        if let Some(slot) = decisive {
            *slot = evaluator.decisive.take();
        }
        outcome
    });
    match evaluation {
        Ok(outcome) => Ok(outcome),
        Err(Stop::Incomplete(incomplete)) => Ok(TemporalOutcome::Incomplete(incomplete)),
        Err(Stop::ConditionRefused(at)) => Err(TemporalRefusal::ConditionRefused { at }),
        Err(Stop::WindowLimit(limit)) => Ok(TemporalOutcome::WindowLimitReached(limit)),
        Err(Stop::Window(refusal)) => Err(TemporalRefusal::Window(refusal)),
        Err(Stop::Fault(internal)) => Err(TemporalRefusal::Fault(internal)),
    }
}

/// One window of positions a scan runs over.
#[derive(Clone, Copy, Debug)]
struct Run {
    start: i128,
    /// `+1` for a forward window, `-1` for a backward one.
    step: i128,
    len: u128,
    /// A window that cycles through the last loop copy.
    wrap: Option<Wrap>,
}

#[derive(Clone, Copy, Debug)]
struct Wrap {
    base: i128,
    period: i128,
}

impl Run {
    fn linear(start: i128, step: i128, len: u128) -> Self {
        Self {
            start,
            step,
            len,
            wrap: None,
        }
    }

    fn at(&self, index: u128) -> Result<i128, InternalFault> {
        let offset = i128::try_from(index).map_err(|_| fault("window-index-in-i128"))?;
        Ok(match self.wrap {
            None => self.start + self.step * offset,
            Some(wrap) => wrap.base + (self.start - wrap.base + offset).rem_euclid(wrap.period),
        })
    }
}

/// The count of positions in `first..=last`.
fn span(first: i128, last: i128) -> Result<u128, InternalFault> {
    u128::try_from(last - first + 1).map_err(|_| fault("window-has-positions"))
}

/// One node visit in progress.
#[derive(Debug)]
struct Frame {
    node: usize,
    pos: i128,
    /// The next window position to scan.
    index: u128,
    /// How many window positions have been paid for, so a visit resumed
    /// after an operand visit does not pay again.
    scanned: u128,
    acc: Tri,
    guard: Tri,
    window: Option<Run>,
}

enum Advance {
    Done(Tri),
    Need(usize, i128),
}

enum Lookup {
    Ready(Tri),
    Missing(usize, i128),
}

macro_rules! ready {
    ($evaluator:expr, $operand:expr, $position:expr) => {
        match $evaluator.lookup($operand, $position)? {
            Lookup::Ready(value) => value,
            Lookup::Missing(node, position) => return Ok(Advance::Need(node, position)),
        }
    };
}

/// What reading the clause's letters over the read positions found: the
/// first undefined atom or the first refused condition, in evaluation order
/// (never both), and the first atom short of data.
struct LettersRead {
    undefined: Option<UndefinedEvaluation>,
    refused: Option<TemporalPosition>,
    insufficient: Option<InsufficientDataEvaluation>,
}

impl LettersRead {
    fn undefined(
        undefined: UndefinedEvaluation,
        insufficient: Option<InsufficientDataEvaluation>,
    ) -> Self {
        Self {
            undefined: Some(undefined),
            refused: None,
            insufficient,
        }
    }

    fn refused(at: TemporalPosition, insufficient: Option<InsufficientDataEvaluation>) -> Self {
        Self {
            undefined: None,
            refused: Some(at),
            insufficient,
        }
    }
}

struct Evaluator<'a> {
    formula: &'a CheckedFormula,
    plan: Plan,
    meter: &'a mut Meter,
    mode: Mode,
    represented: u64,
    geometry: Geometry,
    /// `-T` of each node, as a position.
    floors: Vec<i128>,
    /// The letter at the sentinel position.
    sentinel: Tri,
    /// The counted steps of a trace with an uncounted step; `None` when
    /// distance is the distance in positions.
    steps: Option<Steps>,
    /// The atoms read, by (node, represented position).
    letters: HashMap<(usize, u64), Tri>,
    /// Positions whose activation has at least one short window atom.
    insufficient_positions: HashSet<u64>,
    /// The first represented position the letter does not supply.
    unknown_from: Option<u64>,
    memo: HashMap<(usize, i128), Tri>,
    /// Whether a violation descends to its decisive atom.
    seek: bool,
    /// The descent's result, set when `seek` and the clause is violated.
    decisive: Option<DecisiveSearch>,
}

impl<'a> Evaluator<'a> {
    fn new(
        clause: Subject<'a>,
        plan: Plan,
        shape: Shape,
        steps: Option<Steps>,
        meter: &'a mut Meter,
    ) -> Result<Self, Stop> {
        let unrolled = unroll(
            &plan,
            shape.prefix,
            shape.period(),
            steps.as_ref().and_then(Steps::per_pass),
        )?;
        let geometry = Geometry {
            prefix: i128::from(shape.prefix),
            period: i128::from(shape.period()),
            real_loop: i128::from(shape.real_loop),
            unrolled: convert(unrolled.positions)?,
        };
        let floors = plan
            .left_floor
            .iter()
            .map(|floor| convert(*floor).map(|floor| -floor))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            formula: clause.formula,
            plan,
            meter,
            mode: shape.mode,
            represented: shape.represented,
            geometry,
            floors,
            sentinel: match shape.mode {
                Mode::Prefix => Tri::Unknown,
                Mode::Closed | Mode::Lasso => Tri::False,
            },
            steps,
            letters: HashMap::new(),
            insufficient_positions: HashSet::new(),
            unknown_from: None,
            memo: HashMap::new(),
            seek: false,
            decisive: None,
        })
    }

    fn charge(&mut self) -> Result<(), Stop> {
        self.meter.charge(Charge::new(VISIT))?;
        Ok(())
    }

    fn undefined(at: u64, cause: UndefinedCause) -> UndefinedEvaluation {
        UndefinedEvaluation {
            at: TemporalPosition::new(at),
            cause,
        }
    }

    /// Read the clause's letter at positions `0..read` (ADR-018 UE-1): the
    /// `when` condition when the activation has one, then each atom in
    /// clause-node order. Stops at the first position with an undefined
    /// atom, and at the first condition the letter reports refused (FR-327
    /// "Refused condition"): an atom undefined earlier in clause-node order
    /// at the same position is returned instead.
    fn read_letters<L: TemporalLetters>(
        &mut self,
        source: &mut L,
        read: u64,
        guarded: bool,
        captured: CaptureSite,
        whens: &mut HashMap<u64, bool>,
    ) -> Result<LettersRead, Stop> {
        let mut insufficient = None;
        for position in 0..read {
            let at = TemporalPosition::new(position);
            let mut activates = true;
            if guarded {
                let reading = match source.activation(at, &mut *self.meter) {
                    Err(LetterStop::Refused) => return Ok(LettersRead::refused(at, insufficient)),
                    other => other?,
                };
                match reading {
                    AtomReading::True => {
                        whens.insert(position, true);
                    }
                    AtomReading::False => {
                        whens.insert(position, false);
                        activates = false;
                    }
                    AtomReading::Undefined(cause) => {
                        return Ok(LettersRead::undefined(
                            Self::undefined(position, cause),
                            insufficient,
                        ))
                    }
                }
            }
            // FR-337 "Capture observation": the captures are fixed once, at
            // an activation: position 0 of an `on origin` clause, each
            // position an `on each` clause's guard admits.
            let activation_position = match captured {
                CaptureSite::Never => false,
                CaptureSite::Origin => position == 0,
                CaptureSite::Activations => true,
            };
            if activates && activation_position {
                let reading = match source.captures(at, &mut *self.meter) {
                    Err(LetterStop::Refused) => return Ok(LettersRead::refused(at, insufficient)),
                    other => other?,
                };
                if let CaptureReading::Undefined { cause, .. } = reading {
                    return Ok(LettersRead::undefined(
                        Self::undefined(position, cause),
                        insufficient,
                    ));
                }
            }
            let mut undefined_at_position = None;
            for index in 0..self.plan.atoms.len() {
                let node = slot(&self.plan.atoms, index)?;
                self.charge()?;
                let id = self
                    .formula
                    .node_id(node)
                    .ok_or_else(|| fault("atom-has-node-id"))?;
                let reading = match source.atom(id, at, &mut *self.meter) {
                    Err(LetterStop::Refused) => {
                        return Ok(match undefined_at_position {
                            Some(cause) => LettersRead::undefined(
                                Self::undefined(position, cause),
                                insufficient,
                            ),
                            None => LettersRead::refused(at, insufficient),
                        })
                    }
                    other => other?,
                };
                match reading {
                    AtomReading::True => {
                        self.letters.insert((node, position), Tri::True);
                    }
                    AtomReading::False => {
                        self.letters.insert((node, position), Tri::False);
                    }
                    AtomReading::Undefined(UndefinedCause::InsufficientData(data)) => {
                        self.insufficient_positions.insert(position);
                        self.letters.insert((node, position), Tri::Unknown);
                        if insufficient.is_none() {
                            insufficient = Some(InsufficientDataEvaluation { at, atom: id, data });
                        }
                    }
                    AtomReading::Undefined(cause) => {
                        if undefined_at_position.is_none() {
                            undefined_at_position = Some(cause);
                        }
                    }
                }
            }
            if let Some(cause) = undefined_at_position {
                return Ok(LettersRead::undefined(
                    Self::undefined(position, cause),
                    insufficient,
                ));
            }
        }
        Ok(LettersRead {
            undefined: None,
            refused: None,
            insufficient,
        })
    }

    fn execute<L: TemporalLetters>(
        &mut self,
        clause: Subject<'_>,
        goal: Goal,
        source: &mut L,
    ) -> Result<TemporalOutcome, Stop> {
        let activation = clause.activation;
        let clause_goal = matches!(goal, Goal::Clause);
        // An `on origin` bounded clause reads positions 0 to its horizon;
        // every other clause reads every represented position (UE-1).
        let read = match (clause_goal, activation, clause.horizon) {
            (true, Activation::Origin, Some(horizon)) => self.through_horizon(horizon)?,
            _ => self.represented,
        };
        let guarded = clause_goal && matches!(activation, Activation::Each { guarded: true });
        let mut whens = HashMap::new();
        let captured = if clause_goal {
            clause.captures
        } else {
            CaptureSite::Never
        };
        let LettersRead {
            undefined,
            refused,
            insufficient,
        } = self.read_letters(source, read, guarded, captured, &mut whens)?;
        // No later position is read: a refused condition stops the reading
        // as an undefined atom does.
        let stopped_at = undefined
            .as_ref()
            .map(|undefined| undefined.at)
            .or(refused)
            .map(TemporalPosition::index);
        self.unknown_from = stopped_at;
        let safety = self.plan.is_safety()?;
        // A prefix decides a violation only for a safety formula whose bad
        // prefix falsifies it; every other prefix is pending (FR-328).
        let evaluates = self.mode != Mode::Prefix || safety;

        let (first, limit) = match goal {
            Goal::At(position) => {
                let at = i128::from(position.index());
                (at, at + 1)
            }
            Goal::Clause => {
                let limit = match (activation, self.mode) {
                    (Activation::Origin, _) => 1,
                    (Activation::Each { .. }, Mode::Lasso) => self.geometry.unrolled,
                    (Activation::Each { .. }, Mode::Closed | Mode::Prefix) => {
                        i128::from(self.represented)
                    }
                };
                // Only activations before the first undefined or refused
                // position can refute the claim before it stops.
                let limit = match stopped_at {
                    Some(at) => limit.min(i128::from(at)),
                    None => limit,
                };
                (0, limit)
            }
        };
        let mut every_activation_true = true;
        if evaluates {
            let mut position = first;
            while position < limit {
                if !self.represents(position) {
                    self.charge()?;
                }
                if guarded {
                    let represented = match self.geometry.site(self.geometry.canon(position))? {
                        Site::Real(represented) => represented,
                        Site::Sentinel => {
                            return Err(fault("activation-position-represented").into())
                        }
                    };
                    match whens.get(&represented) {
                        Some(true) => {}
                        Some(false) => {
                            position += 1;
                            continue;
                        }
                        None => return Err(fault("activation-condition-read").into()),
                    }
                }
                let represented_position =
                    match self.geometry.site(self.geometry.canon(position))? {
                        Site::Real(represented) => represented,
                        Site::Sentinel => {
                            position += 1;
                            continue;
                        }
                    };
                if self.insufficient_positions.contains(&represented_position) {
                    every_activation_true = false;
                    position += 1;
                    continue;
                }
                match self.solve(self.plan.root, position)? {
                    Tri::True => {}
                    Tri::False => {
                        let at = u64::try_from(position).map_err(|_| fault("position-in-u64"))?;
                        if self.seek {
                            self.decisive = Some(self.descend(position));
                        }
                        return Ok(TemporalOutcome::Violated {
                            at: TemporalPosition::new(at),
                        });
                    }
                    Tri::Unknown => every_activation_true = false,
                }
                position += 1;
            }
        }
        if let Some(undefined) = undefined {
            return Ok(TemporalOutcome::Undefined(undefined));
        }
        if let Some(at) = refused {
            return Err(Stop::ConditionRefused(at));
        }
        if let Some(insufficient) = insufficient {
            return Ok(TemporalOutcome::InsufficientData(insufficient));
        }
        match self.mode {
            Mode::Prefix => Ok(TemporalOutcome::Pending),
            Mode::Closed | Mode::Lasso if every_activation_true => Ok(TemporalOutcome::Holds),
            Mode::Closed | Mode::Lasso => Err(fault("definite-trace-read-unknown").into()),
        }
    }

    /// FR-330 "The decisive atom": descend from the root, false at the
    /// failing activation `at`, always into an operand that is false at the
    /// position it is visited at, to the `holds` atom that decides it. `None`
    /// when the descent meets `not`, a constant, a position before the trace
    /// or one past a false-extended trace.
    fn descend(&mut self, at: i128) -> DecisiveSearch {
        self.descend_from(at).map_err(|stop| match stop {
            Stop::Incomplete(incomplete) => DecisiveStop::Incomplete(incomplete),
            Stop::Fault(internal) => DecisiveStop::Fault(internal),
            Stop::ConditionRefused(_) | Stop::WindowLimit(_) | Stop::Window(_) => {
                DecisiveStop::Fault(fault("decisive-descent-reads-no-letter"))
            }
        })
    }

    fn descend_from(&mut self, at: i128) -> Result<Option<DecisiveAtom>, Stop> {
        let mut node = self.plan.root;
        let mut at = at;
        loop {
            match slot(&self.plan.nodes, node)? {
                Node::Constant(_) | Node::Not(_) => return Ok(None),
                Node::Atom => return self.decisive_atom(node, at),
                Node::And(left, right) => {
                    node = if self.solve(left, at)? == Tri::False {
                        left
                    } else {
                        right
                    };
                }
                Node::Or(left, _) => node = left,
                Node::Implies(_, right) => node = right,
                Node::Scan(scan) => match self.descend_scan(scan, at)? {
                    Some((operand, position)) => {
                        node = operand;
                        at = position;
                    }
                    None => return Ok(None),
                },
            }
        }
    }

    /// The atom `node` read at `q`, when `q` is a position of the trace: a
    /// position before 0 and a false-extension position have no observation.
    /// Over a lasso the position is the in-trace one `q` reads (FR-330).
    fn decisive_atom(&self, node: usize, q: i128) -> Result<Option<DecisiveAtom>, Stop> {
        if q < 0 {
            return Ok(None);
        }
        Ok(match self.geometry.site(self.geometry.canon(q))? {
            Site::Sentinel => None,
            Site::Real(position) => Some(DecisiveAtom {
                atom: self
                    .formula
                    .node_id(node)
                    .ok_or_else(|| fault("atom-has-node-id"))?,
                position: TemporalPosition::new(position),
            }),
        })
    }

    /// The operand and position `scan` at `p` descends into, over its
    /// eligible positions in order (a past operator takes the nearest first).
    fn descend_scan(&mut self, scan: Scan, p: i128) -> Result<Option<(usize, i128)>, Stop> {
        let run = self.window(scan, p)?;
        let mut first = None;
        for index in 0..run.len {
            let q = run.at(index)?;
            if !self.represents(q) {
                self.charge()?;
            }
            first.get_or_insert(q);
            match (scan.guard, scan.dual) {
                // `eventually`, `once`: every eligible position is false.
                (None, false) => break,
                // `always`, `historically`, `release`, `triggered`: the least
                // eligible position where the witness is false.
                (None, true) | (Some(_), true) => {
                    if self.solve(scan.witness, q)? == Tri::False {
                        return Ok(Some((scan.witness, q)));
                    }
                }
                // `until`, `since`: the least position where both operands
                // are false.
                (Some(guard), false) => {
                    if self.solve(guard, q)? == Tri::False
                        && self.solve(scan.witness, q)? == Tri::False
                    {
                        return Ok(Some((guard, q)));
                    }
                }
            }
        }
        // Eventually/once, and until/since with no such position, descend
        // into the witness at the least eligible position.
        if scan.dual {
            return Err(fault("a-false-dual-scan-has-a-false-witness").into());
        }
        Ok(first.map(|q| (scan.witness, q)))
    }

    /// The letter of `node` at `q`.
    fn letter(&self, node: usize, q: i128) -> Result<Tri, Stop> {
        if q < 0 {
            return Ok(Tri::False);
        }
        match self.geometry.site(self.geometry.canon(q))? {
            Site::Sentinel => Ok(self.sentinel),
            Site::Real(position) => {
                if self.unknown_from.is_some_and(|from| position >= from) {
                    return Ok(Tri::Unknown);
                }
                self.letters
                    .get(&(node, position))
                    .copied()
                    .ok_or_else(|| fault("letter-read").into())
            }
        }
    }

    /// The canonical position of `node` at `q`: positions far enough before
    /// 0 share one value, and positions past the unrolled word wrap.
    fn norm(&self, node: usize, q: i128) -> Result<i128, InternalFault> {
        if q < 0 {
            Ok(q.max(slot(&self.floors, node)?))
        } else {
            Ok(self.geometry.canon(q))
        }
    }

    fn lookup(&self, node: usize, q: i128) -> Result<Lookup, Stop> {
        Ok(match slot(&self.plan.nodes, node)? {
            Node::Constant(value) => Lookup::Ready(Tri::of(value)),
            Node::Atom => Lookup::Ready(self.letter(node, q)?),
            Node::Not(_) | Node::And(..) | Node::Or(..) | Node::Implies(..) | Node::Scan(_) => {
                let position = self.norm(node, q)?;
                match self.memo.get(&(node, position)) {
                    Some(value) => Lookup::Ready(*value),
                    None => Lookup::Missing(node, position),
                }
            }
        })
    }

    /// The value of `node` at `q`, visiting what it has not yet visited.
    fn solve(&mut self, node: usize, q: i128) -> Result<Tri, Stop> {
        let (node, position) = match self.lookup(node, q)? {
            Lookup::Ready(value) => return Ok(value),
            Lookup::Missing(node, position) => (node, position),
        };
        let first = self.frame(node, position)?;
        let mut stack = vec![first];
        loop {
            let Some(top) = stack.last_mut() else {
                return Err(fault("frame-stack-not-empty").into());
            };
            match self.advance(top)? {
                Advance::Need(child, at) => {
                    let frame = self.frame(child, at)?;
                    stack.push(frame);
                }
                Advance::Done(value) => {
                    let Some(done) = stack.pop() else {
                        return Err(fault("frame-stack-not-empty").into());
                    };
                    self.memo.insert((done.node, done.pos), value);
                    if stack.is_empty() {
                        return Ok(value);
                    }
                }
            }
        }
    }

    /// Open the visit of `node` at the canonical `pos`, charging it first.
    fn frame(&mut self, node: usize, pos: i128) -> Result<Frame, Stop> {
        self.charge()?;
        let window = match slot(&self.plan.nodes, node)? {
            Node::Scan(scan) => Some(self.window(scan, pos)?),
            Node::Constant(_)
            | Node::Atom
            | Node::Not(_)
            | Node::And(..)
            | Node::Or(..)
            | Node::Implies(..) => None,
        };
        Ok(Frame {
            node,
            pos,
            index: 0,
            scanned: 0,
            acc: Tri::False,
            guard: Tri::True,
            window,
        })
    }

    /// The window of positions a scan at `p` runs over.
    ///
    /// A window is capped where the word repeats: past `max(start, N) + L - 1`
    /// every position has been read once, and a repeat can neither add a
    /// witness nor weaken a guard that the first pass left standing. An open
    /// window is the limit of the closed one: it starts at the lower bound and
    /// keeps the closed form's left-operand range `[a, d)`, so one window code
    /// serves both.
    fn window(&self, scan: Scan, p: i128) -> Result<Run, InternalFault> {
        let geometry = self.geometry;
        let n = geometry.unrolled;
        let l = geometry.period;
        let witness_floor = slot(&self.floors, scan.witness)?;
        let floor = match scan.guard {
            Some(guard) => witness_floor.min(slot(&self.floors, guard)?),
            None => witness_floor,
        };
        Ok(match (scan.base.direction(), scan.window) {
            (Direction::Future, Window::Closed { lower, upper }) => {
                // No position of a loop without a counted step is as far as
                // the lower bound: the window is empty.
                let Some(start) = self.ahead(p, lower)? else {
                    return Ok(Run::linear(0, 1, 0));
                };
                let cap = start.max(n) + l - 1;
                let last = self.ahead_last(p, upper)?.map_or(cap, |last| last.min(cap));
                Run::linear(start, 1, span(start, last)?)
            }
            (Direction::Past, Window::Closed { lower, upper }) => {
                let first = self.behind(p, lower)?;
                let farthest = self.behind_far(p, upper)?.max(first.min(floor));
                Run::linear(first, -1, span(farthest, first)?)
            }
            (Direction::Future, Window::From { lower }) => {
                let Some(s0) = self.ahead(p, lower)? else {
                    return Ok(Run::linear(0, 1, 0));
                };
                if s0 < 0 {
                    let start = s0.max(floor);
                    Run::linear(start, 1, span(start, n - 1)?)
                } else {
                    let start = geometry.canon(s0);
                    if start < n - l {
                        Run::linear(start, 1, span(start, n - 1)?)
                    } else {
                        Run {
                            start,
                            step: 1,
                            len: span(n - l, n - 1)?,
                            wrap: Some(Wrap {
                                base: n - l,
                                period: l,
                            }),
                        }
                    }
                }
            }
            (Direction::Past, Window::From { lower }) => {
                let s0 = self.behind(p, lower)?;
                if s0 < floor {
                    Run::linear(floor, -1, 1)
                } else {
                    Run::linear(s0, -1, span(floor, s0)?)
                }
            }
        })
    }

    /// The last position to read of a bounded `on origin` clause with
    /// horizon `horizon` (ADR-018 TP-2), counted in steps (ADR-027 PB-3),
    /// as a count of positions. A lasso whose loop has no counted step never
    /// leaves the horizon: it reads every represented position.
    fn through_horizon(&self, horizon: u64) -> Result<u64, InternalFault> {
        let Some(steps) = &self.steps else {
            return Ok(horizon.saturating_add(1).min(self.represented));
        };
        let Some(last) = steps.last_within(i128::from(horizon))? else {
            return Ok(self.represented);
        };
        let positions = u64::try_from(last + 1).map_err(|_| fault("horizon-position-in-u64"))?;
        Ok(positions.min(self.represented))
    }

    /// The first position at distance `offset` or more at or after `p`;
    /// `None` when no position of the lasso is that far.
    fn ahead(&self, p: i128, offset: u64) -> Result<Option<i128>, InternalFault> {
        match &self.steps {
            None => Ok(Some(p + i128::from(offset))),
            Some(steps) => Ok(steps
                .first_reaching(steps.to(p)? + i128::from(offset))?
                .map(|first| first.max(p))),
        }
    }

    /// The last position at distance `offset` or less after `p`; `None` when
    /// every position of the lasso after some point is.
    fn ahead_last(&self, p: i128, offset: u64) -> Result<Option<i128>, InternalFault> {
        match &self.steps {
            None => Ok(Some(p + i128::from(offset))),
            Some(steps) => steps.last_within(steps.to(p)? + i128::from(offset)),
        }
    }

    /// The last position at distance `offset` or more at or before `p`.
    fn behind(&self, p: i128, offset: u64) -> Result<i128, InternalFault> {
        match &self.steps {
            None => Ok(p - i128::from(offset)),
            Some(steps) => Ok(steps
                .last_within(steps.to(p)? - i128::from(offset))?
                .map_or(p, |last| last.min(p))),
        }
    }

    /// The first position at distance `offset` or less before `p`.
    fn behind_far(&self, p: i128, offset: u64) -> Result<i128, InternalFault> {
        match &self.steps {
            None => Ok(p - i128::from(offset)),
            Some(steps) => steps
                .first_reaching(steps.to(p)? - i128::from(offset))?
                .ok_or_else(|| fault("a-position-before-p-is-within-reach")),
        }
    }

    /// Run `frame` as far as the visits already made allow: to its value,
    /// or to the first operand visit it still needs.
    fn advance(&mut self, frame: &mut Frame) -> Result<Advance, Stop> {
        let pos = frame.pos;
        match slot(&self.plan.nodes, frame.node)? {
            Node::Constant(_) | Node::Atom => Err(fault("leaf-has-no-visit").into()),
            Node::Not(operand) => Ok(Advance::Done(ready!(self, operand, pos).negate())),
            Node::And(left, right) => {
                let first = ready!(self, left, pos);
                if first == Tri::False {
                    return Ok(Advance::Done(Tri::False));
                }
                Ok(Advance::Done(first.and(ready!(self, right, pos))))
            }
            Node::Or(left, right) => {
                let first = ready!(self, left, pos);
                if first == Tri::True {
                    return Ok(Advance::Done(Tri::True));
                }
                Ok(Advance::Done(first.or(ready!(self, right, pos))))
            }
            Node::Implies(left, right) => {
                let first = ready!(self, left, pos).negate();
                if first == Tri::True {
                    return Ok(Advance::Done(Tri::True));
                }
                Ok(Advance::Done(first.or(ready!(self, right, pos))))
            }
            Node::Scan(scan) => self.advance_scan(scan, frame),
        }
    }

    /// One scan: `acc` is the disjunction of the witnesses seen, each
    /// conjoined with `guard`, the conjunction of the left operand over the
    /// positions before it (`until`/`since`); a dual reads negated operands
    /// and negates the result. Every update is idempotent, so a visit
    /// suspended on an operand repeats its step when resumed.
    fn advance_scan(&mut self, scan: Scan, frame: &mut Frame) -> Result<Advance, Stop> {
        let done = |acc: Tri| Ok(Advance::Done(acc.flip(scan.dual)));
        let Some(run) = frame.window else {
            return Err(fault("scan-has-window").into());
        };
        loop {
            if frame.index >= run.len {
                return done(frame.acc);
            }
            let q = run.at(frame.index)?;
            // A position the trace does not represent (past it, unrolled
            // copies of the loop, or before 0) carries no letter charge of
            // its own, so each such step is paid for before it runs.
            if frame.scanned == frame.index {
                frame.scanned += 1;
                if !self.represents(q) {
                    self.charge()?;
                }
            }
            let witness = ready!(self, scan.witness, q).flip(scan.dual);
            match scan.guard {
                None => frame.acc = frame.acc.or(witness),
                Some(guard) => {
                    frame.acc = frame.acc.or(witness.and(frame.guard));
                    if frame.acc != Tri::True && frame.index + 1 < run.len {
                        let value = ready!(self, guard, q).flip(scan.dual);
                        frame.guard = frame.guard.and(value);
                        if frame.guard == Tri::False {
                            return done(frame.acc);
                        }
                    }
                }
            }
            if frame.acc == Tri::True {
                return done(Tri::True);
            }
            frame.index += 1;
        }
    }

    /// Whether `q` is a position of the trace itself.
    fn represents(&self, q: i128) -> bool {
        (0..i128::from(self.represented)).contains(&q)
    }
}

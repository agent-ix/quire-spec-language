// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-175 (ADR-023 HM-1, HM-2, HM-4, HM-5, HM-8): a hyper clause's body
//! evaluated over a tuple of behaviours in lockstep.
//!
//! A tuple of lassos is one lasso. Joint position `j` is position `j` of
//! every component; the joint stem is the longest component stem and the
//! joint period the least common multiple of the component loop lengths, so
//! the tuple is ultimately periodic and the one evaluator of this module's
//! parent reads it (ADR-018 SM-1, SM-8): the unrolling, the past operators,
//! the interval counting of joint positions and the work charge are that
//! evaluator's, not a second definition. This module owns only the
//! component indexing (ADR-014 TR-2's prefix-then-loop rule) and the
//! terminal stutter of a component that ends (ADR-023 HM-5).
//!
//! The traces are fair by the caller's premise: the domain of a variable is
//! its subject's fair behaviours (HM-1), checked by the engine that produced
//! the tuple and by replay (FR-183), so the body is evaluated with no
//! fairness set to check.

use std::collections::HashMap;

use qsl_semantics::check::{Activation, CheckedHyperClause, FormulaNodeId};
use quire_exact::{Cancel, Meter, ScalarLimits};

use super::engine::{self, bad_prefix_bound, Goal, Subject};
use super::input::TemporalTrace;
use super::letters::{AtomReading, LetterStop, TemporalLetters};
use super::outcome::{TemporalOutcome, TemporalRefusal};
use super::plan::fault;
use super::position::TemporalPosition;

/// One component trace of a tuple (FR-175 "Inputs"): a model trace as FR-125
/// reads it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ComponentTrace {
    /// A lasso: `prefix` positions, then `loop_len >= 1` positions whose end
    /// re-enters position `prefix`.
    Lasso {
        /// The number of positions before the loop.
        prefix: u64,
        /// The number of positions in the loop.
        loop_len: u64,
    },
    /// A behaviour that ends at a terminal state: `len >= 1` positions, the
    /// last of them terminal. Every later joint position reads that state and
    /// takes the stutter step (ADR-023 HM-5).
    Terminal {
        /// The number of represented positions.
        len: u64,
    },
}

impl ComponentTrace {
    /// The component as the lasso it indexes by. A terminal trace is a lasso
    /// whose loop is one stutter position after its last state, so the joint
    /// position of every later step is a loop copy of the stutter step.
    fn lasso(self) -> Result<(u64, u64), TemporalRefusal> {
        match self {
            Self::Lasso { loop_len: 0, .. } => Err(TemporalRefusal::EmptyLoop),
            Self::Lasso { prefix, loop_len } => Ok((prefix, loop_len)),
            Self::Terminal { len: 0 } => Err(TemporalRefusal::EmptyTrace),
            Self::Terminal { len } => Ok((len, 1)),
        }
    }

    /// Where joint position `joint` reads this component, and whether the
    /// step into `joint` is the stutter step.
    fn read(self, joint: u64) -> Result<ComponentPosition, TemporalRefusal> {
        let (prefix, loop_len) = self.lasso()?;
        let index = if let Self::Terminal { len } = self {
            // The stutter position reads the terminal state.
            joint.min(len - 1)
        } else if joint < prefix {
            joint
        } else {
            // `joint - prefix` is the offset into the unrolled loop.
            prefix + (joint - prefix) % loop_len
        };
        let stutter = matches!(self, Self::Terminal { len } if joint >= len);
        Ok(ComponentPosition {
            position: TemporalPosition::new(index),
            stutter,
        })
    }
}

/// Where one component is at a joint position.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ComponentPosition {
    /// The component's own position (ADR-014 TR-2).
    pub position: TemporalPosition,
    /// Whether the component's step into this joint position is the
    /// `stutter` step of a terminal component.
    pub stutter: bool,
}

/// Reads the body's atoms at a joint position: each member `e @ t` reads
/// component `t`'s observation at that component's position (FR-175
/// "Indexed atoms"). Every method charges the caller's `meter` for its own
/// work, as [`TemporalLetters`] does.
pub trait TupleLetters {
    /// The `holds` atom `atom` at joint position `joint`, whose components
    /// are `components` in quantifier order.
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        joint: TemporalPosition,
        components: &[ComponentPosition],
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop>;
}

/// The joint lasso of a tuple: its stem and its period.
fn joint(components: &[ComponentTrace]) -> Result<TemporalTrace, TemporalRefusal> {
    if components.is_empty() {
        return Err(TemporalRefusal::EmptyTrace);
    }
    let mut prefix = 0u64;
    let mut period = 1u64;
    for component in components {
        let (stem, loop_len) = component.lasso()?;
        prefix = prefix.max(stem);
        let divisor = gcd(period, loop_len);
        period = (period / divisor)
            .checked_mul(loop_len)
            .ok_or(TemporalRefusal::TraceTooLong)?;
    }
    Ok(TemporalTrace::Lasso {
        prefix,
        loop_len: period,
    })
}

fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

/// The seam that turns the joint position into component positions. The
/// joint word is periodic from its stem, so a joint position at or past the
/// represented length reads the congruent represented position: a reading is
/// made once per (atom, represented position), through the caller's meter in
/// the first pass, and the position search below, which evaluates the body
/// over longer and shorter prefixes of the unrolled word, takes every reading
/// from the record and reaches no letter.
struct Lockstep<'a, L> {
    components: &'a [ComponentTrace],
    letters: &'a mut L,
    /// The joint stem and period.
    stem: u64,
    period: u64,
    scratch: Vec<ComponentPosition>,
    readings: HashMap<(FormulaNodeId, u64), AtomReading>,
    /// Set once the first pass is done: no further reading reaches the
    /// caller's letters.
    sealed: bool,
}

impl<L> Lockstep<'_, L> {
    /// Where every component stands at joint position `joint`.
    fn reads(&self, joint: u64) -> Result<Vec<ComponentPosition>, TemporalRefusal> {
        self.components
            .iter()
            .map(|component| component.read(joint))
            .collect()
    }

    /// The represented joint position whose reading is `joint`'s: the same
    /// component positions and the same stutter flags, since both are
    /// periodic from the joint stem.
    fn congruent(&self, joint: u64) -> u64 {
        if joint < self.stem + self.period {
            joint
        } else {
            self.stem + (joint - self.stem) % self.period
        }
    }
}

impl<L: TupleLetters> TemporalLetters for Lockstep<'_, L> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let keyed = self.congruent(position.index());
        if let Some(reading) = self.readings.get(&(atom, keyed)) {
            return Ok(reading.clone());
        }
        if self.sealed {
            // The position search takes every reading from the first pass.
            return Err(LetterStop::Fault(fault("search-reads-recorded-readings")));
        }
        self.scratch = self
            .reads(position.index())
            .map_err(|_| LetterStop::Fault(fault("tuple-component-readable")))?;
        let reading = self.letters.atom(atom, position, &self.scratch, meter)?;
        self.readings.insert((atom, keyed), reading.clone());
        Ok(reading)
    }

    fn activation(
        &mut self,
        _position: TemporalPosition,
        _meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        Err(LetterStop::Fault(fault("hyper-body-activates-on-origin")))
    }
}

fn subject(body: &CheckedHyperClause) -> Subject<'_> {
    Subject {
        formula: body.body(),
        profile: body.profile(),
        activation: Activation::Origin,
        horizon: None,
        fairness: &[],
    }
}

/// The first joint position at which the body's `false` is fixed (FR-175
/// "Outputs"): the shortest joint prefix that is already a bad prefix of the
/// body (ADR-018 SM-6), searched over the unrolled word (ADR-018 SM-8) and
/// the body's future reach beyond it. A body whose falsity no finite prefix
/// shows, a liveness body, is fixed only when the joint loop closes, at the
/// last represented position.
///
/// The search evaluates the body on shorter and longer prefixes. It reaches
/// no letter: `lockstep` is sealed, so every atom it reads is a reading the
/// first pass made and a miss is a fault, never hidden letter work. Its node
/// visits are the search's own and run on a meter with the caller's limits
/// but no work cap, so locating `trace_position` does not charge the caller's
/// work (one unit per node and joint position, FR-175 "Work").
fn false_position<L: TupleLetters>(
    body: &CheckedHyperClause,
    trace: TemporalTrace,
    represented: u64,
    lockstep: &mut Lockstep<'_, L>,
    meter: &Meter,
    cancel: Option<&Cancel>,
) -> Result<Result<TemporalPosition, TemporalOutcome>, TemporalRefusal> {
    let longest = u64::try_from(bad_prefix_bound(subject(body), trace)?)
        .map_err(|_| TemporalRefusal::TraceTooLong)?;
    // `Err` carries the outcome of a search step the meter or a window limit
    // stopped: it has no truth value, so it ends the evaluation.
    lockstep.sealed = true;
    let mut search = Meter::new(ScalarLimits {
        work_units: u64::MAX,
        ..*meter.limits()
    });
    if let Some(cancel) = cancel {
        search = search.with_cancel(cancel.clone());
    }
    let mut bad = |len: u64| {
        engine::run(
            subject(body),
            TemporalTrace::Finite { len },
            Goal::Clause,
            lockstep,
            &mut search,
        )
        .map(|outcome| match outcome {
            TemporalOutcome::Violated { .. } => Ok(true),
            stopped @ (TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_)) => {
                Err(stopped)
            }
            _ => Ok(false),
        })
    };
    // A bad prefix stays bad as it grows, so the shortest one is found by
    // bisection: `low` is not bad, `high` is.
    match bad(longest)? {
        Err(stopped) => return Ok(Err(stopped)),
        Ok(false) => return Ok(Ok(TemporalPosition::new(represented - 1))),
        Ok(true) => {}
    }
    let (mut low, mut high) = (0, longest);
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        match bad(middle)? {
            Err(stopped) => return Ok(Err(stopped)),
            Ok(true) => high = middle,
            Ok(false) => low = middle,
        }
    }
    Ok(Ok(TemporalPosition::new(high - 1)))
}

/// Evaluate `body` over the tuple `components`, one per variable in
/// quantifier order (FR-175).
///
/// The outcome is the temporal evaluator's on the joint behaviour: `Holds`;
/// `Violated`, whose `at` is the first joint position that fixes the
/// `false`, the `trace_position` of a counterexample, not the activation;
/// `Undefined` at the first joint position an atom is undefined, with its
/// cause. `cancel` is the handle `meter` polls, if any; the position search
/// polls it too, so a cancellation during the search is seen. A tuple whose count differs from the clause's variables, an empty
/// component, or a joint period that exceeds `u64` is refused.
pub fn evaluate_tuple<L: TupleLetters>(
    body: &CheckedHyperClause,
    components: &[ComponentTrace],
    letters: &mut L,
    meter: &mut Meter,
    cancel: Option<&Cancel>,
) -> Result<TemporalOutcome, TemporalRefusal> {
    if components.len() != body.quantifiers().len() {
        return Err(TemporalRefusal::TupleArity {
            expected: body.quantifiers().len(),
            found: components.len(),
        });
    }
    if !body.align_skip().is_empty() {
        return Err(TemporalRefusal::AlignSkipNotImplemented);
    }
    let trace = joint(components)?;
    let TemporalTrace::Lasso { prefix, loop_len } = trace else {
        return Err(TemporalRefusal::Fault(fault("joint-trace-is-a-lasso")));
    };
    let represented = trace.represented().ok_or(TemporalRefusal::TraceTooLong)?;
    let mut lockstep = Lockstep {
        components,
        letters,
        stem: prefix,
        period: loop_len,
        scratch: Vec::with_capacity(components.len()),
        readings: HashMap::new(),
        sealed: false,
    };
    let outcome = engine::run(subject(body), trace, Goal::Clause, &mut lockstep, meter)?;
    match outcome {
        TemporalOutcome::Violated { .. } => Ok(
            match false_position(body, trace, represented, &mut lockstep, meter, cancel)? {
                Ok(at) => TemporalOutcome::Violated { at },
                Err(stopped) => stopped,
            },
        ),
        other => Ok(other),
    }
}

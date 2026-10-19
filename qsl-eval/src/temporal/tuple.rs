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

use qsl_semantics::check::{Activation, CheckedHyperClause, FormulaNodeId};
use quire_exact::Meter;

use super::engine::{self, Goal, Subject};
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
    /// The component as the lasso it indexes by: a terminal trace repeats its
    /// last position.
    fn lasso(self) -> Result<(u64, u64), TemporalRefusal> {
        match self {
            Self::Lasso { loop_len: 0, .. } => Err(TemporalRefusal::EmptyLoop),
            Self::Lasso { prefix, loop_len } => Ok((prefix, loop_len)),
            Self::Terminal { len: 0 } => Err(TemporalRefusal::EmptyTrace),
            Self::Terminal { len } => Ok((len - 1, 1)),
        }
    }

    /// Where joint position `joint` reads this component, and whether the
    /// step into `joint` is the stutter step.
    fn read(self, joint: u64) -> Result<ComponentPosition, TemporalRefusal> {
        let (prefix, loop_len) = self.lasso()?;
        let index = if joint < prefix {
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

/// The seam that turns the joint position into component positions.
struct Lockstep<'a, L> {
    components: &'a [ComponentTrace],
    letters: &'a mut L,
    scratch: Vec<ComponentPosition>,
}

impl<L: TupleLetters> TemporalLetters for Lockstep<'_, L> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        position: TemporalPosition,
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        self.scratch.clear();
        for component in self.components {
            let read = component
                .read(position.index())
                .map_err(|_| LetterStop::Fault(fault("tuple-component-readable")))?;
            self.scratch.push(read);
        }
        self.letters.atom(atom, position, &self.scratch, meter)
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
/// body, read three-valued by the one evaluator (ADR-018 SM-6). A body whose
/// falsity no finite prefix shows, a liveness body, is fixed only when the
/// joint loop closes, at the last represented joint position.
fn false_position<L: TemporalLetters>(
    body: &CheckedHyperClause,
    represented: u64,
    letters: &mut L,
    meter: &mut Meter,
) -> Result<TemporalPosition, TemporalRefusal> {
    let bad = |len: u64, letters: &mut L, meter: &mut Meter| {
        engine::run(
            subject(body),
            TemporalTrace::Finite { len },
            Goal::Clause,
            letters,
            meter,
        )
        .map(|outcome| matches!(outcome, TemporalOutcome::Violated { .. }))
    };
    let last = TemporalPosition::new(represented - 1);
    // A bad prefix stays bad as it grows, so the shortest one is found by
    // bisection: `low` is not bad, `high` is.
    if !bad(represented, letters, meter)? {
        return Ok(last);
    }
    let (mut low, mut high) = (0, represented);
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        if bad(middle, letters, meter)? {
            high = middle;
        } else {
            low = middle;
        }
    }
    Ok(TemporalPosition::new(high - 1))
}

/// Evaluate `body` over the tuple `components`, one per variable in
/// quantifier order (FR-175).
///
/// The outcome is the temporal evaluator's on the joint behaviour: `Holds`;
/// `Violated`, whose `at` is the first joint position that fixes the
/// `false`, the `trace_position` of a counterexample, not the activation;
/// `Undefined` at the first joint position an atom is undefined, with its
/// cause. A tuple whose count differs from the clause's variables, an empty
/// component, or a joint period that exceeds `u64` is refused.
pub fn evaluate_tuple<L: TupleLetters>(
    body: &CheckedHyperClause,
    components: &[ComponentTrace],
    letters: &mut L,
    meter: &mut Meter,
) -> Result<TemporalOutcome, TemporalRefusal> {
    if components.len() != body.quantifiers().len() {
        return Err(TemporalRefusal::TupleArity {
            expected: body.quantifiers().len(),
            found: components.len(),
        });
    }
    let trace = joint(components)?;
    let represented = trace.represented().ok_or(TemporalRefusal::TraceTooLong)?;
    let mut lockstep = Lockstep {
        components,
        letters,
        scratch: Vec::with_capacity(components.len()),
    };
    let outcome = engine::run(subject(body), trace, Goal::Clause, &mut lockstep, meter)?;
    match outcome {
        TemporalOutcome::Violated { .. } => Ok(TemporalOutcome::Violated {
            at: false_position(body, represented, &mut lockstep, meter)?,
        }),
        other => Ok(other),
    }
}

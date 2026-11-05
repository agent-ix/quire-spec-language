// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-523: the per-call counters and the meter they sit in front of.
//!
//! [`CallMeter`] is what a temporal evidence call charges instead of the
//! bare [`Meter`]: every unit of work names the counters it spends, each
//! counter is charged before the meter, and a charge any counter refuses is
//! not performed, raises nothing and leaves the meter's charge log as it
//! was. One call has one [`CallMeter`], so the counters are per call; the
//! [`Meter`] under it is the caller's and may be shared by several calls,
//! whose charges it accumulates.

use qsl_foundation::diagnostic::LimitKind;
use qsl_foundation::Setting;
use quire_exact::{Charge, ChargePoint, Incomplete, Meter};

use super::limits::{CounterKind, EvidenceCounter, EvidenceUsage, TemporalEvidenceLimits};
use super::work::{ClauseWork, ReachFault};
use qsl_semantics::check::CheckedFormula;

/// A counter reached its ceiling: `Incomplete(LimitReached{limit, value,
/// setting})` of FR-523 "Outputs".
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LimitReached {
    /// The counter whose charge was refused.
    pub counter: EvidenceCounter,
    /// The limit kind of FR-255 for the counter's setting.
    pub limit: LimitKind,
    /// The ceiling, as the call was given it.
    pub value: u64,
    /// The counter's setting name.
    pub setting: Setting,
    /// The opaque identity of the assessed instance, when the stop is at the
    /// instance. The clause-level stops (`formula_nodes`, `history_span`
    /// and tick arithmetic) assess no instance and carry none.
    pub instance: Option<Box<[u8]>>,
}

/// Why a charge of the call was refused. Neither carries a truth value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceStop {
    /// One of the thirteen counters reached its ceiling.
    Limit(LimitReached),
    /// Every counter admitted the charge and the meter denied the
    /// `temporal.visit` charge (FR-327), so the stop is at that charge point.
    Meter {
        /// The meter's denial, with its charge point.
        incomplete: Incomplete,
        /// The opaque identity of the assessed instance.
        instance: Option<Box<[u8]>>,
    },
}

/// One unit of work that is charged at `temporal.visit` (FR-523 "Counters").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visit {
    /// A visit of a temporal node at a position, or the resolution of one
    /// delivery or origin: `evaluation_steps` only.
    Step,
    /// A position scanned to determine eligibility: `positions`,
    /// `evaluation_steps` and `visited`.
    Scan,
    /// An atomic `holds` lookup, including one that finds no valuation:
    /// `valuations`, `evaluation_steps` and `visited`.
    Lookup,
    /// A capture established and retained: `captures`, `evaluation_steps`
    /// and `visited`.
    Capture,
    /// An evicted record consulted (FR-524): `evaluation_steps` and
    /// `visited`.
    Evicted,
    /// An atomic lookup at a silent sample (FR-504): `valuations` and
    /// `visited`, and the meter at `temporal.visit`. A silent sample is not
    /// an observed position, so no charge made at one spends
    /// `evaluation_steps` or `positions` (FR-523).
    SilentLookup,
    /// An operator node visited at a silent sample: the meter at
    /// `temporal.visit` only (FR-523).
    SilentStep,
    /// A silent sample scanned by an operator node: the meter only.
    SilentScan,
}

impl Visit {
    /// The counters one unit of this kind spends, in counter-table order.
    const fn spends(self) -> &'static [EvidenceCounter] {
        match self {
            Self::Step => &[EvidenceCounter::EvaluationSteps],
            Self::Scan => &[
                EvidenceCounter::Positions,
                EvidenceCounter::EvaluationSteps,
                EvidenceCounter::Visited,
            ],
            Self::Lookup => &[
                EvidenceCounter::Valuations,
                EvidenceCounter::EvaluationSteps,
                EvidenceCounter::Visited,
            ],
            Self::Capture => &[
                EvidenceCounter::Captures,
                EvidenceCounter::EvaluationSteps,
                EvidenceCounter::Visited,
            ],
            Self::Evicted => &[EvidenceCounter::EvaluationSteps, EvidenceCounter::Visited],
            Self::SilentLookup => &[EvidenceCounter::Valuations, EvidenceCounter::Visited],
            Self::SilentStep | Self::SilentScan => &[],
        }
    }
}

/// The meter of one temporal evidence call, with its thirteen counters.
#[derive(Debug)]
pub struct CallMeter<'m> {
    meter: &'m mut Meter,
    limits: TemporalEvidenceLimits,
    usage: EvidenceUsage,
    instance: Option<Box<[u8]>>,
}

impl<'m> CallMeter<'m> {
    /// A call under `limits`, charging `meter`.
    pub fn new(limits: TemporalEvidenceLimits, meter: &'m mut Meter) -> Self {
        Self {
            meter,
            limits,
            usage: EvidenceUsage::default(),
            instance: None,
        }
    }

    /// The ceilings of the call.
    pub const fn limits(&self) -> &TemporalEvidenceLimits {
        &self.limits
    }

    /// The counters' measured values so far.
    pub const fn usage(&self) -> EvidenceUsage {
        self.usage
    }

    /// The meter under the call.
    pub fn meter(&self) -> &Meter {
        self.meter
    }

    /// The meter under the call, for a charge that is not one of the
    /// counters' units.
    pub fn meter_mut(&mut self) -> &mut Meter {
        self.meter
    }

    /// The call now assesses the instance named by the opaque `identity`: a
    /// later stop names it (FR-523 "Stop scope").
    pub fn assess(&mut self, identity: impl Into<Box<[u8]>>) {
        self.instance = Some(identity.into());
    }

    fn reached(&self, counter: EvidenceCounter) -> LimitReached {
        LimitReached {
            counter,
            limit: counter.limit_kind(),
            value: self.limits.ceiling(counter),
            setting: counter.setting(),
            instance: self.instance.clone(),
        }
    }

    /// The usage after `charges`, or the first counter, in the order given,
    /// whose ceiling a charge exceeds. Nothing is raised on a refusal.
    fn admit(
        &self,
        charges: impl IntoIterator<Item = (EvidenceCounter, u64)>,
    ) -> Result<EvidenceUsage, LimitReached> {
        let mut next = self.usage;
        for (counter, amount) in charges {
            let proposed = match counter.kind() {
                CounterKind::Cumulative => next.get(counter).checked_add(amount),
                CounterKind::Peak => Some(next.get(counter).max(amount)),
            };
            match proposed {
                Some(value) if value <= self.limits.ceiling(counter) => {
                    next = next.with(counter, value);
                }
                Some(_) | None => return Err(self.reached(counter)),
            }
        }
        Ok(next)
    }

    /// Charge `charges`, or refuse naming the first counter that cannot
    /// take its charge.
    ///
    /// # Errors
    ///
    /// [`LimitReached`] for the first counter whose ceiling the charge
    /// exceeds; no counter is raised.
    pub fn charge(&mut self, charges: &[(EvidenceCounter, u64)]) -> Result<(), LimitReached> {
        self.usage = self.admit(charges.iter().copied())?;
        Ok(())
    }

    /// Charge one unit of `visit`: the counters it spends, then the meter's
    /// `temporal.visit` charge point.
    ///
    /// # Errors
    ///
    /// [`EvidenceStop::Limit`] when a counter refuses, naming it, and
    /// [`EvidenceStop::Meter`] when every counter admitted the unit and the
    /// meter denied it. A refused unit leaves every counter and the meter as
    /// they were: the operation is not performed.
    pub fn visit(&mut self, visit: Visit) -> Result<(), EvidenceStop> {
        let next = self
            .admit(visit.spends().iter().map(|counter| (*counter, 1)))
            .map_err(EvidenceStop::Limit)?;
        self.meter
            .charge(Charge::new(ChargePoint::TemporalVisit))
            .map_err(|incomplete| EvidenceStop::Meter {
                incomplete,
                instance: self.instance.clone(),
            })?;
        self.usage = next;
        Ok(())
    }

    /// Admit a visit against an evaluator-owned meter, committing counters
    /// only after that meter admits the same unit.
    pub(super) fn visit_with_meter(
        &mut self,
        visit: Visit,
        meter: &mut Meter,
    ) -> Result<(), EvidenceStop> {
        let next = self
            .admit(visit.spends().iter().map(|counter| (*counter, 1)))
            .map_err(EvidenceStop::Limit)?;
        meter
            .charge(Charge::new(ChargePoint::TemporalVisit))
            .map_err(|incomplete| EvidenceStop::Meter {
                incomplete,
                instance: self.instance.clone(),
            })?;
        self.usage = next;
        Ok(())
    }

    /// Admit the clause before any instance is assessed (FR-523 "Tick
    /// arithmetic", "Stop scope"): the distinct formula nodes, the history
    /// span, and the sum `t0 + Hf + m` of every activation anchor `t0` in
    /// checked `u64` arithmetic. `uncertainty` is `Some(u)` under a
    /// `Counter { uncertainty: u }` cover, whose margin is `2u`, and `None`
    /// under `Exact`. No position is visited and the meter is not charged.
    ///
    /// # Errors
    ///
    /// [`ClauseStop::Limit`] naming `formula_nodes` or `history_span`; a
    /// tick sum, a margin or a reach that does not fit in `u64` names
    /// `history_span`. [`ClauseStop::Unreadable`] for a formula whose
    /// operators carry no finite tick bound.
    pub fn admit_clause(
        &mut self,
        formula: &CheckedFormula,
        uncertainty: Option<u64>,
        anchors: impl IntoIterator<Item = u64>,
    ) -> Result<ClauseWork, ClauseStop> {
        self.admit_work(ClauseWork::of(formula), uncertainty, anchors)
    }

    /// [`Self::admit_clause`] for a clause whose formula was measured
    /// already (a [`super::ClauseFormula`] of a checked package).
    ///
    /// # Errors
    ///
    /// As [`Self::admit_clause`].
    pub fn admit_work(
        &mut self,
        work: Result<ClauseWork, ReachFault>,
        uncertainty: Option<u64>,
        anchors: impl IntoIterator<Item = u64>,
    ) -> Result<ClauseWork, ClauseStop> {
        let work = match work {
            Ok(work) => work,
            Err(ReachFault::Overflow) => {
                return Err(ClauseStop::Limit(
                    self.reached(EvidenceCounter::HistorySpan),
                ));
            }
            Err(fault) => return Err(ClauseStop::Unreadable(fault)),
        };
        self.charge(&[
            (EvidenceCounter::FormulaNodes, work.formula_nodes),
            (EvidenceCounter::Visited, work.formula_nodes),
        ])
        .map_err(ClauseStop::Limit)?;
        self.charge(&[(EvidenceCounter::HistorySpan, work.history_span)])
            .map_err(ClauseStop::Limit)?;
        let margin = match uncertainty {
            None => Some(0),
            Some(uncertainty) => uncertainty.checked_mul(2),
        };
        let fits = margin.is_some_and(|margin| {
            anchors.into_iter().all(|anchor| {
                anchor
                    .checked_add(work.future_reach)
                    .and_then(|sum| sum.checked_add(margin))
                    .is_some()
            })
        });
        if fits {
            Ok(work)
        } else {
            Err(ClauseStop::Limit(
                self.reached(EvidenceCounter::HistorySpan),
            ))
        }
    }
}

/// Why a clause was not admitted.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClauseStop {
    /// A counter or the tick arithmetic stopped the call before any
    /// instance was assessed.
    Limit(LimitReached),
    /// The formula has no finite tick reach to measure.
    Unreadable(ReachFault),
}

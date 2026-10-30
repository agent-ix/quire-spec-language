// SPDX-License-Identifier: AGPL-3.0-or-later
//! The QSL FR-251/FR-252 monitor-plan contract handed to later layers.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::{Activation, CheckedBody, CheckedFormula};
use qsl_semantics::library::LibraryName;
use quire_exact::{NodeKey, Rational};

/// The exact checked subject a monitor plan evaluates.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct MonitorSubject {
    package: LibraryName,
    clause_node: NodeKey,
    occurrence: OccurrenceKey,
    clause_name: String,
}

impl MonitorSubject {
    /// Make a subject from the package and clause identity resolved by QSL.
    pub fn new(
        package: LibraryName,
        clause_node: NodeKey,
        occurrence: OccurrenceKey,
        clause_name: impl Into<String>,
    ) -> Self {
        Self {
            package,
            clause_node,
            occurrence,
            clause_name: clause_name.into(),
        }
    }

    /// The checked package's library name.
    pub fn package(&self) -> &LibraryName {
        &self.package
    }
    /// The checked temporal-clause node.
    pub fn clause_node(&self) -> &NodeKey {
        &self.clause_node
    }
    /// The authored occurrence of this declaration.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }
    /// The display name retained for diagnostics.
    pub fn clause_name(&self) -> &str {
        &self.clause_name
    }
}

/// The target's exact counter binding, including the counter unit's scale.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ClockBindingKey {
    counter: String,
    unit: String,
    unit_seconds: Rational,
    width_bits: u32,
}

impl ClockBindingKey {
    /// Bind a hardware counter to a checked time unit and width.
    pub fn new(
        counter: impl Into<String>,
        unit: impl Into<String>,
        unit_seconds: Rational,
        width_bits: u32,
    ) -> Self {
        Self {
            counter: counter.into(),
            unit: unit.into(),
            unit_seconds,
            width_bits,
        }
    }
    /// Counter name.
    pub fn counter(&self) -> &str {
        &self.counter
    }
    /// Time unit name.
    pub fn unit(&self) -> &str {
        &self.unit
    }
    /// Exact duration of one target unit in seconds.
    pub fn unit_seconds(&self) -> &Rational {
        &self.unit_seconds
    }
    /// Counter width in bits.
    pub fn width_bits(&self) -> u32 {
        self.width_bits
    }
}

/// One formula node, scoped to the formula in its plan.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SubformulaId(usize);

impl SubformulaId {
    /// Name a node by its index in one checked formula arena.
    pub fn new(index: usize) -> Self {
        Self(index)
    }
    /// The checked formula arena index.
    pub fn index(self) -> usize {
        self.0
    }
}

/// One checked holds atom, scoped to the formula in its plan.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AtomId(usize);

impl AtomId {
    /// Name an atom by its index in one checked formula arena.
    pub fn new(index: usize) -> Self {
        Self(index)
    }
    /// The checked formula arena index of the atom.
    pub fn index(self) -> usize {
        self.0
    }
}

/// An exact rational interval expressed in counter ticks.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct TickInterval {
    lower: Rational,
    lower_closed: bool,
    upper: Rational,
    upper_closed: bool,
}

impl TickInterval {
    /// An interval in exact tick units.
    pub fn new(lower: Rational, lower_closed: bool, upper: Rational, upper_closed: bool) -> Self {
        Self {
            lower,
            lower_closed,
            upper,
            upper_closed,
        }
    }
    /// Lower endpoint.
    pub fn lower(&self) -> &Rational {
        &self.lower
    }
    /// Whether the lower endpoint is closed.
    pub fn lower_closed(&self) -> bool {
        self.lower_closed
    }
    /// Upper endpoint.
    pub fn upper(&self) -> &Rational {
        &self.upper
    }
    /// Whether the upper endpoint is closed.
    pub fn upper_closed(&self) -> bool {
        self.upper_closed
    }
}

/// A checked atom body and its admitted slot count.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorAtom {
    id: AtomId,
    body: CheckedBody,
    slots: usize,
}

impl MonitorAtom {
    /// Make one checked atom.
    pub fn new(id: AtomId, body: CheckedBody, slots: usize) -> Self {
        Self { id, body, slots }
    }
    /// Formula atom this body evaluates.
    pub fn id(&self) -> AtomId {
        self.id
    }
    /// Name-resolved checked expression body.
    pub fn body(&self) -> &CheckedBody {
        &self.body
    }
    /// Number of admitted inputs read by the body.
    pub fn slots(&self) -> usize {
        self.slots
    }
}

/// The checked formula closure and its atom/activation bodies.
#[derive(Clone, Debug, PartialEq)]
pub struct MonitorProgram {
    root: SubformulaId,
    formula: CheckedFormula,
    activation: Activation,
    atoms: Vec<MonitorAtom>,
    guard: Option<(CheckedBody, usize)>,
}

impl MonitorProgram {
    /// Assemble a program from one checked temporal clause.
    pub fn new(
        root: SubformulaId,
        formula: CheckedFormula,
        activation: Activation,
        atoms: Vec<MonitorAtom>,
        guard: Option<(CheckedBody, usize)>,
    ) -> Self {
        Self {
            root,
            formula,
            activation,
            atoms,
            guard,
        }
    }
    /// Formula root.
    pub fn root(&self) -> SubformulaId {
        self.root
    }
    /// Checked temporal formula topology.
    pub fn formula(&self) -> &CheckedFormula {
        &self.formula
    }
    /// Clause activation.
    pub fn activation(&self) -> Activation {
        self.activation
    }
    /// Checked atom bodies in formula order.
    pub fn atoms(&self) -> &[MonitorAtom] {
        &self.atoms
    }
    /// Optional checked activation guard and its slot count.
    pub fn guard(&self) -> Option<(&CheckedBody, usize)> {
        self.guard.as_ref().map(|(body, slots)| (body, *slots))
    }
}

/// The unique key for one immutable plan in its driver's ownership scope.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PlanId {
    subject: MonitorSubject,
    binding: ClockBindingKey,
    uncertainty: Rational,
    max_reading_gap: u64,
    rate_limit: Rational,
}

impl PlanId {
    /// Construct the structural identity; no new digest is minted.
    pub fn new(
        subject: MonitorSubject,
        binding: ClockBindingKey,
        uncertainty: Rational,
        max_reading_gap: u64,
        rate_limit: Rational,
    ) -> Self {
        Self {
            subject,
            binding,
            uncertainty,
            max_reading_gap,
            rate_limit,
        }
    }
}

/// FR-251: a tick-based monitor plan derived from a checked timed clause.
#[derive(Clone, Debug, PartialEq)]
pub struct TimedMonitorPlan {
    subject: MonitorSubject,
    binding: ClockBindingKey,
    uncertainty: Rational,
    max_reading_gap: u64,
    program: MonitorProgram,
    intervals: Vec<(SubformulaId, TickInterval)>,
    buffers: Vec<(SubformulaId, u64)>,
    rate_limit: Rational,
}

impl TimedMonitorPlan {
    /// Construct the plan emitted by the QSL analyzer.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subject: MonitorSubject,
        binding: ClockBindingKey,
        uncertainty: Rational,
        max_reading_gap: u64,
        program: MonitorProgram,
        intervals: Vec<(SubformulaId, TickInterval)>,
        buffers: Vec<(SubformulaId, u64)>,
        rate_limit: Rational,
    ) -> Self {
        Self {
            subject,
            binding,
            uncertainty,
            max_reading_gap,
            program,
            intervals,
            buffers,
            rate_limit,
        }
    }
    /// The exact checked clause occurrence.
    pub fn subject(&self) -> &MonitorSubject {
        &self.subject
    }
    /// Complete counter binding retained by the plan.
    pub fn binding(&self) -> &ClockBindingKey {
        &self.binding
    }
    /// Clock uncertainty in target units.
    pub fn uncertainty(&self) -> &Rational {
        &self.uncertainty
    }
    /// Required maximum counter reading gap.
    pub fn max_reading_gap(&self) -> u64 {
        self.max_reading_gap
    }
    /// Checked formula and atom closure.
    pub fn program(&self) -> &MonitorProgram {
        &self.program
    }
    /// Timed formula intervals converted to exact counter ticks.
    pub fn intervals(&self) -> &[(SubformulaId, TickInterval)] {
        &self.intervals
    }
    /// Event buffer capacity by buffered formula node.
    pub fn buffers(&self) -> &[(SubformulaId, u64)] {
        &self.buffers
    }
    /// Maximum event rate in events per counter unit.
    pub fn rate_limit(&self) -> &Rational {
        &self.rate_limit
    }
    /// Structural identity used by FR-252 obligations.
    pub fn id(&self) -> PlanId {
        PlanId::new(
            self.subject.clone(),
            self.binding.clone(),
            self.uncertainty.clone(),
            self.max_reading_gap,
            self.rate_limit.clone(),
        )
    }
}

/// FR-252 per-request premises for timing obligations.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimingObligationSettings {
    /// Agreement proof horizon in events.
    pub agreement_horizon: u64,
    /// Checked contract occurrence to timestamp parameter names.
    pub timestamp_parameters: BTreeMap<OccurrenceKey, BTreeSet<String>>,
}

impl Default for TimingObligationSettings {
    fn default() -> Self {
        Self {
            agreement_horizon: 8,
            timestamp_parameters: BTreeMap::new(),
        }
    }
}

/// One FR-252 obligation handed to code generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimingObligation {
    /// The plan agrees with the reference evaluator for traces up to events.
    MonitorAgreement {
        /// The immutable plan being compared.
        plan: PlanId,
        /// The finite event horizon being proved.
        events: u64,
    },
    /// Tick and event-buffer arithmetic properties for the plan.
    TickArithmetic {
        /// The immutable plan being checked.
        plan: PlanId,
        /// Counter width in bits.
        width_bits: u32,
        /// Buffer capacities by formula node.
        buffers: Vec<(SubformulaId, u64)>,
        /// Maximum event rate in events per counter unit.
        rate: Rational,
    },
    /// A checked contract relates declared timestamp parameters.
    TimestampContract {
        /// The checked contract occurrence.
        contract: OccurrenceKey,
    },
}

/// Why FR-251 could not produce a complete plan.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum MonitorPlanRefusal {
    /// The gap can be confused with a full counter wrap.
    #[error("maximum reading gap {max_reading_gap} is not below 2^{width_bits}")]
    GapExceedsWidth {
        /// The maximum gap in ticks.
        max_reading_gap: u64,
        /// The counter width in bits.
        width_bits: u32,
    },
    /// A hardware counter must have positive width.
    #[error("counter width must be positive")]
    ZeroCounterWidth,
    /// The event-buffer rate or window is invalid.
    #[error("event-buffer rate must be positive and window non-negative")]
    InvalidBufferInputs,
    /// The exact capacity does not fit in u64.
    #[error("event-buffer capacity exceeds u64")]
    BufferCapacityOverflow,
    /// The selected clause has no timed binding.
    #[error("selected clause has no checked model-time binding")]
    NotTimedClause,
    /// The selected package has no emitted package identity.
    #[error("checked package identity could not be emitted")]
    PackageIdentityUnavailable,
    /// The target unit is not a checked exact time unit.
    #[error("target unit is not a checked time unit: {unit}")]
    InvalidTimeUnit {
        /// The unresolved unit name.
        unit: String,
    },
    /// A checked temporal atom has no checked body.
    #[error("checked temporal atom {index} has no condition body")]
    MissingAtomBody {
        /// The formula arena index.
        index: usize,
    },
    /// The interval could not be converted exactly.
    #[error("timed interval conversion failed")]
    IntervalConversion,
}

/// FR-251's target input, before unit scales are resolved from the package.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorTarget {
    /// Counter name.
    pub counter: String,
    /// Checked time-unit name.
    pub unit: String,
    /// Hardware counter width.
    pub width_bits: u32,
    /// Clock uncertainty in the counter's unit.
    pub uncertainty: Rational,
    /// Maximum event rate, or None for FR-251's default.
    pub max_event_rate: Option<Rational>,
    /// Required maximum tick gap between readings.
    pub max_reading_gap: u64,
}

/// Generate FR-252's two per-plan obligations.
pub fn plan_obligations(
    plan: &TimedMonitorPlan,
    settings: &TimingObligationSettings,
) -> [TimingObligation; 2] {
    let id = plan.id();
    [
        TimingObligation::MonitorAgreement {
            plan: id.clone(),
            events: settings.agreement_horizon,
        },
        TimingObligation::TickArithmetic {
            plan: id,
            width_bits: plan.binding.width_bits,
            buffers: plan.buffers.clone(),
            rate: plan.rate_limit.clone(),
        },
    ]
}

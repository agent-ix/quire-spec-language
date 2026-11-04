// SPDX-License-Identifier: AGPL-3.0-or-later
//! The QSL FR-251/FR-252 monitor-plan contract handed to later layers.

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_semantics::check::{
    Activation, BinaryTemporal, CheckedBody, CheckedFormula, FormulaNode, FormulaNodeId,
    UnaryTemporal,
};
use qsl_semantics::library::LibraryName;
use quire_exact::{Integer as ExactInteger, NodeKey, Rational};

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
    helpers: BTreeMap<usize, CheckedBody>,
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
            helpers: BTreeMap::new(),
        }
    }
    /// This program with `helpers`, the checked body of every declared
    /// function an atom or the guard calls, directly or through another
    /// helper (the call closure of QSpec FR-487-AC-13), keyed by the
    /// function index a checked call names.
    #[must_use]
    pub fn with_helpers(mut self, helpers: BTreeMap<usize, CheckedBody>) -> Self {
        self.helpers = helpers;
        self
    }
    /// The checked helper bodies of the call closure, by function index.
    pub fn helpers(&self) -> &BTreeMap<usize, CheckedBody> {
        &self.helpers
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

/// One checked formula node reduced to the facts that select its meaning:
/// the operator, its ordered operands and the atom it reads. Intervals are
/// in [`PlanId`] by their tick bounds.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum FormulaToken {
    /// `true` or `false`.
    Constant(bool),
    /// A `holds` atom.
    Holds(AtomId),
    /// An aggregate atom.
    Aggregate,
    /// `not`.
    Not(SubformulaId),
    /// `and`.
    And(SubformulaId, SubformulaId),
    /// `or`.
    Or(SubformulaId, SubformulaId),
    /// `implies`.
    Implies(SubformulaId, SubformulaId),
    /// A unary temporal operator, by name.
    Unary(&'static str, SubformulaId),
    /// A binary temporal operator, by name.
    Binary(&'static str, SubformulaId, SubformulaId),
}

impl FormulaToken {
    fn of(node: &FormulaNode, index: usize) -> Self {
        let id = |node: FormulaNodeId| SubformulaId::new(node.index());
        match node {
            FormulaNode::Constant(value) => Self::Constant(*value),
            FormulaNode::Holds { .. } => Self::Holds(AtomId::new(index)),
            FormulaNode::Aggregate { .. } => Self::Aggregate,
            FormulaNode::Not(operand) => Self::Not(id(*operand)),
            FormulaNode::And(a, b) => Self::And(id(*a), id(*b)),
            FormulaNode::Or(a, b) => Self::Or(id(*a), id(*b)),
            FormulaNode::Implies(a, b) => Self::Implies(id(*a), id(*b)),
            FormulaNode::Unary {
                operator, operand, ..
            } => Self::Unary(
                match operator {
                    UnaryTemporal::Eventually => "eventually",
                    UnaryTemporal::Always => "always",
                    UnaryTemporal::Once => "once",
                    UnaryTemporal::Historically => "historically",
                },
                id(*operand),
            ),
            FormulaNode::Binary {
                operator,
                left,
                right,
                ..
            } => Self::Binary(
                match operator {
                    BinaryTemporal::Until => "until",
                    BinaryTemporal::Release => "release",
                    BinaryTemporal::Since => "since",
                    BinaryTemporal::Triggered => "triggered",
                },
                id(*left),
                id(*right),
            ),
        }
    }
}

/// The unique key for one immutable plan in its driver's ownership scope.
///
/// It covers everything the plan fixes (FR-251 "Build inputs and identity"):
/// the subject, the formula topology and root, the activation, the checked
/// body and admitted input slots of every atom and of the activation guard,
/// the tick intervals, the buffer capacities and every target premise.
/// Changing any one yields a different `PlanId`, including through
/// [`TimedMonitorPlan::new`] with the subject unchanged (FR-251-AC-37,
/// AC-39), and the checked body of every helper function in the call
/// closure of the atoms and the guard (FR-251-AC-38): the clause key names a
/// callee by its declaration, not its body. No new digest is minted.
#[derive(Clone, Debug, PartialEq)]
pub struct PlanId {
    subject: MonitorSubject,
    binding: ClockBindingKey,
    uncertainty: Rational,
    max_reading_gap: u64,
    rate_limit: Rational,
    root: SubformulaId,
    topology: Vec<FormulaToken>,
    activation: ActivationToken,
    atoms: Vec<MonitorAtom>,
    guard: Option<(CheckedBody, usize)>,
    helpers: BTreeMap<usize, CheckedBody>,
    intervals: Vec<(SubformulaId, TickInterval)>,
    buffers: Vec<(SubformulaId, u64)>,
    activation_capacity: u64,
}

/// `CheckedBody` equality is total: a literal compares by key, a float by
/// its IEEE bits, so every `PlanId` equals itself.
impl Eq for PlanId {}

/// The clause activation as part of a plan identity.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ActivationToken {
    /// `on origin`.
    Origin,
    /// `on each`, with or without a guard.
    Each {
        /// Whether a `when` guard selects activating events.
        guarded: bool,
    },
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
    activation_capacity: u64,
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
        activation_capacity: u64,
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
            activation_capacity,
        }
    }
    /// The plan's `activation_capacity` premise.
    pub fn activation_capacity(&self) -> u64 {
        self.activation_capacity
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
    /// The identity of the complete immutable plan, used by FR-252
    /// obligations and by driver admission.
    pub fn id(&self) -> PlanId {
        let program = &self.program;
        PlanId {
            subject: self.subject.clone(),
            binding: self.binding.clone(),
            uncertainty: self.uncertainty.clone(),
            max_reading_gap: self.max_reading_gap,
            rate_limit: self.rate_limit.clone(),
            root: program.root(),
            topology: program
                .formula()
                .nodes()
                .iter()
                .enumerate()
                .map(|(index, node)| FormulaToken::of(node, index))
                .collect(),
            activation: match program.activation() {
                Activation::Origin => ActivationToken::Origin,
                Activation::Each { guarded } => ActivationToken::Each { guarded },
            },
            atoms: program.atoms().to_vec(),
            guard: program.guard.clone(),
            helpers: program.helpers.clone(),
            intervals: self.intervals.clone(),
            buffers: self.buffers.clone(),
            activation_capacity: self.activation_capacity,
        }
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
    /// A checked call names a function the package does not hold.
    #[error("called function {index} has no checked body")]
    MissingHelperBody {
        /// The function index the call names.
        index: usize,
    },
    /// The interval could not be converted exactly.
    #[error("timed interval conversion failed")]
    IntervalConversion,
    /// A premise the plan-root node carries would be refused by its reader.
    #[error("plan-root premise refused: {0}")]
    Premise(PremiseDefect),
    /// An unbounded operator that looks backwards (`once`, `historically`,
    /// `since`, `triggered` with no interval) has no program (FR-251 "Fault
    /// boundary").
    #[error("unbounded past operator {node} yields no complete program")]
    UnboundedPast {
        /// The formula arena index.
        node: usize,
    },
    /// An unbounded operator whose operands hold another unbounded operator
    /// has no program (FR-251 "Fault boundary").
    #[error("unbounded operator {node} over an unbounded operator yields no complete program")]
    UnboundedOverUnbounded {
        /// The formula arena index.
        node: usize,
    },
    /// A tick threshold of the operator does not fit the program's integers.
    #[error("the ticks of operator {node} do not fit the program's integers")]
    TickRange {
        /// The formula arena index.
        node: usize,
    },
    /// An interval operator has no row in the buffer table.
    #[error("interval operator {node} has no buffer capacity")]
    MissingBuffer {
        /// The formula arena index.
        node: usize,
    },
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
    /// The bound on live obligation instances of unbounded operators
    /// (`monitor.max_activations`, QSpec FR-487 `activation_capacity`).
    pub max_activations: u64,
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

/// The binding names of the `temporal`/`monitor_plan` node, in body order
/// (QSpec FR-487 "body").
pub const PLAN_ROOT_BINDINGS: [&str; 17] = [
    "clause",
    "state",
    "inputs",
    "scope",
    "results",
    "init",
    "on_event",
    "on_watermark",
    "counter_width_bits",
    "max_reading_gap",
    "max_event_rate",
    "buffers",
    "counter",
    "tick_unit",
    "uncertainty",
    "on_gap",
    "activation_capacity",
];

/// `value` in the canonical rational spelling of QSpec FR-322-AC-114:
/// reduced, the denominator omitted when it is 1.
pub fn canonical_rational(value: &Rational) -> String {
    if value.is_integer() {
        value.numerator().to_string()
    } else {
        format!("{}/{}", value.numerator(), value.denominator())
    }
}

/// The identity-bearing premises of a plan, as the plan-root node carries
/// them (QSpec FR-487 "body": `counter_width_bits` to `uncertainty`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanRootPremises {
    /// `counter`: the counter's name.
    pub counter: String,
    /// `counter_width_bits`.
    pub counter_width_bits: u32,
    /// `max_reading_gap`.
    pub max_reading_gap: u64,
    /// `max_event_rate`, events per tick.
    pub max_event_rate: Rational,
    /// `uncertainty`, in ticks.
    pub uncertainty: Rational,
    /// `buffers`: capacity per buffered subformula, ascending by subformula.
    pub buffers: Vec<(SubformulaId, u64)>,
    /// `activation_capacity` (member 16), at least 0.
    pub activation_capacity: u64,
}

/// The first premise a reader would refuse (QSpec FR-487 reader order,
/// steps 4 and 5), in body order.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PremiseDefect {
    /// `counter_width_bits` outside 1 to 64.
    #[error("counter width outside 1 to 64")]
    CounterWidth,
    /// `max_reading_gap` not below `2^counter_width_bits`.
    #[error("maximum reading gap not below 2^width")]
    MaxReadingGap,
    /// `max_event_rate` not above 0.
    #[error("maximum event rate not above 0")]
    MaxEventRate,
    /// A `capacity` below 1.
    #[error("buffer capacity below 1")]
    Capacity,
    /// An empty `counter`.
    #[error("empty counter name")]
    Counter,
    /// A negative `uncertainty`.
    #[error("negative uncertainty")]
    Uncertainty,
    /// `buffers` not strictly ascending by subformula.
    #[error("buffers not strictly ascending")]
    BufferOrder,
}

impl PlanRootPremises {
    /// Check the premises in the reader's order.
    pub fn validate(&self) -> Result<(), PremiseDefect> {
        let zero = Rational::from_integer(ExactInteger::from(0_i64));
        if !(1..=64).contains(&self.counter_width_bits) {
            return Err(PremiseDefect::CounterWidth);
        }
        if u128::from(self.max_reading_gap) >= (1_u128 << self.counter_width_bits) {
            return Err(PremiseDefect::MaxReadingGap);
        }
        if self.max_event_rate <= zero {
            return Err(PremiseDefect::MaxEventRate);
        }
        if self.buffers.iter().any(|(_, capacity)| *capacity < 1) {
            return Err(PremiseDefect::Capacity);
        }
        if self.counter.is_empty() {
            return Err(PremiseDefect::Counter);
        }
        if self.uncertainty < zero {
            return Err(PremiseDefect::Uncertainty);
        }
        if self.buffers.windows(2).any(|pair| pair[0].0 >= pair[1].0) {
            return Err(PremiseDefect::BufferOrder);
        }
        Ok(())
    }
}

impl TimedMonitorPlan {
    /// The premises the plan-root node carries.
    pub fn root_premises(&self) -> PlanRootPremises {
        PlanRootPremises {
            counter: self.binding.counter().to_owned(),
            counter_width_bits: self.binding.width_bits(),
            max_reading_gap: self.max_reading_gap,
            max_event_rate: self.rate_limit.clone(),
            uncertainty: self.uncertainty.clone(),
            buffers: self.buffers.clone(),
            activation_capacity: self.activation_capacity,
        }
    }
}

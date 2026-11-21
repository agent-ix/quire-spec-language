// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-201 exact-value certificate types and the independent Bellman checker
//! core. The bounded graph adapter below enumerates a checker-owned model
//! graph through `fsl-eval::ModelSystem`; it does not consume analyzer output.

use filament_exact::{Cancel, CancelCause, Integer, Rational};
use fsl_semantics::check::Bound;
use std::collections::VecDeque;
use std::collections::{BTreeMap, BTreeSet};

use filament_exact::ObjectReference;
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::model::{ModelSimState, ModelSystem};
use fsl_eval::simulation::TransitionSystem;
use fsl_semantics::check::{
    CheckedProbabilisticClaim, ProbForm, ProbScheduler, ReachTarget, StatePredicate, Workload,
};
use serde_json::{json, Value};

use fsl_package::ObligationIdentity;

mod bounded;
mod key_admission;
pub use key_admission::ProbabilityKeyAdmission;
mod cyclic;
pub use cyclic::admit_acyclic_probabilistic_refutation_subject;
#[cfg(feature = "test-support")]
pub use cyclic::admit_acyclic_probabilistic_refutation_subject_with_identity;
mod document;
mod reward;
#[cfg(feature = "test-support")]
pub use reward::RewardProbabilityPhase;
mod state_predicate;
pub use bounded::probabilistic_formula_monitor_context;
mod source;
pub use document::*;
pub use source::*;
mod witness;
pub use witness::*;

/// A complete canonical product-state key, encoded by the checker.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProbabilityProductKey(Vec<u8>);

impl ProbabilityProductKey {
    /// Construct a key from its canonical encoding.
    pub fn from_canonical_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The canonical product-state encoding.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The key of an untimed model state whose claim control is absent: an
    /// unbounded reachability form, which has no monitor, accumulator or
    /// pending draw (FR-196). `binding` is the claim's bound `over` reference,
    /// encoded as FR-181's reference triple, and
    /// `model_bytes` the bare canonical FR-181 form returned by
    /// `ModelSystem::state_key`. The parsed form is wrapped once with the
    /// `filament.simulation.state-key/v1` domain. Construction uses finite
    /// 1 GiB input/output and cumulative working-storage defaults. This is
    /// the key the checker enumerates for the same state,
    /// so a certificate built with it names the checker's states.
    ///
    /// # Errors
    ///
    /// [`ExactProbabilityGraphError::ExpansionRefused`] if the key cannot be
    /// encoded.
    pub fn for_model_state(
        binding: Option<&ObjectReference>,
        model_bytes: &[u8],
    ) -> Result<Self, ExactProbabilityGraphError> {
        product_key(binding, model_bytes)
    }

    /// Construct the unbounded FR-196 key with caller byte/storage admission.
    /// The input is the bare canonical form returned by `ModelSystem::state_key`.
    ///
    /// # Errors
    ///
    /// Malformed/noncanonical input, the actual exhausted certificate ceiling,
    /// or the supplied cancellation cause; no partial key is returned.
    pub fn for_model_state_with_limits(
        binding: Option<&ObjectReference>,
        model_bytes: &[u8],
        limits: ProbabilityDocumentIoLimits,
        cancel: &Cancel,
    ) -> Result<Self, ProbabilityDocumentError> {
        document::model_product_key(binding, model_bytes, None, limits, cancel)
    }

    /// Construct a complete untimed key under the identity owner's byte policy.
    /// The same caller budget admits canonical parsing, staging and output.
    ///
    /// # Errors
    /// Caller denial, cancellation, identity byte or allocator refusal remain
    /// distinct in the model owner's error vocabulary.
    pub fn for_model_state_with_budget(
        binding: Option<&ObjectReference>,
        model_bytes: &[u8],
        identity: fsl_foundation::IdentityLimits,
        budget: &mut dyn fsl_eval::simulation::model::ModelKeyBudget,
        cancel: &Cancel,
    ) -> Result<Self, fsl_eval::simulation::model::ModelKeyError> {
        document::model_product_key_with_budget(binding, model_bytes, identity, budget, cancel)
    }

    /// Construct a formula product using the parsed complete FR-338 key.
    /// Formula context and state membership still require independent checking.
    ///
    /// # Errors
    ///
    /// The same admission/cancellation refusals as `for_model_state_with_limits`,
    /// or a malformed automaton envelope.
    pub fn for_formula_state(
        binding: Option<&ObjectReference>,
        model_bytes: &[u8],
        monitor: &fsl_eval::automaton::AutomatonStateKey,
        limits: ProbabilityDocumentIoLimits,
        cancel: &Cancel,
    ) -> Result<Self, ProbabilityDocumentError> {
        document::model_product_key(binding, model_bytes, Some(monitor), limits, cancel)
    }

    /// Construct the complete formula key using the raw identity policy and
    /// the caller's cumulative work and storage admission.
    ///
    /// # Errors
    /// Returns the owner's identity/encoding refusal or caller denial. A denied
    /// budget retains the precise caller work/storage/cancellation cause.
    pub fn for_formula_state_with_budget(
        binding: Option<&ObjectReference>,
        model_bytes: &[u8],
        monitor: &fsl_eval::automaton::AutomatonStateKey,
        identity: fsl_foundation::IdentityLimits,
        budget: &mut dyn fsl_eval::simulation::model::ModelKeyBudget,
        cancel: &Cancel,
    ) -> Result<Self, fsl_eval::simulation::model::ModelKeyError> {
        document::formula_product_key_with_budget(
            binding,
            model_bytes,
            Some(monitor),
            identity,
            budget,
            cancel,
        )
    }
}

/// The timed-tail control included in an FR-204 model key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DigitalTail {
    /// A state that still admits ordinary discrete operations.
    Running,
    /// A selected idle tail before its first positive delay.
    IdleReady,
    /// A selected idle tail after a positive delay.
    IdleDelayed,
}

impl DigitalTail {
    fn tag(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::IdleReady => "idle-ready",
            Self::IdleDelayed => "idle-delayed",
        }
    }
}

/// One admitted digital clock and its checker-derived cap.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigitalClockKey {
    /// The checked declaring type, field and receiver tuple.
    pub identity: Value,
    /// The clock value in digital units.
    pub value: u64,
    /// The checker-derived saturation cap in digital units.
    pub cap: u64,
}

/// A timed deadline clock and its scaled closed bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DigitalDeadlineKey {
    /// The scaled closed deadline bound.
    pub limit: u64,
    /// The current non-resetting deadline clock value.
    pub elapsed: u64,
}

/// One timed clock in a region or boundary-region product key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionClockKey {
    /// The checked declaring type, field and receiver tuple.
    pub identity: Value,
    /// The clock value in refined integer units.
    pub value: u64,
    /// The checker-derived saturation cap in refined integer units.
    pub cap: u64,
}

/// The deadline clock in a region or boundary-region product key.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RegionDeadlineKey {
    /// The deadline bound in refined integer units.
    pub limit: u64,
    /// The elapsed deadline value in refined integer units.
    pub elapsed: u64,
}

/// Whether a region key names an interior valuation or its boundary corner.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionKeyKind {
    /// The state is a canonical clock region.
    Region,
    /// The state is a corner of the region closure for elapsed-time evidence.
    BoundaryRegion,
}

/// The checked semantic components used to encode one region ProductKey.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegionProductKeyInput {
    /// The checked claim's `over` binding, or `None` when there is no binding.
    pub binding: Option<Value>,
    /// The complete non-clock simulation-state form.
    pub discrete: Value,
    /// FR-196's closed time-unit form.
    pub unit: Value,
    /// The unrefined canonical time scale.
    pub scale: u64,
    /// The region representative's subdivision count.
    pub subdivisions: u64,
    /// Every admitted clock with its checked identity and refined value/cap.
    pub clocks: Vec<RegionClockKey>,
    /// The timed deadline clock, when the checked claim has a deadline.
    pub deadline: Option<RegionDeadlineKey>,
    /// TS-5 tail control for the state.
    pub tail: DigitalTail,
    /// The checked claim's monitor state, or `None` when it has no monitor.
    pub monitor: Option<Value>,
    /// The checked claim's exact measure accumulator, if present.
    pub accumulator: Option<Value>,
    /// The actual pending draw control, if present.
    pub pending: Option<Value>,
    /// Region encoding kind.
    pub kind: RegionKeyKind,
    /// The corner coordinates in clock-entry order, including the deadline.
    pub corner: Option<Vec<String>>,
}

/// The checked semantic components used to encode one digital ProductKey.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DigitalProductKeyInput {
    /// The checked claim's `over` binding, or `None` when there is no binding.
    pub binding: Option<Value>,
    /// The complete non-clock simulation-state form.
    pub discrete: Value,
    /// FR-196's closed time-unit form.
    pub unit: Value,
    /// The positive canonical digital scale.
    pub scale: u64,
    /// Every admitted clock with its checked identity and cap.
    pub clocks: Vec<DigitalClockKey>,
    /// The timed deadline clock, when the checked claim has a deadline.
    pub deadline: Option<DigitalDeadlineKey>,
    /// TS-5 tail control for the state.
    pub tail: DigitalTail,
    /// The checked claim's monitor state, or `None` when it has no monitor.
    pub monitor: Option<Value>,
    /// The checked claim's exact measure accumulator, if present.
    pub accumulator: Option<Value>,
    /// The actual pending draw control, if present.
    pub pending: Option<Value>,
}

/// Encode the FR-196 canonical ProductKey for one admitted digital state.
///
/// `discrete` is the complete non-clock simulation-state form and `unit` is
/// FR-196's closed time-unit form. Clock identities are canonical three-part
/// tuples; the encoder orders them by their complete JCS bytes. The numeric
/// fields use decimal strings in the key so values are not rounded through
/// JSON numbers.
///
/// The JSON forms supplied for `binding`, `discrete`, `unit`, `monitor`,
/// `accumulator` and `pending` must already have been derived from the checked
/// subject and claim. This routine owns only their canonical representation;
/// contextual admission and derivation remain the caller's responsibility.
pub fn digital_probability_product_key(
    input: DigitalProductKeyInput,
) -> Result<ProbabilityProductKey, ExactProbabilityGraphError> {
    let DigitalProductKeyInput {
        binding,
        discrete,
        unit,
        scale,
        clocks,
        deadline,
        tail,
        monitor,
        accumulator,
        pending,
    } = input;
    if scale == 0 {
        return Err(ExactProbabilityGraphError::InvalidDigitalKey);
    }

    let mut keyed_clocks = clocks
        .into_iter()
        .map(|clock| {
            if clock
                .identity
                .as_array()
                .is_none_or(|parts| parts.len() != 3)
                || clock.value > clock.cap
            {
                return Err(ExactProbabilityGraphError::InvalidDigitalKey);
            }
            let identity_bytes = canonical_json(&clock.identity)?;
            Ok((identity_bytes, clock.identity, clock.value, clock.cap))
        })
        .collect::<Result<Vec<_>, _>>()?;
    keyed_clocks.sort_by(|left, right| left.0.cmp(&right.0));
    if keyed_clocks.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(ExactProbabilityGraphError::InvalidDigitalKey);
    }
    let clocks = keyed_clocks
        .into_iter()
        .map(|(_, identity, value, cap)| json!([identity, value.to_string(), cap.to_string()]))
        .collect::<Vec<_>>();

    let deadline = match deadline {
        Some(DigitalDeadlineKey { limit, elapsed })
            if limit.checked_add(1).is_some_and(|cap| elapsed <= cap) =>
        {
            json!(["deadline", limit.to_string(), elapsed.to_string()])
        }
        Some(_) => return Err(ExactProbabilityGraphError::InvalidDigitalKey),
        None => Value::Null,
    };
    let model = json!([
        "filament.simulation.state-key/v1",
        [
            "digital",
            discrete,
            unit,
            scale.to_string(),
            clocks,
            deadline,
            tail.tag()
        ]
    ]);
    let product = json!([
        "filament.probability-product-key/v1",
        binding,
        null,
        null,
        model,
        monitor,
        accumulator,
        pending
    ]);
    canonical_json(&product).map(ProbabilityProductKey)
}

/// Encode the FR-920 canonical ProductKey for a region or boundary-region state.
pub fn region_probability_product_key(
    input: RegionProductKeyInput,
) -> Result<ProbabilityProductKey, ExactProbabilityGraphError> {
    let RegionProductKeyInput {
        binding,
        discrete,
        unit,
        scale,
        subdivisions,
        clocks,
        deadline,
        tail,
        monitor,
        accumulator,
        pending,
        kind,
        corner,
    } = input;
    if scale == 0 || subdivisions < 2 {
        return Err(ExactProbabilityGraphError::InvalidDigitalKey);
    }
    let mut keyed_clocks = clocks
        .into_iter()
        .map(|clock| {
            if clock
                .identity
                .as_array()
                .is_none_or(|parts| parts.len() != 3)
                || clock.value > clock.cap
            {
                return Err(ExactProbabilityGraphError::InvalidDigitalKey);
            }
            let identity_bytes = canonical_json(&clock.identity)?;
            Ok((identity_bytes, clock.identity, clock.value, clock.cap))
        })
        .collect::<Result<Vec<_>, _>>()?;
    keyed_clocks.sort_by(|left, right| left.0.cmp(&right.0));
    if keyed_clocks.windows(2).any(|pair| pair[0].0 == pair[1].0) {
        return Err(ExactProbabilityGraphError::InvalidDigitalKey);
    }
    let clock_count = keyed_clocks.len();
    let deadline_values = match deadline {
        Some(deadline)
            if deadline
                .limit
                .checked_add(subdivisions)
                .is_some_and(|cap| deadline.elapsed <= cap) =>
        {
            Some((deadline.limit, deadline.elapsed))
        }
        Some(_) => return Err(ExactProbabilityGraphError::InvalidDigitalKey),
        None => None,
    };
    let mut fractions = keyed_clocks
        .iter()
        .filter_map(|(_, _, value, cap)| {
            (*value < *cap)
                .then_some(*value % subdivisions)
                .filter(|fraction| *fraction > 0)
        })
        .collect::<BTreeSet<_>>();
    if let Some((limit, elapsed)) = deadline_values {
        if elapsed < limit.saturating_add(subdivisions) {
            let fraction = elapsed % subdivisions;
            if fraction > 0 {
                fractions.insert(fraction);
            }
        }
    }
    let fraction_rank = |value: u64, cap: u64| -> String {
        if value >= cap {
            return "0".to_owned();
        }
        let fraction = value % subdivisions;
        if fraction == 0 {
            return "0".to_owned();
        }
        fractions
            .iter()
            .position(|candidate| *candidate == fraction)
            .and_then(|position| u64::try_from(position + 1).ok())
            .map(|rank| rank.to_string())
            .unwrap_or_else(|| "0".to_owned())
    };
    let clocks = keyed_clocks
        .into_iter()
        .map(|(_, identity, value, cap)| {
            json!([
                identity,
                if value >= cap {
                    value.div_ceil(subdivisions)
                } else {
                    value / subdivisions
                }
                .to_string(),
                cap.div_ceil(subdivisions).to_string(),
                fraction_rank(value, cap)
            ])
        })
        .collect::<Vec<_>>();
    let deadline = match deadline_values {
        Some((limit, elapsed)) => json!([
            "deadline",
            (limit / subdivisions).to_string(),
            (elapsed / subdivisions).to_string(),
            fraction_rank(elapsed, limit.saturating_add(subdivisions))
        ]),
        None => Value::Null,
    };
    let tag = match kind {
        RegionKeyKind::Region => "region",
        RegionKeyKind::BoundaryRegion => "boundary-region",
    };
    let mut model = vec![
        Value::String("filament.simulation.state-key/v1".to_owned()),
        json!([
            tag,
            discrete,
            unit,
            scale.to_string(),
            clocks,
            deadline,
            tail.tag()
        ]),
    ];
    if kind == RegionKeyKind::BoundaryRegion {
        let corner = corner.ok_or(ExactProbabilityGraphError::InvalidDigitalKey)?;
        if corner.len() != clock_count + usize::from(deadline_values.is_some()) {
            return Err(ExactProbabilityGraphError::InvalidDigitalKey);
        }
        model[1]
            .as_array_mut()
            .ok_or(ExactProbabilityGraphError::InvalidDigitalKey)?
            .push(json!(corner));
    } else if corner.is_some() {
        return Err(ExactProbabilityGraphError::InvalidDigitalKey);
    }
    let product = json!([
        "filament.probability-product-key/v1",
        binding,
        null,
        null,
        model,
        monitor,
        accumulator,
        pending
    ]);
    canonical_json(&product).map(ProbabilityProductKey)
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, ExactProbabilityGraphError> {
    quire_canonical::to_vec(value, quire_canonical::Limits::new(u64::MAX))
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)
}

/// The exact rational value certified for one probabilistic product state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityValue {
    /// The complete canonical product-state key.
    pub state: ProbabilityProductKey,
    /// The exact value at that state.
    pub value: Rational,
}

/// An exact-value certificate's state values (FR-201).
///
/// This certificate form does not itself establish graph completeness. The
/// replay facade must re-enumerate the subject and pass that checker-owned
/// graph to the independent checker before accepting the certificate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityCertificate {
    /// One value for every state in the checker-owned graph, in key order.
    pub values: Vec<ExactProbabilityValue>,
}

/// The scheduler objective for an exact event or expected-reward certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityObjective {
    /// The minimum over all enabled actions.
    Minimum,
    /// The maximum over all enabled actions.
    Maximum,
    /// A workload with exactly one enabled probability distribution.
    Workload,
}

/// Whether the certified initial value satisfies the checked claim's bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityBoundSide {
    /// The value satisfies an authored `>=` bound.
    AtLeast,
    /// The value satisfies an authored `<=` bound.
    AtMost,
}

/// An exact certificate rejection and the first canonical state at fault.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityCertificateRejection {
    /// The failed FR-201 condition.
    pub rule: ProbabilityCertificateRule,
    /// The state or member at which it failed.
    pub at: ProbabilityCertificateLocus,
}

/// The exact-certificate rejection rules implemented by this checker core.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityCertificateRule {
    /// A certificate value does not equal the recomputed Bellman value.
    InequalityFails,
    /// A structurally valid certificate's initial value misses the claim bound.
    ThresholdMissed,
    /// A certificate omits, duplicates, or adds a state value.
    Malformed,
    /// A positive non-target state has no required progress rank.
    RankMissing,
    /// A rank fails its profile's progress or exact-height/debt condition.
    RankBroken,
}

/// The checked component of a conditional probability certificate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProbabilityCertificatePart {
    /// The satisfied-and-activated numerator.
    Numerator,
    /// The activation denominator.
    Denominator,
}
impl ProbabilityCertificatePart {
    /// Canonical published part spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Numerator => "numerator",
            Self::Denominator => "denominator",
        }
    }
}

/// The locus of an exact-certificate rejection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProbabilityCertificateLocus {
    /// A full probabilistic product state key.
    ProductState(ProbabilityProductKey),
    /// A full state key and its failing conditional-certificate component.
    ProductPart {
        /// Complete probabilistic product key.
        key: ProbabilityProductKey,
        /// The independently checked part.
        part: ProbabilityCertificatePart,
    },
    /// A malformed certificate member.
    Member(&'static str),
}

/// Evidence produced only by the independent exact certificate checker.
/// Its checked value cannot be constructed or changed by downstream callers.
///
/// ```compile_fail,E0451
/// use fsl_replay::{ExactProbabilityAcceptance, ProbabilityBoundSide};
/// use filament_exact::{Integer, Rational};
/// let forged = ExactProbabilityAcceptance {
///     value: Rational::from_integer(Integer::from(1_i64)),
///     side: ProbabilityBoundSide::AtLeast,
/// };
/// ```
///
/// ```compile_fail,E0594
/// use fsl_replay::ExactProbabilityAcceptance;
/// use filament_exact::{Integer, Rational};
/// fn alter(accepted: &mut ExactProbabilityAcceptance) {
///     *accepted.value() = Rational::from_integer(Integer::from(1_i64));
/// }
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityAcceptance {
    // Total admitted initial coverage. None retains the older single-root profiles;
    // a None value in this list is independently proved nonactivation.
    initial_values: Option<CheckedInitialValues>,
    value: Rational,
    side: ProbabilityBoundSide,
    certificate: ExactProbabilityCertificate,
    document: Option<Box<[u8]>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CheckedInitialValues {
    values: Vec<(u32, Option<Rational>)>,
    binding: Option<crate::ExactBinding>,
}

impl ExactProbabilityAcceptance {
    /// Canonical document bytes when this token was produced by the document checker.
    pub fn document(&self) -> Option<&[u8]> {
        self.document.as_deref()
    }
    /// The exact certificate this checker accepted, retained with its token.
    pub fn certificate(&self) -> &ExactProbabilityCertificate {
        &self.certificate
    }
    /// The independently checked initial value, or the least favourable
    /// activating initial value for a conditional measure certificate.
    pub fn value(&self) -> &Rational {
        &self.value
    }

    pub(crate) fn initial_values(&self) -> Option<&[(u32, Option<Rational>)]> {
        self.initial_values
            .as_ref()
            .map(|coverage| coverage.values.as_slice())
    }

    pub(crate) fn initial_entries_match(&self, entries: &[crate::ExactEntry]) -> bool {
        let Some(coverage) = &self.initial_values else {
            return true;
        };
        let mut offered = entries.iter();
        for (initial, value) in &coverage.values {
            let Some(value) = value else {
                continue;
            };
            let Some(entry) = offered.next() else {
                return false;
            };
            if entry.initial != *initial
                || entry.binding != coverage.binding
                || entry.extremum.is_some()
                || !matches!(&entry.value, crate::ExactOrBounds::Exact { value: offered } if offered.value() == value)
            {
                return false;
            }
        }
        offered.next().is_none()
    }

    /// The authored bound side checked against that value.
    pub fn side(&self) -> ProbabilityBoundSide {
        self.side
    }
}

/// A value-free vacuous probability proof accepted by the independent checker.
/// Its private document can only be obtained by proving activation unreachable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProbabilityVacuityAcceptance {
    document: Box<[u8]>,
    side: ProbabilityBoundSide,
}
impl ProbabilityVacuityAcceptance {
    /// The accepted Conditional::Vacuous document's canonical bytes.
    pub fn document(&self) -> &[u8] {
        &self.document
    }
    /// The authored claim direction, without an invented conditional value.
    pub fn side(&self) -> ProbabilityBoundSide {
        self.side
    }
}

/// The result of checking an exact-value certificate against a graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactProbabilityCertificateCheck {
    /// Every value matches the independently evaluated graph and the initial
    /// value satisfies the claim bound.
    Accepted(ExactProbabilityAcceptance),
    /// Activation is unreachable; this certified proof has no numeric value.
    AcceptedVacuous(ProbabilityVacuityAcceptance),
    /// The certificate fails a checked condition.
    Rejected(ProbabilityCertificateRejection),
}

/// Why the exact Bellman-check core could not check a graph.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExactProbabilityGraphError {
    /// The caller cancelled the check, retaining its original cause.
    Cancelled(CancelCause),
    /// A listed successor has no state in the checker graph.
    MissingSuccessor,
    /// The graph's edge count exceeds the representable in-memory state count.
    TooManyEdges,
    /// The graph is cyclic; cyclic certificates require the later
    /// unbounded-certificate checking path.
    Cycle,
    /// The checker-side model expansion was undecided or refused.
    ExpansionRefused,
    /// The checker-side graph exceeded its configured state bound.
    LimitReached,
    /// The checker-side graph exceeded its configured edge bound.
    EdgeLimitReached,
    /// The model API exposed a draw that was not a single-state probability.
    UnsupportedDraw,
    /// A digital key contains an invalid or inconsistent timed component.
    InvalidDigitalKey,
    /// The model has no single initial state; this slice checks exactly one.
    UnsupportedInitial,
    /// The claim's `over` parameter has not exactly one instance in the
    /// subject; this slice checks one binding.
    UnsupportedBinding,
    /// No unique checked claim has the requested complete obligation identity.
    ClaimNotBound,
    /// The bound claim is not an unbounded `eventually` reachability claim,
    /// the only form this bounded slice checks.
    UnsupportedClaimForm,
}

/// Enumerate an unbound reachability graph from the checked model itself.
/// Product keys use the FR-196 v1 array with only the model component set;
/// monitor, accumulator, and pending controls are null in this bounded slice.
///
/// This adapter intentionally refuses draws with multiple post-states and
/// stops at explicit limits. It is a bounded exact-check foundation, not a
/// complete probabilistic product implementation.
#[allow(clippy::too_many_arguments)]
fn enumerate_model_graph<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: &P,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    max_states: usize,
    max_edges: usize,
    cancel: &Cancel,
    stop_at_target: bool,
    visit: &mut impl FnMut(&ProbabilityProductKey, &ModelSimState),
    failure: &mut impl FnMut(crate::ExactProbBuildIssue),
) -> Result<CheckerGraph, ExactProbabilityGraphError> {
    let key_of = |state: &ModelSimState| {
        let (model_bytes, _) = system
            .state_key(state)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        product_key(binding, &model_bytes)
    };
    enumerate_model_graph_with_keys(
        system,
        Some(target),
        binding,
        workload,
        max_states,
        max_edges,
        cancel,
        stop_at_target,
        &key_of,
        &mut |key, state| {
            visit(key, state);
            Ok(())
        },
        failure,
    )
}

#[allow(clippy::too_many_arguments)]
fn enumerate_model_graph_with_keys<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: Option<&P>,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    max_states: usize,
    max_edges: usize,
    cancel: &Cancel,
    stop_at_target: bool,
    key_of: &impl Fn(&ModelSimState) -> Result<ProbabilityProductKey, ExactProbabilityGraphError>,
    visit: &mut impl FnMut(
        &ProbabilityProductKey,
        &ModelSimState,
    ) -> Result<(), ExactProbabilityGraphError>,
    failure: &mut impl FnMut(crate::ExactProbBuildIssue),
) -> Result<CheckerGraph, ExactProbabilityGraphError> {
    enumerate_model_graph_with_keys_and_reservation(
        system,
        target,
        binding,
        workload,
        max_states,
        max_edges,
        cancel,
        stop_at_target,
        key_of,
        &mut |_| Ok(()),
        #[cfg(feature = "test-support")]
        &mut |_| {},
        visit,
        failure,
    )
}

#[allow(clippy::too_many_arguments)]
fn enumerate_model_graph_with_keys_and_reservation<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: Option<&P>,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    max_states: usize,
    max_edges: usize,
    cancel: &Cancel,
    stop_at_target: bool,
    key_of: &impl Fn(&ModelSimState) -> Result<ProbabilityProductKey, ExactProbabilityGraphError>,
    reserve: &mut impl FnMut(usize) -> Result<(), ExactProbabilityGraphError>,
    #[cfg(feature = "test-support")] observe: &mut dyn FnMut(ProbabilityWitnessSemanticPhase),
    visit: &mut impl FnMut(
        &ProbabilityProductKey,
        &ModelSimState,
    ) -> Result<(), ExactProbabilityGraphError>,
    failure: &mut impl FnMut(crate::ExactProbBuildIssue),
) -> Result<CheckerGraph, ExactProbabilityGraphError> {
    let mut distributions =
        |state: &ModelSimState, failure: &mut dyn FnMut(crate::ExactProbBuildIssue)| {
            model_distributions_observing(
                system,
                state,
                workload,
                cancel,
                #[cfg(feature = "test-support")]
                observe,
                failure,
            )
        };
    enumerate_model_graph_with_distributions(
        system,
        target,
        binding,
        max_states,
        max_edges,
        cancel,
        stop_at_target,
        key_of,
        reserve,
        &mut distributions,
        visit,
        failure,
    )
}

#[allow(clippy::too_many_arguments)]
fn enumerate_model_graph_with_distributions<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: Option<&P>,
    binding: Option<&ObjectReference>,
    max_states: usize,
    max_edges: usize,
    cancel: &Cancel,
    stop_at_target: bool,
    key_of: &impl Fn(&ModelSimState) -> Result<ProbabilityProductKey, ExactProbabilityGraphError>,
    reserve: &mut impl FnMut(usize) -> Result<(), ExactProbabilityGraphError>,
    distributions: &mut impl FnMut(
        &ModelSimState,
        &mut dyn FnMut(crate::ExactProbBuildIssue),
    ) -> Result<
        Vec<Vec<(Rational, ModelSimState)>>,
        ExactProbabilityGraphError,
    >,
    visit: &mut impl FnMut(
        &ProbabilityProductKey,
        &ModelSimState,
    ) -> Result<(), ExactProbabilityGraphError>,
    failure: &mut impl FnMut(crate::ExactProbBuildIssue),
) -> Result<CheckerGraph, ExactProbabilityGraphError> {
    enumerate_model_graph_visiting(
        system,
        target,
        binding,
        max_states,
        max_edges,
        cancel,
        stop_at_target,
        key_of,
        reserve,
        distributions,
        ModelVisitPhase::BeforeTarget,
        &mut |key, state, _| visit(key, state),
        &mut |_| {},
        failure,
    )
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ModelVisitPhase {
    BeforeTarget,
    AfterTarget,
}

#[allow(clippy::too_many_arguments)]
fn enumerate_model_graph_visiting<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: Option<&P>,
    binding: Option<&ObjectReference>,
    max_states: usize,
    max_edges: usize,
    cancel: &Cancel,
    stop_at_target: bool,
    key_of: &impl Fn(&ModelSimState) -> Result<ProbabilityProductKey, ExactProbabilityGraphError>,
    reserve: &mut impl FnMut(usize) -> Result<(), ExactProbabilityGraphError>,
    distributions: &mut impl FnMut(
        &ModelSimState,
        &mut dyn FnMut(crate::ExactProbBuildIssue),
    ) -> Result<
        Vec<Vec<(Rational, ModelSimState)>>,
        ExactProbabilityGraphError,
    >,
    visit_phase: ModelVisitPhase,
    visit: &mut impl FnMut(
        &ProbabilityProductKey,
        &ModelSimState,
        Option<bool>,
    ) -> Result<(), ExactProbabilityGraphError>,
    undefined: &mut impl FnMut(fsl_eval::temporal::UndefinedCause),
    failure: &mut impl FnMut(crate::ExactProbBuildIssue),
) -> Result<CheckerGraph, ExactProbabilityGraphError> {
    poll_cancel(cancel)?;
    let initial = system.initial();
    if initial.len() != 1 {
        return Err(ExactProbabilityGraphError::UnsupportedInitial);
    }
    if max_states == 0 {
        return Err(ExactProbabilityGraphError::LimitReached);
    }
    let first = initial
        .into_iter()
        .next()
        .ok_or(ExactProbabilityGraphError::UnsupportedInitial)?;
    let root = key_of(&first)?;
    let bytes = root
        .as_bytes()
        .len()
        .checked_mul(2)
        .and_then(|bytes| {
            bytes.checked_add(std::mem::size_of::<(ProbabilityProductKey, ModelSimState)>())
        })
        .and_then(|bytes| bytes.checked_add(std::mem::size_of::<ProbabilityProductKey>()))
        .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
    reserve(bytes)?;
    let mut pending = VecDeque::from([(root.clone(), first)]);
    let mut states = BTreeMap::new();
    let mut seen = BTreeSet::from([root.clone()]);
    let mut edge_count = 0usize;
    while let Some((key, state)) = pending.pop_front() {
        poll_cancel(cancel)?;
        if visit_phase == ModelVisitPhase::BeforeTarget {
            visit(&key, &state, None)?;
        }
        let terminal = if let Some(target) = target {
            let reading = system
                .state_predicates_with_cancel(&state, &[(target, binding)], cancel)
                .map_err(|stop| {
                    if let Err(error) = poll_cancel(cancel) {
                        return error;
                    }
                    failure(crate::ExactProbBuildIssue::Expansion(stop));
                    ExactProbabilityGraphError::ExpansionRefused
                })?
                .into_iter()
                .next()
                .ok_or(ExactProbabilityGraphError::ExpansionRefused)?;
            match reading {
                fsl_eval::simulation::model::PredicateReading::True => Some(true),
                fsl_eval::simulation::model::PredicateReading::False => None,
                fsl_eval::simulation::model::PredicateReading::Refused(outcome) => {
                    failure(crate::ExactProbBuildIssue::Predicate(outcome));
                    return Err(ExactProbabilityGraphError::ExpansionRefused);
                }
                fsl_eval::simulation::model::PredicateReading::Undefined { cause, .. } => {
                    undefined(cause);
                    return Err(ExactProbabilityGraphError::ExpansionRefused);
                }
            }
        } else {
            None
        };
        if visit_phase == ModelVisitPhase::AfterTarget {
            visit(&key, &state, terminal)?;
        }
        let choices = if terminal.is_some() && stop_at_target {
            Vec::new()
        } else {
            let distributions = distributions(&state, failure)?;
            if distributions.is_empty() {
                reserve(std::mem::size_of::<(ProbabilityProductKey, CheckerState)>())?;
                states.insert(
                    key,
                    CheckerState {
                        terminal: terminal.or(Some(false)),
                        choices: Vec::new(),
                    },
                );
                continue;
            }
            reserve(
                distributions
                    .len()
                    .checked_mul(std::mem::size_of::<Vec<ProbabilityEdge>>())
                    .ok_or(ExactProbabilityGraphError::TooManyEdges)?,
            )?;
            let mut choices = Vec::with_capacity(distributions.len());
            for distribution in distributions {
                poll_cancel(cancel)?;
                let mut choice = Vec::new();
                for (probability, successor) in distribution {
                    poll_cancel(cancel)?;
                    if probability == rational(0) {
                        continue;
                    }
                    let target_key = key_of(&successor)?;
                    edge_count = edge_count
                        .checked_add(1)
                        .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
                    if edge_count > max_edges {
                        return Err(ExactProbabilityGraphError::EdgeLimitReached);
                    }
                    reserve(
                        target_key
                            .as_bytes()
                            .len()
                            .checked_add(std::mem::size_of::<ProbabilityEdge>())
                            .ok_or(ExactProbabilityGraphError::TooManyEdges)?,
                    )?;
                    choice.push(ProbabilityEdge {
                        target: target_key.clone(),
                        probability,
                    });
                    if !seen.contains(&target_key) {
                        if seen.len() >= max_states {
                            return Err(ExactProbabilityGraphError::LimitReached);
                        }
                        reserve(
                            target_key
                                .as_bytes()
                                .len()
                                .checked_add(std::mem::size_of::<ProbabilityProductKey>())
                                .and_then(|bytes| {
                                    bytes.checked_add(std::mem::size_of::<(
                                        ProbabilityProductKey,
                                        ModelSimState,
                                    )>())
                                })
                                .ok_or(ExactProbabilityGraphError::TooManyEdges)?,
                        )?;
                        seen.insert(target_key.clone());
                        pending.push_back((target_key, successor));
                    }
                }
                choices.push(choice);
            }
            choices
        };
        reserve(std::mem::size_of::<(ProbabilityProductKey, CheckerState)>())?;
        states.insert(key, CheckerState { terminal, choices });
    }
    Ok(CheckerGraph {
        initial: root,
        states,
    })
}

fn product_key(
    binding: Option<&ObjectReference>,
    model_bytes: &[u8],
) -> Result<ProbabilityProductKey, ExactProbabilityGraphError> {
    ProbabilityProductKey::for_model_state_with_limits(
        binding,
        model_bytes,
        ProbabilityDocumentIoLimits::default(),
        &Cancel::new(),
    )
    .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)
}

fn model_distributions(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    workload: Option<&Workload>,
    cancel: &Cancel,
    failure: &mut impl FnMut(crate::ExactProbBuildIssue),
) -> Result<Vec<Vec<(Rational, ModelSimState)>>, ExactProbabilityGraphError> {
    model_distributions_observing(
        system,
        state,
        workload,
        cancel,
        #[cfg(feature = "test-support")]
        &mut |_| {},
        failure,
    )
}

fn model_distributions_observing(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    workload: Option<&Workload>,
    cancel: &Cancel,
    #[cfg(feature = "test-support")] observe: &mut dyn FnMut(ProbabilityWitnessSemanticPhase),
    failure: &mut dyn FnMut(crate::ExactProbBuildIssue),
) -> Result<Vec<Vec<(Rational, ModelSimState)>>, ExactProbabilityGraphError> {
    let mut expansion = |stop| {
        if let Err(error) = poll_cancel(cancel) {
            return error;
        }
        failure(crate::ExactProbBuildIssue::Expansion(stop));
        ExactProbabilityGraphError::ExpansionRefused
    };
    if let Some(workload) = workload {
        #[cfg(feature = "test-support")]
        observe(ProbabilityWitnessSemanticPhase::ExpansionEntered);
        let result = system.weighted_steps_with_cancel(state, workload, cancel);
        #[cfg(feature = "test-support")]
        if cancel.tripped().is_none() {
            observe(ProbabilityWitnessSemanticPhase::ExpansionReturned);
        }
        let steps = result.map_err(&mut expansion)?.map_err(|stop| {
            failure(match stop {
                fsl_eval::simulation::model::Unsteppable::NotMarkov(_) => {
                    crate::ExactProbBuildIssue::NotMarkov
                }
                fsl_eval::simulation::model::Unsteppable::UndecidedSuccessor(_) => {
                    crate::ExactProbBuildIssue::UndecidedSuccessor
                }
            });
            ExactProbabilityGraphError::ExpansionRefused
        })?;
        return workload_distribution(steps);
    }
    #[cfg(feature = "test-support")]
    observe(ProbabilityWitnessSemanticPhase::ExpansionEntered);
    let result = system.actions_with_cancel(state, cancel);
    #[cfg(feature = "test-support")]
    if cancel.tripped().is_none() {
        observe(ProbabilityWitnessSemanticPhase::ExpansionReturned);
    }
    let actions = result.map_err(&mut expansion)?.map_err(|_| {
        failure(crate::ExactProbBuildIssue::UndecidedSuccessor);
        ExactProbabilityGraphError::ExpansionRefused
    })?;
    action_distributions(actions, cancel)
}

fn model_distributions_with_budget(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    workload: Option<&Workload>,
    identity: fsl_foundation::IdentityLimits,
    admission: &mut ProbabilityKeyAdmission<'_>,
    cancel: &Cancel,
) -> Result<Vec<Vec<(Rational, ModelSimState)>>, ProbabilityDocumentError> {
    if let Some(workload) = workload {
        let steps = system
            .weighted_steps_with_budget(state, workload, identity, admission, cancel)
            .map_err(|error| admission.expansion_refusal(error))?
            .map_err(|stop| {
                ProbabilityDocumentError::Build(Box::new(match stop {
                    fsl_eval::simulation::model::Unsteppable::NotMarkov(_) => {
                        crate::ExactProbBuildIssue::NotMarkov
                    }
                    fsl_eval::simulation::model::Unsteppable::UndecidedSuccessor(_) => {
                        crate::ExactProbBuildIssue::UndecidedSuccessor
                    }
                }))
            })?;
        return workload_distribution(steps).map_err(Into::into);
    }
    let actions = system
        .actions_with_budget(state, identity, admission, cancel)
        .map_err(|error| admission.expansion_refusal(error))?
        .map_err(|_| {
            ProbabilityDocumentError::Build(Box::new(
                crate::ExactProbBuildIssue::UndecidedSuccessor,
            ))
        })?;
    action_distributions(actions, cancel).map_err(Into::into)
}

fn workload_distribution(
    steps: Vec<fsl_eval::simulation::model::WeightedStep>,
) -> Result<Vec<Vec<(Rational, ModelSimState)>>, ExactProbabilityGraphError> {
    if steps.iter().any(|step| {
        matches!(
            step.transition,
            fsl_eval::simulation::model::TransitionId::Stutter
        )
    }) {
        return if steps.len() == 1 {
            Ok(Vec::new())
        } else {
            Err(ExactProbabilityGraphError::UnsupportedDraw)
        };
    }
    Ok(vec![steps
        .into_iter()
        .map(|step| (step.probability, step.post))
        .collect()])
}

fn action_distributions(
    actions: Vec<fsl_eval::simulation::model::Action>,
    cancel: &Cancel,
) -> Result<Vec<Vec<(Rational, ModelSimState)>>, ExactProbabilityGraphError> {
    let mut distributions = Vec::with_capacity(actions.len());
    for action in actions {
        poll_cancel(cancel)?;
        let mut distribution = Vec::with_capacity(action.draws.len());
        for draw in action.draws {
            poll_cancel(cancel)?;
            if draw.probability == rational(0) {
                continue;
            }
            if draw.successors.len() != 1 {
                return Err(ExactProbabilityGraphError::UnsupportedDraw);
            }
            let (_, successor) = draw
                .successors
                .into_iter()
                .next()
                .ok_or(ExactProbabilityGraphError::UnsupportedDraw)?;
            distribution.push((draw.probability, successor));
        }
        distributions.push(distribution);
    }
    Ok(distributions)
}

fn claim_workload<'a>(
    system: &ModelSystem<'a>,
    claim: &CheckedProbabilisticClaim,
) -> Result<Option<&'a Workload>, ExactProbabilityGraphError> {
    match &claim.scheduler {
        ProbScheduler::Every => Ok(None),
        ProbScheduler::Workload(name) => system
            .input()
            .probabilistic
            .workload(name)
            .filter(|workload| workload.model() == claim.subject.alias)
            .map(Some)
            .ok_or(ExactProbabilityGraphError::UnsupportedClaimForm),
    }
}

/// Independently enumerate and check a bounded unbound exact reachability
/// certificate against the supplied checked model system.
///
/// The caller constructs `system` from replay/recompiled subject inputs and
/// supplies the checked claim predicate. The certificate cannot supply graph
/// states or transitions. This slice handles one initial state and exact
/// draws with one successor each; unsupported shapes return an error.
#[allow(clippy::too_many_arguments)]
pub(crate) fn check_model_exact_reachability<P: StatePredicate>(
    system: &ModelSystem<'_>,
    target: &P,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    certificate: &ExactProbabilityCertificate,
    objective: ProbabilityObjective,
    bound: &Bound,
    (max_states, max_edges): (usize, usize),
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    let graph = enumerate_model_graph(
        system,
        target,
        binding,
        workload,
        max_states,
        max_edges,
        cancel,
        true,
        &mut |_, _| {},
        &mut |_| {},
    )?;
    check_exact_reachability(
        &graph,
        certificate,
        objective,
        bound,
        GraphCycles::Reject,
        cancel,
    )
}

/// Bind `item` to its checked claim by complete obligation identity, then check the
/// certificate over that claim's target predicate and bound (FR-498).
///
/// The claim is the one whose `occurrence().node()` equals the obligation's
/// node; name, span and source order never select it. A claim without a
/// package identity is never bound.
/// Cancellation is polled while binding, enumerating and validating the
/// certificate; a stop retains the caller's cause and yields no acceptance.
pub fn check_claim_exact_reachability(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    max_states: usize,
    max_edges: usize,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    let claim = bind_claim(claims, item, cancel)?;
    let ProbForm::Reach {
        bound,
        target: ReachTarget::Eventually(predicate),
    } = &claim.form
    else {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm);
    };
    let objective = claim_objective(&claim.scheduler, bound);
    let instance = claim_binding(system, claim)?;
    check_model_exact_reachability(
        system,
        predicate,
        instance.as_ref().map(|bound| &bound.reference),
        claim_workload(system, claim)?,
        certificate,
        objective,
        bound,
        (max_states, max_edges),
        cancel,
    )
}

/// Derive the complete request-item identity from the checked claim's own
/// occurrence using the qualified planner's claim identity contract. The
/// subject carries the owning model; the actual occurrence ordinal is retained.
///
/// # Errors
///
/// Returns `ClaimNotBound` for an unassembled claim or an unrepresentable ordinal.
pub fn probabilistic_claim_identity(
    claim: &CheckedProbabilisticClaim,
) -> Result<ObligationIdentity, ExactProbabilityGraphError> {
    let occurrence = claim
        .occurrence()
        .ok_or(ExactProbabilityGraphError::ClaimNotBound)?;
    if u32::try_from(occurrence.origin().ordinal()).is_err() {
        return Err(ExactProbabilityGraphError::ClaimNotBound);
    }
    Ok(ObligationIdentity::from_claim_occurrence(occurrence))
}

fn bind_claim<'a>(
    claims: &'a [CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    cancel: &Cancel,
) -> Result<&'a CheckedProbabilisticClaim, ExactProbabilityGraphError> {
    poll_cancel(cancel)?;
    let mut bound_claim = None;
    for claim in claims {
        poll_cancel(cancel)?;
        if claim
            .occurrence()
            .is_some_and(|key| key.node() == item.node())
            && (&probabilistic_claim_identity(claim)? != item
                || bound_claim.replace(claim).is_some())
        {
            return Err(ExactProbabilityGraphError::ClaimNotBound);
        }
    }
    bound_claim.ok_or(ExactProbabilityGraphError::ClaimNotBound)
}

/// The one `over` instance of a claim and the reference its predicates read.
#[derive(Clone, Debug)]
pub struct ClaimBinding {
    /// The instance in its population's universe.
    pub instance: ModelInstance,
    /// The object reference the instance binds the claim's parameter to.
    pub reference: ObjectReference,
}

/// The binding of `claim`'s `over` parameter in `system`: `None` for a claim
/// with no `over`, the one instance otherwise.
///
/// # Errors
///
/// [`ExactProbabilityGraphError::UnsupportedBinding`] when the parameter is
/// not an object reference or has no instance or several (this slice checks
/// one); [`ExactProbabilityGraphError::ExpansionRefused`] when the subject
/// cannot enumerate or reference them.
pub fn claim_binding(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
) -> Result<Option<ClaimBinding>, ExactProbabilityGraphError> {
    let Some(over) = &claim.over else {
        return Ok(None);
    };
    let filament_exact::ValueType::Reference(object) = &over.value_type else {
        return Err(ExactProbabilityGraphError::UnsupportedBinding);
    };
    let mut instances = system
        .instances(&over.value_type)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    let (Some(instance), None) = (instances.pop(), instances.pop()) else {
        return Err(ExactProbabilityGraphError::UnsupportedBinding);
    };
    let reference = system
        .bound_reference(object.clone(), &instance)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    Ok(Some(ClaimBinding {
        instance,
        reference,
    }))
}

/// The scheduler objective the claim itself fixes (FR-198): a claim `under
/// every scheduler` must hold at the worst scheduler, the minimum for a `>=`
/// bound and the maximum for a `<=` bound; a workload has one distribution.
fn claim_objective(scheduler: &ProbScheduler, bound: &Bound) -> ProbabilityObjective {
    match (scheduler, bound) {
        (ProbScheduler::Workload(_), _) => ProbabilityObjective::Workload,
        (ProbScheduler::Every, Bound::AtLeast(_)) => ProbabilityObjective::Minimum,
        (ProbScheduler::Every, Bound::AtMost(_)) => ProbabilityObjective::Maximum,
    }
}

/// One successor and its exact probability mass.
#[derive(Clone, Debug)]
struct ProbabilityEdge {
    target: ProbabilityProductKey,
    probability: Rational,
}

/// One action's probabilistic successors.
type ProbabilityChoice = Vec<ProbabilityEdge>;

/// One checker-enumerated state. This type is private so an eventual replay
/// facade owns graph construction instead of trusting solver data.
#[derive(Clone, Debug)]
struct CheckerState {
    terminal: Option<bool>,
    choices: Vec<ProbabilityChoice>,
}

/// The graph produced by a checker-side enumerator.
#[derive(Clone, Debug)]
struct CheckerGraph {
    initial: ProbabilityProductKey,
    states: BTreeMap<ProbabilityProductKey, CheckerState>,
}

fn topological_order(
    graph: &CheckerGraph,
    cancel: &Cancel,
) -> Result<Vec<ProbabilityProductKey>, ExactProbabilityGraphError> {
    let zero = rational(0);
    let mut indegree = BTreeMap::new();
    for key in graph.states.keys() {
        poll_cancel(cancel)?;
        indegree.insert(key.clone(), 0_usize);
    }
    for state in graph.states.values() {
        for edge in state.choices.iter().flatten() {
            poll_cancel(cancel)?;
            if edge.probability == zero {
                continue;
            }
            let Some(count) = indegree.get_mut(&edge.target) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            *count = count
                .checked_add(1)
                .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
        }
    }
    let mut ready = BTreeSet::new();
    for (key, degree) in &indegree {
        poll_cancel(cancel)?;
        if *degree == 0 {
            ready.insert(key.clone());
        }
    }
    let mut order = Vec::with_capacity(graph.states.len());
    while let Some(key) = ready.pop_first() {
        poll_cancel(cancel)?;
        order.push(key.clone());
        let Some(state) = graph.states.get(&key) else {
            return Err(ExactProbabilityGraphError::MissingSuccessor);
        };
        for edge in state.choices.iter().flatten() {
            poll_cancel(cancel)?;
            if edge.probability == zero {
                continue;
            }
            let Some(count) = indegree.get_mut(&edge.target) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            let Some(remaining) = count.checked_sub(1) else {
                return Err(ExactProbabilityGraphError::MissingSuccessor);
            };
            *count = remaining;
            if *count == 0 {
                ready.insert(edge.target.clone());
            }
        }
    }
    if order.len() == graph.states.len() {
        Ok(order)
    } else {
        Err(ExactProbabilityGraphError::Cycle)
    }
}

/// How a checked graph's cycles are treated.
#[derive(Clone, Copy)]
enum GraphCycles {
    /// The graph must be acyclic; any cycle is an error.
    Reject,
    /// Timed deadline graphs: closed nonprogressive action components are
    /// collapsed to their best exit.
    CollapseNonprogressive,
}

/// Check exact reachability values on a checker-owned graph.
///
/// The graph representation and this evaluator are independent of
/// `fsl-analyze`. Under [`GraphCycles::Reject`] a cyclic graph is an error.
/// Under [`GraphCycles::CollapseNonprogressive`] closed nonprogressive action
/// components are reconstructed here and checked against their best exit
/// values before ordinary Bellman equations are checked.
fn check_exact_reachability(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
    objective: ProbabilityObjective,
    bound: &Bound,
    cycles: GraphCycles,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ExactProbabilityGraphError> {
    poll_cancel(cancel)?;
    let values = match certificate_values(graph, certificate, cancel)? {
        CertificateValues::Complete(values) => values,
        CertificateValues::Rejected(rejection) => {
            return Ok(ExactProbabilityCertificateCheck::Rejected(rejection));
        }
    };
    let zero = rational(0);
    let one = rational(1);
    let components = match cycles {
        GraphCycles::Reject => {
            topological_order(graph, cancel)?;
            Vec::new()
        }
        GraphCycles::CollapseNonprogressive => nonprogressive_end_components(graph, cancel, None)?,
    };
    let mut component_of = BTreeMap::new();
    for (component, members) in components.iter().enumerate() {
        for key in members {
            component_of.insert(key.clone(), component);
        }
    }
    for members in &components {
        poll_cancel(cancel)?;
        let member_set = members.iter().cloned().collect::<BTreeSet<_>>();
        let mut exit_values = Vec::new();
        for key in members {
            let Some(state) = graph.states.get(key) else {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            };
            for choice in &state.choices {
                poll_cancel(cancel)?;
                let mut mass = zero.clone();
                let mut internal = zero.clone();
                let mut outside = zero.clone();
                for edge in choice {
                    poll_cancel(cancel)?;
                    if edge.probability < zero {
                        return Ok(rejected(
                            ProbabilityCertificateRule::InequalityFails,
                            key.clone(),
                        ));
                    }
                    if edge.probability == zero {
                        continue;
                    }
                    let Some(successor) = values.get(&edge.target) else {
                        return Ok(rejected(
                            ProbabilityCertificateRule::InequalityFails,
                            key.clone(),
                        ));
                    };
                    mass = mass.add(&edge.probability);
                    if member_set.contains(&edge.target) {
                        internal = internal.add(&edge.probability);
                    } else {
                        outside = outside.add(&edge.probability.mul(successor));
                    }
                }
                if mass != one {
                    return Ok(rejected(
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    ));
                }
                let denominator = one.sub(&internal);
                if !denominator.is_zero() {
                    if let Some(value) = outside.div(&denominator) {
                        exit_values.push(value);
                    }
                }
            }
        }
        let actual = match objective {
            ProbabilityObjective::Minimum => exit_values.into_iter().min(),
            ProbabilityObjective::Maximum => exit_values.into_iter().max(),
            ProbabilityObjective::Workload => {
                return Err(ExactProbabilityGraphError::UnsupportedClaimForm);
            }
        }
        .unwrap_or_else(|| zero.clone());
        for key in members {
            if values.get(key) != Some(&actual) {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
        }
    }
    for (key, state) in &graph.states {
        poll_cancel(cancel)?;
        if component_of.contains_key(key) {
            continue;
        }
        let actual = if let Some(target) = state.terminal {
            if target {
                one.clone()
            } else {
                zero.clone()
            }
        } else {
            if state.choices.is_empty()
                || (objective == ProbabilityObjective::Workload && state.choices.len() != 1)
            {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    key.clone(),
                ));
            }
            let mut choice_values = Vec::with_capacity(state.choices.len());
            for choice in &state.choices {
                poll_cancel(cancel)?;
                let mut mass = zero.clone();
                let mut value = zero.clone();
                for edge in choice {
                    poll_cancel(cancel)?;
                    if edge.probability < zero {
                        return Ok(rejected(
                            ProbabilityCertificateRule::InequalityFails,
                            key.clone(),
                        ));
                    }
                    if edge.probability == zero {
                        continue;
                    }
                    let Some(successor) = values.get(&edge.target) else {
                        return Ok(rejected(
                            ProbabilityCertificateRule::InequalityFails,
                            key.clone(),
                        ));
                    };
                    mass = mass.add(&edge.probability);
                    value = value.add(&edge.probability.mul(successor));
                }
                if mass != one {
                    return Ok(rejected(
                        ProbabilityCertificateRule::InequalityFails,
                        key.clone(),
                    ));
                }
                choice_values.push(value);
            }
            match objective {
                ProbabilityObjective::Minimum => choice_values.into_iter().min(),
                ProbabilityObjective::Maximum => choice_values.into_iter().max(),
                ProbabilityObjective::Workload => choice_values.into_iter().next(),
            }
            .unwrap_or_else(|| zero.clone())
        };
        let Some(certified) = values.get(key) else {
            return Ok(rejected(ProbabilityCertificateRule::Malformed, key.clone()));
        };
        if actual != *certified {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                key.clone(),
            ));
        }
    }

    let Some(initial) = values.get(&graph.initial) else {
        return Ok(rejected(
            ProbabilityCertificateRule::InequalityFails,
            graph.initial.clone(),
        ));
    };
    let meets_bound = match bound {
        Bound::AtLeast(threshold) => initial >= threshold,
        Bound::AtMost(threshold) => initial <= threshold,
    };
    if !meets_bound {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial.clone(),
        ));
    }
    poll_cancel(cancel)?;
    Ok(ExactProbabilityCertificateCheck::Accepted(
        ExactProbabilityAcceptance {
            initial_values: None,
            document: None,
            value: initial.clone(),
            certificate: certificate.clone(),
            side: match bound {
                Bound::AtLeast(_) => ProbabilityBoundSide::AtLeast,
                Bound::AtMost(_) => ProbabilityBoundSide::AtMost,
            },
        },
    ))
}

enum CertificateValues {
    Complete(BTreeMap<ProbabilityProductKey, Rational>),
    Rejected(ProbabilityCertificateRejection),
}

fn certificate_values(
    graph: &CheckerGraph,
    certificate: &ExactProbabilityCertificate,
    cancel: &Cancel,
) -> Result<CertificateValues, ExactProbabilityGraphError> {
    let mut values = BTreeMap::new();
    let mut previous = None;
    for entry in &certificate.values {
        poll_cancel(cancel)?;
        if previous.as_ref().is_some_and(|key| key >= &entry.state) {
            return Ok(CertificateValues::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::Malformed,
                    at: ProbabilityCertificateLocus::Member("values"),
                },
            ));
        }
        previous = Some(entry.state.clone());
        if values
            .insert(entry.state.clone(), entry.value.clone())
            .is_some()
            || !graph.states.contains_key(&entry.state)
        {
            return Ok(CertificateValues::Rejected(
                ProbabilityCertificateRejection {
                    rule: ProbabilityCertificateRule::Malformed,
                    at: ProbabilityCertificateLocus::Member("values"),
                },
            ));
        }
    }
    if values.len() != graph.states.len() {
        poll_cancel(cancel)?;
        return Ok(CertificateValues::Rejected(
            ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member("values"),
            },
        ));
    }
    Ok(CertificateValues::Complete(values))
}

/// Independently enumerate the maximal checker-side components whose action
/// support is closed and does not advance the checked deadline clock.
fn nonprogressive_end_components(
    graph: &CheckerGraph,
    cancel: &Cancel,
    costs: Option<&BTreeMap<ProbabilityProductKey, Vec<Vec<Rational>>>>,
) -> Result<Vec<Vec<ProbabilityProductKey>>, ExactProbabilityGraphError> {
    let keys = graph.states.keys().cloned().collect::<Vec<_>>();
    let indices = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.clone(), index))
        .collect::<BTreeMap<_, _>>();
    let mut pending = vec![(0..keys.len()).collect::<BTreeSet<_>>()];
    let mut result: Vec<Vec<ProbabilityProductKey>> = Vec::new();
    let zero = rational(0);
    while let Some(mut candidate) = pending.pop() {
        loop {
            poll_cancel(cancel)?;
            let mut retained = BTreeSet::new();
            for state_index in &candidate {
                let Some(key) = keys.get(*state_index) else {
                    return Err(ExactProbabilityGraphError::MissingSuccessor);
                };
                let Some(state) = graph.states.get(key) else {
                    return Err(ExactProbabilityGraphError::MissingSuccessor);
                };
                let mut has_internal_action = false;
                for (choice_index, choice) in state.choices.iter().enumerate() {
                    let choice_costs = costs
                        .and_then(|costs| costs.get(key))
                        .and_then(|costs| costs.get(choice_index))
                        .map(Vec::as_slice);
                    if internal_nonprogressive_choice(
                        key,
                        choice,
                        choice_costs,
                        &candidate,
                        &indices,
                        &zero,
                    )? {
                        has_internal_action = true;
                        break;
                    }
                }
                if has_internal_action {
                    retained.insert(*state_index);
                }
            }
            if retained == candidate {
                break;
            }
            candidate = retained;
            if candidate.is_empty() {
                break;
            }
        }
        if candidate.is_empty() {
            continue;
        }
        let mut adjacency = vec![Vec::new(); keys.len()];
        let mut reverse = vec![Vec::new(); keys.len()];
        for state_index in &candidate {
            let key = keys
                .get(*state_index)
                .ok_or(ExactProbabilityGraphError::MissingSuccessor)?;
            let state = graph
                .states
                .get(key)
                .ok_or(ExactProbabilityGraphError::MissingSuccessor)?;
            for (choice_index, choice) in state.choices.iter().enumerate() {
                let choice_costs = costs
                    .and_then(|costs| costs.get(key))
                    .and_then(|costs| costs.get(choice_index))
                    .map(Vec::as_slice);
                if !internal_nonprogressive_choice(
                    key,
                    choice,
                    choice_costs,
                    &candidate,
                    &indices,
                    &zero,
                )? {
                    continue;
                }
                for edge in choice.iter().filter(|edge| edge.probability > zero) {
                    let target = *indices
                        .get(&edge.target)
                        .ok_or(ExactProbabilityGraphError::MissingSuccessor)?;
                    adjacency[*state_index].push(target);
                    reverse[target].push(*state_index);
                }
            }
        }
        let components = restricted_components(&candidate, &adjacency, &reverse, cancel)?;
        if components.len() == 1 && components[0] == candidate {
            result.push(candidate.iter().map(|index| keys[*index].clone()).collect());
        } else {
            pending.extend(components);
        }
    }
    result.sort_by(|left, right| left.first().cmp(&right.first()));
    Ok(result)
}

fn internal_nonprogressive_choice(
    source: &ProbabilityProductKey,
    choice: &ProbabilityChoice,
    costs: Option<&[Rational]>,
    candidate: &BTreeSet<usize>,
    indices: &BTreeMap<ProbabilityProductKey, usize>,
    zero: &Rational,
) -> Result<bool, ExactProbabilityGraphError> {
    let mut has_support = false;
    let clocks = timed_clock_count(source);
    let has_deadline = timed_has_deadline(source);
    let mut positive_cost = false;
    for (edge_index, edge) in choice.iter().enumerate() {
        if edge.probability <= *zero {
            continue;
        }
        has_support = true;
        let Some(target) = indices.get(&edge.target) else {
            return Err(ExactProbabilityGraphError::MissingSuccessor);
        };
        if !candidate.contains(target) || timed_key_progresses(source, &edge.target) {
            return Ok(false);
        }
        if costs
            .and_then(|costs| costs.get(edge_index))
            .is_some_and(|cost| !cost.is_zero())
        {
            positive_cost = true;
        }
    }
    Ok(has_support && !(!has_deadline && clocks == Some(0) && positive_cost))
}

/// Whether a checker-key edge advances time beyond the current region.
fn timed_key_progresses(from: &ProbabilityProductKey, to: &ProbabilityProductKey) -> bool {
    fn timed_model(key: &ProbabilityProductKey) -> Option<Value> {
        let root: Value = serde_json::from_slice(key.as_bytes()).ok()?;
        Some(root.get(4)?.get(1)?.clone())
    }
    let (Some(from_model), Some(to_model)) = (timed_model(from), timed_model(to)) else {
        return false;
    };
    if let (Some(from_deadline), Some(to_deadline)) = (
        from_model.get(5).and_then(Value::as_array),
        to_model.get(5).and_then(Value::as_array),
    ) {
        if from_deadline.first().and_then(Value::as_str) == Some("deadline")
            && to_deadline.first().and_then(Value::as_str) == Some("deadline")
        {
            return from_deadline.get(2) != to_deadline.get(2);
        }
    }
    let (Some(from_clocks), Some(to_clocks)) = (
        from_model.get(4).and_then(Value::as_array),
        to_model.get(4).and_then(Value::as_array),
    ) else {
        return false;
    };
    to_clocks.iter().any(|to_clock| {
        let Some(identity) = to_clock.get(0) else {
            return true;
        };
        let Some(from_clock) = from_clocks
            .iter()
            .find(|from_clock| from_clock.get(0) == Some(identity))
        else {
            return false;
        };
        let Some(from_value) = from_clock.get(1).and_then(Value::as_str) else {
            return true;
        };
        let Some(to_value) = to_clock.get(1).and_then(Value::as_str) else {
            return true;
        };
        to_value.parse::<u64>().unwrap_or(0) > from_value.parse::<u64>().unwrap_or(0)
    })
}

fn timed_clock_count(key: &ProbabilityProductKey) -> Option<usize> {
    let root: Value = serde_json::from_slice(key.as_bytes()).ok()?;
    root.get(4)?.get(1)?.get(4)?.as_array().map(Vec::len)
}

fn timed_has_deadline(key: &ProbabilityProductKey) -> bool {
    let Some(root) = serde_json::from_slice::<Value>(key.as_bytes()).ok() else {
        return false;
    };
    root.get(4)
        .and_then(|model| model.get(1))
        .and_then(|timed| timed.get(5))
        .and_then(Value::as_array)
        .and_then(|deadline| deadline.first())
        .and_then(Value::as_str)
        == Some("deadline")
}

fn restricted_components(
    allowed: &BTreeSet<usize>,
    adjacency: &[Vec<usize>],
    reverse: &[Vec<usize>],
    cancel: &Cancel,
) -> Result<Vec<BTreeSet<usize>>, ExactProbabilityGraphError> {
    let mut visited = vec![false; adjacency.len()];
    let mut order = Vec::with_capacity(allowed.len());
    for root in allowed {
        poll_cancel(cancel)?;
        if visited.get(*root).copied().unwrap_or(true) {
            continue;
        }
        if let Some(slot) = visited.get_mut(*root) {
            *slot = true;
        }
        let mut work = vec![(*root, 0_usize)];
        while let Some((state, cursor)) = work.pop() {
            poll_cancel(cancel)?;
            let successors = adjacency
                .get(state)
                .ok_or(ExactProbabilityGraphError::MissingSuccessor)?;
            if let Some(target) = successors.get(cursor).copied() {
                work.push((state, cursor + 1));
                if allowed.contains(&target) && !visited[target] {
                    visited[target] = true;
                    work.push((target, 0));
                }
            } else {
                order.push(state);
            }
        }
    }
    visited.fill(false);
    let mut components = Vec::new();
    for root in order.into_iter().rev() {
        poll_cancel(cancel)?;
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut component = BTreeSet::new();
        let mut work = vec![root];
        while let Some(state) = work.pop() {
            poll_cancel(cancel)?;
            component.insert(state);
            for predecessor in reverse
                .get(state)
                .ok_or(ExactProbabilityGraphError::MissingSuccessor)?
            {
                if allowed.contains(predecessor) && !visited[*predecessor] {
                    visited[*predecessor] = true;
                    work.push(*predecessor);
                }
            }
        }
        components.push(component);
    }
    Ok(components)
}

fn poll_cancel(cancel: &Cancel) -> Result<(), ExactProbabilityGraphError> {
    if let Some(cause) = cancel.tripped() {
        return Err(ExactProbabilityGraphError::Cancelled(cause));
    }
    if cancel.poll() {
        if let Some(cause) = cancel.tripped() {
            return Err(ExactProbabilityGraphError::Cancelled(cause));
        }
    }
    Ok(())
}

fn rejected(
    rule: ProbabilityCertificateRule,
    state: ProbabilityProductKey,
) -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
        rule,
        at: ProbabilityCertificateLocus::ProductState(state),
    })
}

fn rational(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[trace("FR-203-AC-12")]
    #[test]
    fn checked_mixed_initial_entries_reject_missing_duplicate_and_nonactivation_values() {
        let value = Rational::new(Integer::from(3_i64), Integer::from(4_i64)).unwrap();
        let accepted = ExactProbabilityAcceptance {
            value: value.clone(),
            side: ProbabilityBoundSide::AtMost,
            certificate: ExactProbabilityCertificate { values: Vec::new() },
            document: None,
            initial_values: Some(CheckedInitialValues {
                values: vec![(0, None), (1, Some(value.clone()))],
                binding: None,
            }),
        };
        let entry = crate::ExactEntry {
            initial: 1,
            binding: None,
            value: crate::ExactOrBounds::Exact {
                value: crate::ExactRational::new(value),
            },
            extremum: None,
        };
        assert!(accepted.initial_entries_match(std::slice::from_ref(&entry)));
        assert!(!accepted.initial_entries_match(&[]));
        assert!(!accepted.initial_entries_match(&[entry.clone(), entry.clone()]));
        let mut nonactivating = entry;
        nonactivating.initial = 0;
        assert!(!accepted.initial_entries_match(&[nonactivating]));
        let check = ExactProbabilityCertificateCheck::Accepted(accepted);
        assert!(crate::TerminalRecord::proved_by_exact_certificate(
            fsl_foundation::RequestIndex::new(0),
            fsl_foundation::BackendId::new("en-5"),
            &check,
            None,
            None
        )
        .is_some());
        assert!(crate::TerminalRecord::proved_by_exact_certificate(
            fsl_foundation::RequestIndex::new(0),
            fsl_foundation::BackendId::new("en-5"),
            &check,
            Some(crate::Extremum::Min),
            None
        )
        .is_none());
    }

    fn key(value: u8) -> ProbabilityProductKey {
        ProbabilityProductKey(vec![value])
    }

    fn edge(target: u8, numerator: i64, denominator: i64) -> ProbabilityEdge {
        ProbabilityEdge {
            target: key(target),
            probability: Rational::new(Integer::from(numerator), Integer::from(denominator))
                .expect("nonzero denominator"),
        }
    }

    fn graph() -> CheckerGraph {
        let initial = key(0);
        let mut states = BTreeMap::new();
        states.insert(
            initial.clone(),
            CheckerState {
                terminal: None,
                choices: vec![
                    vec![edge(1, 4, 5), edge(2, 1, 5)],
                    vec![edge(1, 9, 10), edge(2, 1, 10)],
                ],
            },
        );
        states.insert(
            key(1),
            CheckerState {
                terminal: Some(true),
                choices: Vec::new(),
            },
        );
        states.insert(
            key(2),
            CheckerState {
                terminal: Some(false),
                choices: Vec::new(),
            },
        );
        CheckerGraph { initial, states }
    }

    fn zero_time_mec_graph() -> CheckerGraph {
        let initial = key(0);
        let accepted = key(1);
        let rejected = key(2);
        let mut states = BTreeMap::new();
        states.insert(
            initial.clone(),
            CheckerState {
                terminal: None,
                choices: vec![
                    vec![edge(0, 1, 1)],
                    vec![edge(1, 1, 1)],
                    vec![edge(2, 1, 1)],
                ],
            },
        );
        states.insert(
            accepted,
            CheckerState {
                terminal: Some(true),
                choices: Vec::new(),
            },
        );
        states.insert(
            rejected,
            CheckerState {
                terminal: Some(false),
                choices: Vec::new(),
            },
        );
        CheckerGraph { initial, states }
    }

    fn zero_time_mec_certificate(initial: i64) -> ExactProbabilityCertificate {
        ExactProbabilityCertificate {
            values: vec![
                ExactProbabilityValue {
                    state: key(0),
                    value: rational(initial),
                },
                ExactProbabilityValue {
                    state: key(1),
                    value: rational(1),
                },
                ExactProbabilityValue {
                    state: key(2),
                    value: rational(0),
                },
            ],
        }
    }

    #[test]
    fn checker_reconstructs_action_level_zero_time_mec_best_exit() {
        let graph = zero_time_mec_graph();
        let minimum = check_exact_reachability(
            &graph,
            &zero_time_mec_certificate(0),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(rational(0)),
            GraphCycles::CollapseNonprogressive,
            &Cancel::new(),
        )
        .expect("checker evaluates the minimum MEC exit");
        assert!(matches!(
            minimum,
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        let maximum = check_exact_reachability(
            &graph,
            &zero_time_mec_certificate(1),
            ProbabilityObjective::Maximum,
            &Bound::AtLeast(rational(1)),
            GraphCycles::CollapseNonprogressive,
            &Cancel::new(),
        )
        .expect("checker evaluates the maximum MEC exit");
        assert!(matches!(
            maximum,
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
        let forged = check_exact_reachability(
            &graph,
            &zero_time_mec_certificate(1),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(rational(0)),
            GraphCycles::CollapseNonprogressive,
            &Cancel::new(),
        )
        .expect("a non-optimal MEC value is a typed rejection");
        assert!(matches!(
            forged,
            ExactProbabilityCertificateCheck::Rejected(_)
        ));
    }

    #[test]
    fn checker_does_not_collapse_a_leaky_zero_time_cycle() {
        let initial = key(0);
        let target = key(1);
        let mut states = BTreeMap::new();
        states.insert(
            initial.clone(),
            CheckerState {
                terminal: None,
                choices: vec![vec![edge(0, 1, 2), edge(1, 1, 2)]],
            },
        );
        states.insert(
            target.clone(),
            CheckerState {
                terminal: Some(true),
                choices: Vec::new(),
            },
        );
        let graph = CheckerGraph { initial, states };
        let certificate = ExactProbabilityCertificate {
            values: vec![
                ExactProbabilityValue {
                    state: key(0),
                    value: rational(1),
                },
                ExactProbabilityValue {
                    state: target,
                    value: rational(1),
                },
            ],
        };
        let checked = check_exact_reachability(
            &graph,
            &certificate,
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(rational(1)),
            GraphCycles::CollapseNonprogressive,
            &Cancel::new(),
        )
        .expect("the leaky cycle has an exact checker value");
        assert!(matches!(
            checked,
            ExactProbabilityCertificateCheck::Accepted(_)
        ));
    }

    #[test]
    fn checker_mec_progress_uses_the_checked_deadline_clock() {
        let key = |elapsed| {
            digital_probability_product_key(DigitalProductKeyInput {
                binding: None,
                discrete: json!([]),
                unit: json!("second"),
                scale: 1,
                clocks: Vec::new(),
                deadline: Some(DigitalDeadlineKey { limit: 2, elapsed }),
                tail: DigitalTail::Running,
                monitor: Some(json!(["deadline", "eventually", "pending"])),
                accumulator: None,
                pending: None,
            })
            .expect("digital timed keys are valid")
        };
        assert!(!timed_key_progresses(&key(0), &key(0)));
        assert!(timed_key_progresses(&key(0), &key(1)));

        let no_clock = digital_probability_product_key(DigitalProductKeyInput {
            binding: None,
            discrete: json!([]),
            unit: json!("second"),
            scale: 1,
            clocks: Vec::new(),
            deadline: None,
            tail: DigitalTail::Running,
            monitor: None,
            accumulator: None,
            pending: None,
        })
        .expect("clockless timed key is valid");
        let one_clock = digital_probability_product_key(DigitalProductKeyInput {
            binding: None,
            discrete: json!([]),
            unit: json!("second"),
            scale: 1,
            clocks: vec![DigitalClockKey {
                identity: json!(["Type", "clock", ["u", "object"]]),
                value: 0,
                cap: 2,
            }],
            deadline: None,
            tail: DigitalTail::Running,
            monitor: None,
            accumulator: None,
            pending: None,
        })
        .expect("clocked timed key is valid");
        let graph = |state: ProbabilityProductKey| CheckerGraph {
            initial: state.clone(),
            states: BTreeMap::from([(
                state.clone(),
                CheckerState {
                    terminal: None,
                    choices: vec![vec![ProbabilityEdge {
                        target: state,
                        probability: rational(1),
                    }]],
                },
            )]),
        };
        let positive_cost = BTreeMap::from([(no_clock.clone(), vec![vec![rational(1)]])]);
        assert!(nonprogressive_end_components(
            &graph(no_clock),
            &Cancel::new(),
            Some(&positive_cost),
        )
        .expect("clockless time progress is understood")
        .is_empty());
        let reset_cost = BTreeMap::from([(one_clock.clone(), vec![vec![rational(1)]])]);
        assert_eq!(
            nonprogressive_end_components(&graph(one_clock), &Cancel::new(), Some(&reset_cost),)
                .expect("a positive-cost reset loop remains nonprogressive")
                .len(),
            1
        );
    }

    fn certificate(initial_value: Rational) -> ExactProbabilityCertificate {
        ExactProbabilityCertificate {
            values: vec![
                ExactProbabilityValue {
                    state: key(0),
                    value: initial_value,
                },
                ExactProbabilityValue {
                    state: key(1),
                    value: rational(1),
                },
                ExactProbabilityValue {
                    state: key(2),
                    value: rational(0),
                },
            ],
        }
    }

    #[trace("FSpec-FR-752-AC-1")]
    #[test]
    fn region_key_separates_largest_constant_from_saturated_clock() {
        let key_at_constant = region_probability_product_key(RegionProductKeyInput {
            binding: None,
            discrete: json!("model"),
            unit: json!("unit"),
            scale: 1,
            subdivisions: 3,
            clocks: vec![RegionClockKey {
                identity: json!(["type", "clock", "receiver"]),
                value: 6,
                cap: 7,
            }],
            deadline: None,
            tail: DigitalTail::Running,
            monitor: None,
            accumulator: None,
            pending: None,
            kind: RegionKeyKind::Region,
            corner: None,
        })
        .expect("the exact largest constant encodes");
        let key_saturated = region_probability_product_key(RegionProductKeyInput {
            binding: None,
            discrete: json!("model"),
            unit: json!("unit"),
            scale: 1,
            subdivisions: 3,
            clocks: vec![RegionClockKey {
                identity: json!(["type", "clock", "receiver"]),
                value: 7,
                cap: 7,
            }],
            deadline: None,
            tail: DigitalTail::Running,
            monitor: None,
            accumulator: None,
            pending: None,
            kind: RegionKeyKind::Region,
            corner: None,
        })
        .expect("the saturated clock encodes");

        assert_ne!(key_at_constant, key_saturated);
    }

    #[trace("TC-636")]
    #[test]
    fn the_claim_fixes_the_scheduler_objective() {
        let half = || Rational::new(Integer::from(1_i64), Integer::from(2_i64)).unwrap();
        let every = ProbScheduler::Every;
        let workload = ProbScheduler::Workload("W".to_owned());
        assert_eq!(
            claim_objective(&every, &Bound::AtLeast(half())),
            ProbabilityObjective::Minimum
        );
        assert_eq!(
            claim_objective(&every, &Bound::AtMost(half())),
            ProbabilityObjective::Maximum
        );
        for bound in [Bound::AtLeast(half()), Bound::AtMost(half())] {
            assert_eq!(
                claim_objective(&workload, &bound),
                ProbabilityObjective::Workload
            );
        }
    }

    #[trace("TC-636")]
    #[test]
    fn bounded_model_product_key_uses_the_complete_fr196_v1_tuple() {
        let model = include_bytes!(
            "../../tools/fsl-probability-check/tests/fixtures/link.model-key-0.json"
        );
        let key = product_key(None, model).expect("the production model key encodes");
        let actual: serde_json::Value =
            serde_json::from_slice(key.as_bytes()).expect("the key is JSON");
        assert_eq!(
            actual,
            serde_json::json!([
                "filament.probability-product-key/v1",
                null,
                null,
                null,
                [
                    "filament.simulation.state-key/v1",
                    serde_json::from_slice::<Value>(model).unwrap()
                ],
                null,
                null,
                null
            ])
        );
    }

    #[trace("TC-636")]
    #[test]
    fn exact_certificate_checks_each_state_and_reports_first_bad_key() {
        let graph = graph();
        let result = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(3_i64), Integer::from(4_i64)).unwrap()),
            GraphCycles::Reject,
            &Cancel::new(),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            result,
            ExactProbabilityCertificateCheck::Accepted(ExactProbabilityAcceptance {
                initial_values: None,
                document: None,
                value: Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap(),
                side: ProbabilityBoundSide::AtLeast,
                certificate: certificate(
                    Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()
                ),
            })
        );

        let rejected_result = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(9_i64), Integer::from(10_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(3_i64), Integer::from(4_i64)).unwrap()),
            GraphCycles::Reject,
            &Cancel::new(),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            rejected_result,
            rejected(ProbabilityCertificateRule::InequalityFails, key(0))
        );
    }

    #[trace("TC-636")]
    #[test]
    fn exact_certificate_rejects_threshold_miss_and_cycles() {
        let graph = graph();
        let missed = check_exact_reachability(
            &graph,
            &certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()),
            ProbabilityObjective::Minimum,
            &Bound::AtLeast(Rational::new(Integer::from(9_i64), Integer::from(10_i64)).unwrap()),
            GraphCycles::Reject,
            &Cancel::new(),
        )
        .expect("the checker graph is acyclic and closed");
        assert_eq!(
            missed,
            rejected(ProbabilityCertificateRule::ThresholdMissed, key(0))
        );

        let mut cyclic = graph;
        let state = cyclic.states.get_mut(&key(2)).unwrap();
        state.terminal = None;
        state.choices = vec![vec![edge(2, 1, 1)]];
        for objective in [
            ProbabilityObjective::Minimum,
            ProbabilityObjective::Maximum,
            ProbabilityObjective::Workload,
        ] {
            assert_eq!(
                check_exact_reachability(
                    &cyclic,
                    &certificate(
                        Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap()
                    ),
                    objective,
                    &Bound::AtLeast(rational(0)),
                    GraphCycles::Reject,
                    &Cancel::new(),
                ),
                Err(ExactProbabilityGraphError::Cycle)
            );
        }
    }

    #[trace("TC-636", "FR-201")]
    #[test]
    fn an_untimed_wait_loop_cannot_certify_a_scheduler_minimum() {
        // `wait` loops on the start state, `flip` reaches the target: a
        // scheduler that waits forever never reaches it, so the value 1 is
        // forged under either optimization.
        let initial = key(0);
        let target = key(1);
        let mut states = BTreeMap::new();
        states.insert(
            initial.clone(),
            CheckerState {
                terminal: None,
                choices: vec![vec![edge(0, 1, 1)], vec![edge(1, 1, 1)]],
            },
        );
        states.insert(
            target,
            CheckerState {
                terminal: Some(true),
                choices: Vec::new(),
            },
        );
        let graph = CheckerGraph { initial, states };
        let forged = ExactProbabilityCertificate {
            values: vec![
                ExactProbabilityValue {
                    state: key(0),
                    value: rational(1),
                },
                ExactProbabilityValue {
                    state: key(1),
                    value: rational(1),
                },
            ],
        };
        for objective in [ProbabilityObjective::Minimum, ProbabilityObjective::Maximum] {
            assert_eq!(
                check_exact_reachability(
                    &graph,
                    &forged,
                    objective,
                    &Bound::AtLeast(
                        Rational::new(Integer::from(9_i64), Integer::from(10_i64)).unwrap()
                    ),
                    GraphCycles::Reject,
                    &Cancel::new(),
                ),
                Err(ExactProbabilityGraphError::Cycle)
            );
        }
    }

    #[trace("TC-636", "FR-201", "FR-282")]
    #[test]
    fn cancellation_stops_every_observed_graph_validation_poll() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::{Arc, OnceLock};

        let graph = graph();
        let certificate =
            certificate(Rational::new(Integer::from(4_i64), Integer::from(5_i64)).unwrap());
        let bound = Bound::AtLeast(rational(0));
        let run = |cancel: &Cancel| {
            check_exact_reachability(
                &graph,
                &certificate,
                ProbabilityObjective::Minimum,
                &bound,
                GraphCycles::Reject,
                cancel,
            )
        };
        let live = Cancel::new();
        let _counting = live.count_charges();
        assert!(matches!(
            run(&live),
            Ok(ExactProbabilityCertificateCheck::Accepted(_))
        ));
        let total = live.charges();
        assert!(total > 0);
        for cause in [CancelCause::Requested, CancelCause::Deadline] {
            for at in 1..=total {
                let slot = Arc::new(OnceLock::<Cancel>::new());
                let weak = Arc::downgrade(&slot);
                let seen = Arc::new(AtomicU32::new(0));
                let count = Arc::clone(&seen);
                let cancel = Cancel::observing(move || {
                    if count.fetch_add(1, Ordering::Relaxed) + 1 == at {
                        weak.upgrade()
                            .expect("the run owns the slot")
                            .get()
                            .expect("the slot is initialized")
                            .cancel(cause);
                    }
                });
                slot.set(cancel.clone())
                    .expect("the slot is initialized once");
                assert_eq!(
                    run(&cancel),
                    Err(ExactProbabilityGraphError::Cancelled(cause)),
                    "validation poll {at}/{total} must stop instead of accepting"
                );
                assert_eq!(seen.load(Ordering::Relaxed), at);
            }
        }
    }

    #[trace("FR-196-AC-6")]
    #[test]
    fn digital_product_key_encodes_the_complete_timed_model_and_orders_clocks() {
        let key = digital_probability_product_key(DigitalProductKeyInput {
            binding: None,
            discrete: json!(["state", {"z": 1, "a": true}]),
            unit: json!(["unit", "second"]),
            scale: 10,
            clocks: vec![
                DigitalClockKey {
                    identity: json!(["Type", "zeta", ["u", "object-z"]]),
                    value: 1,
                    cap: 3,
                },
                DigitalClockKey {
                    identity: json!(["Type", "alpha", ["u", "object-a"]]),
                    value: 0,
                    cap: 3,
                },
            ],
            deadline: Some(DigitalDeadlineKey {
                limit: 4,
                elapsed: 1,
            }),
            tail: DigitalTail::IdleDelayed,
            monitor: None,
            accumulator: None,
            pending: None,
        })
        .expect("the complete timed key is representable");
        assert_eq!(
            key.as_bytes(),
            br#"["filament.probability-product-key/v1",null,null,null,["filament.simulation.state-key/v1",["digital",["state",{"a":true,"z":1}],["unit","second"],"10",[[["Type","alpha",["u","object-a"]],"0","3"],[["Type","zeta",["u","object-z"]],"1","3"]],["deadline","4","1"],"idle-delayed"]],null,null,null]"#
        );
    }

    #[trace("FR-196-AC-6")]
    #[test]
    fn digital_product_key_rejects_invalid_scale_clock_and_deadline_values() {
        let clock = || DigitalClockKey {
            identity: json!(["Type", "clock", ["u", "object"]]),
            value: 1,
            cap: 1,
        };
        let key = |scale, clocks, deadline| {
            digital_probability_product_key(DigitalProductKeyInput {
                binding: None,
                discrete: json!([]),
                unit: json!(["unit", "second"]),
                scale,
                clocks,
                deadline,
                tail: DigitalTail::Running,
                monitor: None,
                accumulator: None,
                pending: None,
            })
        };
        assert_eq!(
            key(0, vec![], None),
            Err(ExactProbabilityGraphError::InvalidDigitalKey)
        );
        assert_eq!(
            key(
                1,
                vec![DigitalClockKey {
                    value: 2,
                    ..clock()
                }],
                None
            ),
            Err(ExactProbabilityGraphError::InvalidDigitalKey)
        );
        assert_eq!(
            key(1, vec![clock(), clock()], None),
            Err(ExactProbabilityGraphError::InvalidDigitalKey)
        );
        assert_eq!(
            key(
                1,
                vec![clock()],
                Some(DigitalDeadlineKey {
                    limit: 1,
                    elapsed: 3,
                })
            ),
            Err(ExactProbabilityGraphError::InvalidDigitalKey)
        );
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-072 (ADR-013 O-27): the typed replay result and record carrier.
//!
//! The per-item result's `arm` is a sum of two distinct, independently
//! typed arm results -- [`WitnessArmResult`] and [`InputArmResult`] -- with
//! no `From`, `TryFrom`, `Into` or blanket conversion between them in
//! either direction (FR-072-AC-1). Each arm's only public constructor is
//! its own `settle`, which computes `Inconclusive` on any verdict
//! disagreement; no other public API on either type can turn a
//! disagreement into an agreement result (FR-072-AC-2, "never repaired").

use std::collections::BTreeMap;

use qsl_eval::value::StopReport;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::witness::{RuntimeValuePath, SeparationStep, ValuePathStep, ValuePathSubject};
use quire_exact::{compare_keys, Incomplete, ScalarLimits, Value};
use quire_semantic_value::location::{Location, Origin};

use crate::bounds::{BoundExceeded, ReplayLimits};

mod wire;
use crate::identity::TracePosition;
use qsl_foundation::diagnostic::Category;
use qsl_foundation::source::provenance::SourceRegion;
pub use wire::CauseCodecError;

/// The verdict a proved or replayed outcome settles to, taken from the
/// QSpec outcome-to-verdict map fixed per ADR-013 O-16 category (QC-8). A
/// newtype over the category, not a bare alias: FR-072 says the verdict
/// comes from "the QSpec outcome-to-verdict map fixed per O-16 category",
/// so [`Self::from_category`] is the one place that map lands, rather than
/// every `settle` call site treating plain category equality as agreement
/// by construction. Until that map lands, `from_category` is the identity
/// map, and this module needs only that two verdicts either agree or do
/// not, which the category's own equality already gives.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct Verdict(Category);

impl Verdict {
    /// The one place the QSpec outcome-to-verdict map (fixed per ADR-013
    /// O-16 category, QC-8) lands. Presently the identity map over the
    /// category -- the real map is QSpec's, not re-derived or guessed here
    /// -- so that when it lands, only this function's body changes.
    pub fn from_category(category: Category) -> Self {
        Self(category)
    }

    /// The category this verdict is presently defined as (identity map; see
    /// [`Self::from_category`]'s doc).
    pub fn category(self) -> Category {
        self.0
    }
}

/// Why a replay settled `inconclusive` (FR-072-AC-2): typed, never a
/// display string. Each case carries both verdicts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisagreementCause {
    /// The replay completed a value, and its verdict differs from the
    /// proved one.
    Verdicts {
        /// The verdict the original proving run reached.
        proved: Verdict,
        /// The verdict the replay run reached.
        replayed: Verdict,
    },
    /// The replay completed no value, so nothing decides it, whatever the
    /// two verdicts are.
    NoValue {
        /// The verdict the original proving run reached.
        proved: Verdict,
        /// The verdict the replay run reached.
        replayed: Verdict,
    },
    /// FR-269 (ADR-031 SW-12, SW-13): the verdicts agree but the
    /// separating witness does not: the payload's record differs from the
    /// re-derived one, is present on one side only, or fails the
    /// separation check.
    Witness {
        /// The verdict the original proving run reached.
        proved: Verdict,
        /// The verdict the replay run reached, equal to `proved`.
        replayed: Verdict,
        /// The payload's record, or its absence.
        given: Option<Box<SeparatingWitnessRecord>>,
        /// The re-derived record, or its absence.
        derived: Option<Box<SeparatingWitnessRecord>>,
        /// Why the witness does not agree.
        failure: WitnessFailure,
    },
}

impl DisagreementCause {
    /// The reason's wire spelling.
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Verdicts { .. } => "verdicts",
            Self::NoValue { .. } => "no-value",
            Self::Witness { .. } => "witness",
        }
    }

    /// The verdict the original proving run reached.
    pub fn proved(&self) -> Verdict {
        match self {
            Self::Verdicts { proved, .. }
            | Self::NoValue { proved, .. }
            | Self::Witness { proved, .. } => *proved,
        }
    }

    /// The verdict the replay run reached.
    pub fn replayed(&self) -> Verdict {
        match self {
            Self::Verdicts { replayed, .. }
            | Self::NoValue { replayed, .. }
            | Self::Witness { replayed, .. } => *replayed,
        }
    }

    /// The cause of settling `proved` against `replayed`, where `completed`
    /// tells whether the replay completed a value and `witness` is the
    /// witness comparison: `None` exactly when the settlement agrees.
    fn of(
        proved: Verdict,
        replayed: Verdict,
        completed: bool,
        witness: &WitnessCheck,
    ) -> Option<Self> {
        if !completed {
            Some(Self::NoValue { proved, replayed })
        } else if proved != replayed {
            Some(Self::Verdicts { proved, replayed })
        } else {
            match witness {
                WitnessCheck::Agrees(_) => None,
                WitnessCheck::Disagrees {
                    given,
                    derived,
                    failure,
                } => Some(Self::Witness {
                    proved,
                    replayed,
                    given: given.clone(),
                    derived: derived.clone(),
                    failure: failure.clone(),
                }),
            }
        }
    }
}

impl DisagreementCause {
    /// The bytes of this cause's variable-length members: the nested
    /// witness records and the failure's strings, location and refusal
    /// fields. Zero for a verdict-only cause.
    pub(crate) fn measured_bytes(&self) -> usize {
        match self {
            Self::Verdicts { .. } | Self::NoValue { .. } => 0,
            Self::Witness {
                given,
                derived,
                failure,
                ..
            } => {
                given
                    .iter()
                    .chain(derived)
                    .map(|record| record.measured_bytes())
                    .sum::<usize>()
                    + failure.measured_bytes()
            }
        }
    }
}

/// FR-269: why a separating witness does not agree.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WitnessFailure {
    /// The payload's record differs from the re-derived one, or only one
    /// is present.
    Mismatch,
    /// The separation check failed at `step`, for `reason`.
    Separation {
        /// The failing FR-268 step.
        step: SeparationStep,
        /// Why it failed.
        reason: SeparationReason,
    },
}

/// The bytes of an expression origin's declared name; the index members
/// are fixed-width and not counted.
fn origin_bytes(origin: &Origin) -> usize {
    match origin {
        Origin::Body { function: name, .. }
        | Origin::Measure { function: name, .. }
        | Origin::TypeDeclaration { name }
        | Origin::StateClause { clause: name, .. } => name.len(),
        Origin::Expression | Origin::ProtocolAttempt { .. } => 0,
    }
}

impl WitnessFailure {
    /// The bytes of this failure's variable-length members.
    fn measured_bytes(&self) -> usize {
        match self {
            Self::Mismatch => 0,
            Self::Separation { reason, .. } => match reason {
                SeparationReason::Unmet => 0,
                SeparationReason::UndefinedEvaluation { expression, cause } => {
                    origin_bytes(&expression.origin)
                        + expression.depth() * std::mem::size_of::<usize>()
                        + cause.len()
                }
                SeparationReason::Refused(refusal) => {
                    refusal.code.len()
                        + refusal.cause.len()
                        + refusal
                            .fields
                            .iter()
                            .map(|(name, value)| name.len() + value.len())
                            .sum::<usize>()
                }
            },
        }
    }
}

/// FR-269: why a separation-check step failed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SeparationReason {
    /// The step's requirement does not hold.
    Unmet,
    /// The step's evaluation ended `undefined`: the expression where it
    /// arose and the undefined reason's tabled spelling (FR-100).
    UndefinedEvaluation {
        /// The expression the undefined result arose at.
        expression: Location,
        /// The undefined reason, as `quire.native.diagnostics/v1` spells
        /// it.
        cause: String,
    },
    /// The step's evaluation was refused, with its refusal record.
    Refused(SeparationRefusal),
}

/// FR-269: a separation-check evaluation's refusal record (ADR-013 O-17):
/// its catalog code and cause and its catalog fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SeparationRefusal {
    /// The catalog code's spelling.
    pub code: String,
    /// The catalog cause.
    pub cause: String,
    /// The catalog fields, by payload name.
    pub fields: BTreeMap<String, String>,
}

/// FR-268: how a replay's separating witness compares, given to
/// [`WitnessArmResult::settle`] and [`InputArmResult::settle`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum WitnessCheck {
    /// The witness agrees: the record the result carries on agreement
    /// (present exactly when the settlement basis is decisive; always
    /// absent for a function or frame replay, which has no decisive
    /// occurrence, ADR-031 SW-7).
    Agrees(Option<Box<SeparatingWitnessRecord>>),
    /// The witness disagrees (FR-269).
    Disagrees {
        /// The payload's record, or its absence.
        given: Option<Box<SeparatingWitnessRecord>>,
        /// The re-derived record, or its absence.
        derived: Option<Box<SeparatingWitnessRecord>>,
        /// Why.
        failure: WitnessFailure,
    },
}

/// The `Witness`-arm settlement (ADR-013 O-27, AD-016 WP9).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessSettlement {
    /// Agreement, with backend evidence: the only settlement CG's sealed
    /// backend-evidence-verdict type (AD-016, CG-owned) ever admits.
    ReproducedWithEvaluatedWitness,
    /// Disagreement; never repaired.
    Inconclusive,
}

/// The `Input`-arm settlement. A structurally distinct enum from
/// [`WitnessSettlement`] -- not the same type reused with a different
/// variant -- part of what keeps [`InputArmResult`] and [`WitnessArmResult`]
/// unconvertible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputSettlement {
    /// Agreement, but never backend evidence (a corpus counterexample
    /// replayed with no witness).
    ReproducedWithoutWitness,
    /// Disagreement; never repaired.
    Inconclusive,
}

/// The evaluated value a replay produced. A minimal stand-in for the
/// kernel `Value` (ADR-013 O-13), which has no structural equality to derive
/// this envelope's from: none of FR-072's acceptance criteria turn on this
/// value's own shape, only on arm distinctness, settlement and the nested
/// FR-351 record's typed fields. It holds the one scalar kind a replay
/// reads today: the Boolean a replayed predicate returns (FR-098).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvaluatedValue {
    /// A Boolean value.
    Boolean(bool),
}

/// QSpec FR-351's separating witness record (ADR-031 SW-5, SW-7): the
/// deciding quantifier, the deciding element, its index, its value path and
/// its trace position. Compared componentwise, the deciding element under
/// ADR-013 O-13 semantic equality.
#[derive(Clone, Debug)]
pub struct SeparatingWitnessRecord {
    /// The decisive quantifier's occurrence key (ADR-013 O-07).
    pub quantifier: OccurrenceKey,
    /// The kernel value bound to the quantifier's binder at the stop.
    pub deciding_element: Value,
    /// The element's zero-based position in its stored or built
    /// collection; absent for a family that assigns no position.
    pub index: Option<u64>,
    /// The QSpec FR-207 runtime value path of the element's location.
    pub value_path: RuntimeValuePath,
    /// The trace position; absent for a family that keeps no trace (a
    /// state clause reads one observation).
    pub trace_position: Option<TracePosition>,
}

impl PartialEq for SeparatingWitnessRecord {
    fn eq(&self, other: &Self) -> bool {
        self.quantifier == other.quantifier
            && same_element(&self.deciding_element, &other.deciding_element)
            && self.index == other.index
            && self.value_path == other.value_path
            && self.trace_position == other.trace_position
    }
}

impl Eq for SeparatingWitnessRecord {}

/// Two deciding elements are the same under ADR-013 O-13 semantic
/// equality. A value holding a float has no canonical key (O-13 excludes
/// floats from `=`), so two such values are the same exactly when they
/// encode identically, bit pattern for bit pattern.
pub(crate) fn same_element(left: &Value, right: &Value) -> bool {
    match compare_keys(left, right) {
        Some(order) => order == std::cmp::Ordering::Equal,
        None => wire::same_encoding(left, right),
    }
}

impl SeparatingWitnessRecord {
    /// FR-268's state-clause record: FR-265's stop report, with no trace
    /// position.
    pub(crate) fn from_stop(report: &StopReport) -> Self {
        Self {
            quantifier: report.quantifier.clone(),
            deciding_element: report.element.clone(),
            index: Some(report.index),
            value_path: report.value_path.clone(),
            trace_position: None,
        }
    }

    /// This record's own measured encoded size (FR-070-AC-7, FR-072-AC-5,
    /// ADR-031 SW-14): every variable-length member's byte length and a
    /// fixed width for each fixed-width one.
    pub fn measured_bytes(&self) -> usize {
        let observation = &self.value_path.observation;
        let subject = match &self.value_path.subject {
            ValuePathSubject::Object(object) => 64 + object.object().as_str().len(),
            ValuePathSubject::Built(occurrence) => {
                32 + occurrence.origin().role().as_str().len() + 8
            }
        };
        let steps: usize = self
            .value_path
            .steps
            .iter()
            .map(|step| match step {
                ValuePathStep::Field(name) | ValuePathStep::Member(name) => name.len(),
                ValuePathStep::Index(_) => 8,
                ValuePathStep::OptionValue => 1,
            })
            .sum();
        32 + self.quantifier.origin().role().as_str().len()
            + 8
            + value_bytes(&self.deciding_element)
            + 8
            + observation.authority.len()
            + observation.identity.len()
            + observation.revision_namespace.len()
            + observation.revision.len()
            + subject
            + steps
            + self
                .trace_position
                .as_ref()
                .map_or(0, |position| position.as_str().len())
    }
}

/// A kernel value's measured encoded size, summed over every nested
/// occurrence: each variable-length member (integer digits, text, object
/// identity) by its byte length, each digest by its 32 bytes and each other
/// fixed-width member by its width.
pub(crate) fn value_bytes(value: &Value) -> usize {
    let digits = |integer: &quire_exact::Integer| integer.to_string().len();
    let mut total = 0usize;
    let mut pending = vec![value];
    while let Some(value) = pending.pop() {
        total += match value {
            Value::Boolean(_) => 1,
            Value::Integer(integer) => digits(integer),
            Value::Rational(rational) => {
                digits(rational.numerator()) + digits(rational.denominator())
            }
            Value::Decimal(decimal) => digits(decimal.representation().coefficient()) + 4,
            Value::Float(_) => 8,
            Value::Quantity(quantity) => {
                let magnitude = quantity.magnitude();
                digits(magnitude.numerator()) + digits(magnitude.denominator()) + 32
            }
            // The payload, then the type's two bounds and its profile.
            Value::Text(text) => {
                text.payload().as_str().len() + 16 + text.text_type().profile().as_str().len()
            }
            Value::Enum(_) => 32 + 4,
            Value::Population(_) => 32,
            Value::Reference(reference) => 64 + reference.object().as_str().len(),
            Value::Collection(collection) => {
                pending.extend(collection.elements());
                8
            }
            Value::Option(option) => {
                pending.extend(option.payload());
                1
            }
            Value::Composite(composite) => {
                pending.extend(composite.slots().iter().filter_map(|slot| match slot {
                    quire_exact::FieldValue::Present(value) => Some(value),
                    quire_exact::FieldValue::Absent | quire_exact::FieldValue::Null => None,
                }));
                32
            }
        };
    }
    total
}

/// FR-072/ADR-013 O-27: the `Witness`-arm per-item result. No `From`,
/// `TryFrom` or `Into` exists between this type and [`InputArmResult`], in
/// either direction (FR-072-AC-1): a call site typed to accept one can
/// never be satisfied by the other.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WitnessArmResult {
    settlement: WitnessSettlement,
    disagreement: Option<DisagreementCause>,
    category: Category,
    value: Option<EvaluatedValue>,
    record: Option<Box<SeparatingWitnessRecord>>,
    resolved_regions: Vec<SourceRegion>,
    charges: ScalarLimits,
    limit: Option<Incomplete>,
}

impl WitnessArmResult {
    /// The only public constructor (FR-072-AC-2): settles
    /// `ReproducedWithEvaluatedWitness` only when the replay completed a
    /// value, `proved` and `replayed` agree and the witness agrees, with
    /// the agreeing record attached when there is one (ADR-031 SW-7: the
    /// record is its own `Option`, independent of the value); otherwise
    /// `Inconclusive` with no record and a typed [`DisagreementCause`]. No
    /// other public API on this type can turn a disagreement, or a replay
    /// that completed no value, into an agreement result (FR-098, FR-269).
    pub fn settle(
        proved: Verdict,
        replayed: Verdict,
        category: Category,
        value: Option<EvaluatedValue>,
        witness: WitnessCheck,
        resolved_regions: Vec<SourceRegion>,
        charges: ScalarLimits,
    ) -> Self {
        let disagreement = DisagreementCause::of(proved, replayed, value.is_some(), &witness);
        let record = match witness {
            WitnessCheck::Agrees(record) if disagreement.is_none() => record,
            WitnessCheck::Agrees(_) | WitnessCheck::Disagrees { .. } => None,
        };
        Self {
            settlement: if disagreement.is_none() {
                WitnessSettlement::ReproducedWithEvaluatedWitness
            } else {
                WitnessSettlement::Inconclusive
            },
            disagreement,
            category,
            value,
            record,
            resolved_regions,
            charges,
            limit: None,
        }
    }

    /// FR-122: settles a state-clause replay whose post snapshot holds an
    /// integer outside its declared range (FR-106 check 6.5). The range
    /// violation is itself the witness, so the arm settles
    /// `ReproducedWithEvaluatedWitness` with category `Violation`
    /// whatever the clause did over the exact value: `value` is the
    /// clause's own boolean when it completed one, `None` when it refused,
    /// ran out of budget or faulted. It carries no FR-351 record.
    pub fn settle_range_violation(
        value: Option<EvaluatedValue>,
        resolved_regions: Vec<SourceRegion>,
        charges: ScalarLimits,
    ) -> Self {
        Self {
            settlement: WitnessSettlement::ReproducedWithEvaluatedWitness,
            disagreement: None,
            category: Category::Violation,
            value,
            record: None,
            resolved_regions,
            charges,
            limit: None,
        }
    }

    /// The accounting limit that stopped the replay, when one did: the
    /// `Incomplete` outcome naming the counter, its configured value and the
    /// count reached (FR-098).
    pub fn limit(&self) -> Option<&Incomplete> {
        self.limit.as_ref()
    }

    /// This result carrying `limit` as the accounting limit that stopped
    /// the replay.
    #[must_use]
    pub fn with_limit(mut self, limit: Incomplete) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Whether this arm settled with evaluated-witness backend evidence or
    /// went inconclusive.
    pub fn settlement(&self) -> WitnessSettlement {
        self.settlement
    }
    /// The typed disagreement cause, present only when settlement went
    /// `Inconclusive`.
    pub fn disagreement(&self) -> Option<&DisagreementCause> {
        self.disagreement.as_ref()
    }
    /// The category this arm's settlement carries.
    pub fn category(&self) -> Category {
        self.category
    }
    /// The evaluated value this arm produced, or `None` when the replay
    /// completed no value.
    pub fn value(&self) -> Option<EvaluatedValue> {
        self.value
    }
    /// The nested FR-351 record, present only when the settlement basis is
    /// decisive (an agreement).
    pub fn record(&self) -> Option<&SeparatingWitnessRecord> {
        self.record.as_deref()
    }
    /// The resolved source regions this arm's result cites.
    pub fn resolved_regions(&self) -> &[SourceRegion] {
        &self.resolved_regions
    }
    /// The accounting charges this arm's replay run incurred.
    pub fn charges(&self) -> ScalarLimits {
        self.charges
    }
}

/// FR-072/ADR-013 O-27: the `Input`-arm per-item result. Structurally
/// distinct from [`WitnessArmResult`] -- it carries no nested FR-351 record
/// field at all, since an `Input`-arm agreement is never decisive backend
/// evidence -- reinforcing FR-072-AC-1's type-boundary property beyond just
/// "no conversion impl exists".
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputArmResult {
    settlement: InputSettlement,
    disagreement: Option<DisagreementCause>,
    category: Category,
    value: Option<EvaluatedValue>,
    resolved_regions: Vec<SourceRegion>,
    charges: ScalarLimits,
    limit: Option<Incomplete>,
}

impl InputArmResult {
    /// The only public constructor: settles `ReproducedWithoutWitness` only
    /// when the replay completed a value, `proved` and `replayed` agree and
    /// the witness agrees; otherwise `Inconclusive` with a typed
    /// [`DisagreementCause`]. Agreement here is never backend evidence, and
    /// this arm carries no record.
    pub fn settle(
        proved: Verdict,
        replayed: Verdict,
        category: Category,
        value: Option<EvaluatedValue>,
        witness: &WitnessCheck,
        resolved_regions: Vec<SourceRegion>,
        charges: ScalarLimits,
    ) -> Self {
        let disagreement = DisagreementCause::of(proved, replayed, value.is_some(), witness);
        Self {
            settlement: if disagreement.is_none() {
                InputSettlement::ReproducedWithoutWitness
            } else {
                InputSettlement::Inconclusive
            },
            disagreement,
            category,
            value,
            resolved_regions,
            charges,
            limit: None,
        }
    }

    /// FR-122: [`WitnessArmResult::settle_range_violation`]'s `Input`-arm
    /// counterpart: settles `ReproducedWithoutWitness` with category
    /// `Violation`.
    pub fn settle_range_violation(
        value: Option<EvaluatedValue>,
        resolved_regions: Vec<SourceRegion>,
        charges: ScalarLimits,
    ) -> Self {
        Self {
            settlement: InputSettlement::ReproducedWithoutWitness,
            disagreement: None,
            category: Category::Violation,
            value,
            resolved_regions,
            charges,
            limit: None,
        }
    }

    /// The accounting limit that stopped the replay, when one did: the
    /// `Incomplete` outcome naming the counter, its configured value and the
    /// count reached (FR-098).
    pub fn limit(&self) -> Option<&Incomplete> {
        self.limit.as_ref()
    }

    /// This result carrying `limit` as the accounting limit that stopped
    /// the replay.
    #[must_use]
    pub fn with_limit(mut self, limit: Incomplete) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Whether this arm settled without a witness or went inconclusive.
    pub fn settlement(&self) -> InputSettlement {
        self.settlement
    }
    /// The typed disagreement cause, present only when settlement went
    /// `Inconclusive`.
    pub fn disagreement(&self) -> Option<&DisagreementCause> {
        self.disagreement.as_ref()
    }
    /// The category this arm's settlement carries.
    pub fn category(&self) -> Category {
        self.category
    }
    /// The evaluated value this arm produced, or `None` when the replay
    /// completed no value.
    pub fn value(&self) -> Option<EvaluatedValue> {
        self.value
    }
    /// The resolved source regions this arm's result cites.
    pub fn resolved_regions(&self) -> &[SourceRegion] {
        &self.resolved_regions
    }
    /// The accounting charges this arm's replay run incurred.
    pub fn charges(&self) -> ScalarLimits {
        self.charges
    }
}

/// FR-072/ADR-013 O-27: one per-item replay result, a sum of the
/// `Witness`-arm and `Input`-arm result types. No shared, arm-agnostic
/// Boolean or flag represents agreement across both arms; a caller must
/// match on the arm to read a settlement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayResult {
    /// The `Witness`-arm result.
    Witness(WitnessArmResult),
    /// The `Input`-arm result.
    Input(InputArmResult),
}

impl ReplayResult {
    /// The accounting limit that stopped the replay, when one did (FR-098).
    pub fn limit(&self) -> Option<&Incomplete> {
        match self {
            Self::Witness(result) => result.limit(),
            Self::Input(result) => result.limit(),
        }
    }

    /// This result carrying `limit` as the accounting limit that stopped the
    /// replay.
    #[must_use]
    pub fn with_limit(self, limit: Incomplete) -> Self {
        match self {
            Self::Witness(result) => Self::Witness(result.with_limit(limit)),
            Self::Input(result) => Self::Input(result.with_limit(limit)),
        }
    }

    /// The category the result carries.
    pub fn category(&self) -> Category {
        match self {
            Self::Witness(result) => result.category(),
            Self::Input(result) => result.category(),
        }
    }

    /// The typed disagreement cause, present when the replay settled
    /// `inconclusive`.
    pub fn disagreement(&self) -> Option<&DisagreementCause> {
        match self {
            Self::Witness(result) => result.disagreement(),
            Self::Input(result) => result.disagreement(),
        }
    }

    /// The accounting charges the replay's call incurred.
    pub fn charges(&self) -> ScalarLimits {
        match self {
            Self::Witness(result) => result.charges(),
            Self::Input(result) => result.charges(),
        }
    }
}

/// `resolved_regions`' combined byte length -- shared by both arms of
/// [`measured_encoded_bytes`].
fn common_measured_bytes(resolved_regions: &[SourceRegion]) -> usize {
    resolved_regions
        .iter()
        .map(|region| {
            let source = region.source();
            source.authority().len() + source.identity().len()
        })
        .sum::<usize>()
}

/// [`read_bounded`]'s own measurement of `result`'s encoded size
/// (FR-072-AC-5): every variable-length member's own byte length -- the
/// nested FR-351 record's value-path segments among them -- never a
/// caller-declared number a result could understate to launder oversized
/// content past the bound (B3).
fn measured_encoded_bytes(result: &ReplayResult) -> usize {
    match result {
        ReplayResult::Witness(arm) => {
            common_measured_bytes(arm.resolved_regions())
                + arm
                    .record()
                    .map_or(0, SeparatingWitnessRecord::measured_bytes)
        }
        ReplayResult::Input(arm) => common_measured_bytes(arm.resolved_regions()),
    }
}

/// [`ReplayResult`]'s bound-checked reader (FR-072-AC-5): refuses an
/// oversized encoding rather than decoding a truncated result. B3: the
/// bound is measured from `result`'s own content -- there is no
/// caller-declared `encoded_bytes` a caller could understate to launder an
/// oversized value past the check.
pub fn read_bounded(
    result: ReplayResult,
    limits: ReplayLimits,
) -> Result<ReplayResult, BoundExceeded> {
    BoundExceeded::check(measured_encoded_bytes(&result), limits)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bounds::DEFAULT_INPUT_BYTES;
    use ix_trace_rs::trace;

    fn regions() -> Vec<SourceRegion> {
        use qsl_foundation::digest::{DigestDomain, DigestRecord};
        use qsl_foundation::source::provenance::RawSourceRef;
        let source = RawSourceRef::new(
            "registry",
            "pkg-a",
            DigestRecord::mint(DigestDomain::SourceBytesV1, [3; 32]),
        )
        .unwrap();
        vec![SourceRegion::new(source, 10, 20).unwrap()]
    }

    fn charges() -> ScalarLimits {
        ScalarLimits {
            integer_bits: 64,
            decimal_digits: 34,
            scale_expansion: 8,
            text_input_bytes: 1024,
            text_scalars: 1024,
            normalized_scalars: 1024,
            unit_edges: 4,
            value_occurrences: 16,
            work_units: 100,
            result_units: 10,
        }
    }

    fn record(value_path: Vec<&str>) -> SeparatingWitnessRecord {
        use qsl_eval::value::ObservationIdentity;
        use qsl_foundation::digest::WireNodeId;
        use quire_exact::{Origin, Role};
        SeparatingWitnessRecord {
            quantifier: OccurrenceKey::new(
                WireNodeId::from_digest([7; 32]),
                Origin::new(Role::new("expression"), 0),
            ),
            deciding_element: Value::Integer(quire_exact::Integer::from(600_i64)),
            index: Some(2),
            value_path: RuntimeValuePath {
                observation: ObservationIdentity {
                    authority: "test".to_owned(),
                    identity: "high".to_owned(),
                    revision_namespace: "ns".to_owned(),
                    revision: "1".to_owned(),
                },
                subject: ValuePathSubject::Built(OccurrenceKey::new(
                    WireNodeId::from_digest([8; 32]),
                    Origin::new(Role::new("expression"), 0),
                )),
                steps: value_path
                    .into_iter()
                    .map(|name| ValuePathStep::Member(name.to_owned()))
                    .collect(),
            },
            trace_position: Some(TracePosition::new("frame-0".to_owned())),
        }
    }

    fn witness_cause(
        given: Option<SeparatingWitnessRecord>,
        derived: Option<SeparatingWitnessRecord>,
        failure: WitnessFailure,
    ) -> DisagreementCause {
        DisagreementCause::Witness {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Violation),
            given: given.map(Box::new),
            derived: derived.map(Box::new),
            failure,
        }
    }

    /// FR-069-AC-4 (TC-178): a replay-parity cause measures every
    /// variable-length member it carries -- both nested witness records, and
    /// each failure's location, strings and refusal fields -- so none of it
    /// escapes the reader bound. Each expected total is summed from the
    /// parts independently of `measured_bytes`.
    #[trace("TC-178", "FR-069-AC-4")]
    #[test]
    fn a_disagreement_cause_measures_its_records_and_failure() {
        let one = record(vec!["a"]).measured_bytes();
        let two = record(vec!["a", "bb"]).measured_bytes();
        assert!(one > 0 && two > one);

        let verdicts = DisagreementCause::Verdicts {
            proved: Verdict::from_category(Category::Violation),
            replayed: Verdict::from_category(Category::Success),
        };
        assert_eq!(verdicts.measured_bytes(), 0);

        let mismatch = witness_cause(
            Some(record(vec!["a"])),
            Some(record(vec!["a", "bb"])),
            WitnessFailure::Mismatch,
        );
        assert_eq!(mismatch.measured_bytes(), one + two);

        let undefined = witness_cause(
            None,
            None,
            WitnessFailure::Separation {
                step: SeparationStep::Body,
                reason: SeparationReason::UndefinedEvaluation {
                    expression: Location::at(
                        Origin::Body {
                            function: "check".to_owned(),
                            index: 0,
                        },
                        &[1, 2, 3],
                    ),
                    cause: "division-by-zero".to_owned(),
                },
            },
        );
        assert_eq!(
            undefined.measured_bytes(),
            "check".len() + 3 * std::mem::size_of::<usize>() + "division-by-zero".len()
        );

        let refused = witness_cause(
            None,
            None,
            WitnessFailure::Separation {
                step: SeparationStep::Domain,
                reason: SeparationReason::Refused(SeparationRefusal {
                    code: "invalid_runtime_input".to_owned(),
                    cause: "invalid-value".to_owned(),
                    fields: BTreeMap::from([("path".to_owned(), "/x/y".to_owned())]),
                }),
            },
        );
        assert_eq!(
            refused.measured_bytes(),
            "invalid_runtime_input".len() + "invalid-value".len() + "path".len() + "/x/y".len()
        );
    }

    fn agrees(record: SeparatingWitnessRecord) -> WitnessCheck {
        WitnessCheck::Agrees(Some(Box::new(record)))
    }

    /// FR-072-AC-1 (TC-189): a `Witness`-arm agreement settles
    /// `ReproducedWithEvaluatedWitness`, an `Input`-arm agreement settles
    /// `ReproducedWithoutWitness`, the two are distinct values, and the two
    /// arm-result types are structurally distinct (no shared conversion; an
    /// `Input`-arm result's record field does not exist at all, so a call
    /// site expecting `WitnessArmResult::record()` cannot be satisfied by
    /// `InputArmResult`, which has no such method).
    /// A resolved region's measured bytes are its source's authority and
    /// identity lengths: `registry` 8 + `pkg-a` 5 = 13.
    #[test]
    fn common_measured_bytes_counts_each_source_member() {
        assert_eq!(common_measured_bytes(&regions()), 13);
    }

    #[trace("TC-189", "FR-072-AC-1")]
    #[test]
    fn tc_189_witness_and_input_arms_stay_distinct() {
        let witness_result = WitnessArmResult::settle(
            Verdict::from_category(Category::Success),
            Verdict::from_category(Category::Success),
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            agrees(record(vec!["field"])),
            regions(),
            charges(),
        );
        let input_result = InputArmResult::settle(
            Verdict::from_category(Category::Success),
            Verdict::from_category(Category::Success),
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            &WitnessCheck::Agrees(None),
            regions(),
            charges(),
        );

        assert_eq!(
            witness_result.settlement(),
            WitnessSettlement::ReproducedWithEvaluatedWitness
        );
        assert_eq!(
            input_result.settlement(),
            InputSettlement::ReproducedWithoutWitness
        );

        // Structurally distinct types: a function requiring a
        // `WitnessArmResult` cannot be called with `input_result` -- there
        // is no `From`/`TryFrom`/`Into` between them to attempt.
        fn requires_witness_arm(_: &WitnessArmResult) {}
        requires_witness_arm(&witness_result);
        // requires_witness_arm(&input_result); // would not compile: no conversion exists.
        let _ = &input_result;
    }

    /// FR-072-AC-2 (TC-190): a proved/replayed verdict disagreement settles
    /// `Inconclusive` with a typed [`DisagreementCause`]; the only public
    /// constructor is `settle`, which computes this from the two verdicts
    /// with no way to override it into an agreement.
    #[trace("TC-190", "FR-072-AC-2")]
    #[test]
    fn tc_190_disagreement_settles_inconclusive_and_is_never_repaired() {
        let disagreeing = WitnessArmResult::settle(
            Verdict::from_category(Category::Success),
            Verdict::from_category(Category::Refusal),
            Category::Refusal,
            Some(EvaluatedValue::Boolean(false)),
            agrees(record(vec!["field"])),
            regions(),
            charges(),
        );
        assert_eq!(disagreeing.settlement(), WitnessSettlement::Inconclusive);
        assert_eq!(
            disagreeing.disagreement(),
            Some(&DisagreementCause::Verdicts {
                proved: Verdict::from_category(Category::Success),
                replayed: Verdict::from_category(Category::Refusal),
            })
        );
        // A disagreement never carries a decisive record.
        assert!(disagreeing.record().is_none());

        let disagreeing_input = InputArmResult::settle(
            Verdict::from_category(Category::Success),
            Verdict::from_category(Category::Refusal),
            Category::Refusal,
            Some(EvaluatedValue::Boolean(false)),
            &WitnessCheck::Agrees(None),
            regions(),
            charges(),
        );
        assert_eq!(
            disagreeing_input.settlement(),
            InputSettlement::Inconclusive
        );
    }

    /// FR-072-AC-2, FR-098-AC-5 (TC-190): a replay that completed no value
    /// settles `Inconclusive` on either arm even when the two verdicts
    /// agree -- nothing decides it -- with a `NoValue` cause and no record.
    #[trace("TC-190", "FR-072-AC-2")]
    #[test]
    fn tc_190_a_replay_with_no_value_never_agrees() {
        let violation = Verdict::from_category(Category::Violation);
        let witness = WitnessArmResult::settle(
            violation,
            violation,
            Category::Violation,
            None,
            WitnessCheck::Agrees(None),
            regions(),
            charges(),
        );
        assert_eq!(witness.settlement(), WitnessSettlement::Inconclusive);
        assert_eq!(
            witness.disagreement(),
            Some(&DisagreementCause::NoValue {
                proved: violation,
                replayed: violation,
            })
        );
        assert!(witness.record().is_none());
        assert_eq!(witness.value(), None);

        let input = InputArmResult::settle(
            violation,
            violation,
            Category::Violation,
            None,
            &WitnessCheck::Agrees(None),
            regions(),
            charges(),
        );
        assert_eq!(input.settlement(), InputSettlement::Inconclusive);
        assert_eq!(
            input.disagreement().map(DisagreementCause::replayed),
            Some(violation)
        );
    }

    /// FR-072-AC-3 (TC-191): a decisive `Witness`-arm result's nested
    /// FR-351 record round-trips its four fields exactly -- construct via
    /// `settle`, "serialize" by reading the record's own typed fields back
    /// out, "read" by re-`settle`ing a *second, independent* result from
    /// those read-back fields (since #231 builds no separate
    /// `native-run-result/2` wire serializer -- that is #186's,
    /// FR-072-CON-1) -- and comparing two results reads only those typed
    /// fields: two results with the same rendered text but different value
    /// paths compare unequal. N2: this is a real construct/serialize/read
    /// round trip between two distinct `WitnessArmResult` values, not a
    /// same-object field read asserted against the fixture's own constants.
    #[trace("TC-191", "FR-072-AC-3", "FR-072-AC-5")]
    #[test]
    fn tc_191_round_trips_the_fr351_record_and_compares_structurally() {
        let first = WitnessArmResult::settle(
            Verdict::from_category(Category::Success),
            Verdict::from_category(Category::Success),
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            agrees(record(vec!["outer", "items", "member"])),
            regions(),
            charges(),
        );

        // "Serialize": read the record's own typed fields back out.
        let read_back_record = first.record().unwrap().clone();

        // "Read": re-`settle` an independent, second result from exactly
        // those read-back fields -- not from the fixture's own constants --
        // and confirm the record makes it across unchanged.
        let round_tripped = WitnessArmResult::settle(
            Verdict::from_category(Category::Success),
            Verdict::from_category(Category::Success),
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            agrees(read_back_record.clone()),
            regions(),
            charges(),
        );
        assert_eq!(round_tripped.record(), Some(&read_back_record));
        assert_eq!(round_tripped.record().unwrap().index, Some(2));
        assert_eq!(
            round_tripped.record().unwrap().value_path,
            record(vec!["outer", "items", "member"]).value_path
        );
        assert_eq!(
            round_tripped.record().unwrap().trace_position,
            Some(TracePosition::new("frame-0".to_owned()))
        );

        let second = WitnessArmResult::settle(
            Verdict::from_category(Category::Success),
            Verdict::from_category(Category::Success),
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            agrees(record(vec!["outer", "items", "other_member"])),
            regions(),
            charges(),
        );
        // Same rendered `{:?}` shape modulo the path text, but `PartialEq`
        // (derived, structural) says unequal because the value path
        // differs -- never computed by comparing `to_string()` output.
        assert_ne!(first, second);
        assert_ne!(
            first.record().unwrap().value_path,
            second.record().unwrap().value_path
        );

        // FR-072-AC-5: an oversized encoding refuses. B3: `read_bounded`
        // measures the result's own content, so the oversized case has to
        // actually carry oversized content -- a huge value-path segment.
        let huge_segment = "x".repeat(DEFAULT_INPUT_BYTES + 1);
        let oversized_result = WitnessArmResult::settle(
            Verdict::from_category(Category::Success),
            Verdict::from_category(Category::Success),
            Category::Success,
            Some(EvaluatedValue::Boolean(true)),
            agrees(record(vec![huge_segment.as_str()])),
            regions(),
            charges(),
        );
        let oversized = read_bounded(
            ReplayResult::Witness(oversized_result),
            crate::ReplayLimits::default(),
        );
        assert!(oversized.is_err());
    }

    /// FR-072-AC-4 (TC-192): a minimal function-application exemplar,
    /// standing in for #217's real integration, is fully representable
    /// with only the FR-069 through FR-072 types -- no new witness or
    /// replay type is defined for it. A second, structurally different
    /// function (different arity) reuses the identical types unchanged.
    ///
    /// N1: no `#[trace]` tag -- TC-192's literal claim ("no fifth type is
    /// defined for #217's exemplar") is a fact about #217's own, separate
    /// repository scope that nothing runnable in this repo can observe.
    #[test]
    fn tc_192_function_exemplar_reuses_the_four_types_with_none_new() {
        use crate::identity::QualifiedName;
        use crate::proof_result::{
            read_backend_provider_envelope, BackendProviderSource, TerminalRecord, TerminalValue,
        };
        use crate::witness::{
            origin, NoPayload, ReplaySource, Witness, WitnessEnvelope, WitnessPacket,
        };
        use qsl_foundation::digest::WireNodeId;
        use qsl_foundation::source::provenance::OccurrenceKey;
        use quire_exact::Identifier;

        // FR-069: a proof-result envelope for a `Counterexample` Kani run.
        let proof_source = BackendProviderSource {
            backend_identity: "kani-backend-1".to_owned(),
            items: vec![TerminalRecord::new(
                qsl_foundation::RequestIndex::new(0),
                TerminalValue::Refuted,
            )],
        };
        let proof_envelopes =
            read_backend_provider_envelope(&proof_source, crate::ReplayLimits::default()).unwrap();
        assert_eq!(proof_envelopes[0].category(), Category::Violation);

        // FR-070: a witness envelope decoding a function call's
        // counterexample, built for two structurally different functions
        // (one and two arguments) to confirm no per-arity type is needed.
        let build_witness_envelope =
            |function_name: &str, values: &str| -> WitnessEnvelope<NoPayload> {
                let witness =
                    Witness::parse(format!("<<<assertion|harness|check|{values}>>>")).unwrap();
                let packet = WitnessPacket::<NoPayload> {
                    obligation_identity: Some([2; 32]),
                    occurrence_key: Some(OccurrenceKey::new(
                        WireNodeId::from_digest([3; 32]),
                        origin("reference", 0),
                    )),
                    clause_node: Some(WireNodeId::from_digest([4; 32])),
                    selected_function: Some(
                        QualifiedName::new(vec![
                            Identifier::new("module").unwrap(),
                            Identifier::new(function_name).unwrap(),
                        ])
                        .unwrap(),
                    ),
                    package_id: Some((
                        Some(
                            qsl_foundation::digest::DigestDomain::PackageSemanticV2
                                .as_str()
                                .to_owned(),
                        ),
                        qsl_foundation::digest::DigestRecord::mint(
                            qsl_foundation::digest::DigestDomain::PackageSemanticV2,
                            [5; 32],
                        )
                        .hex(),
                    )),
                    source_digests: Some(vec![]),
                    profile_selections: Some(vec![]),
                    run_limits: Some(charges()),
                    declared_domains: Some(vec![]),
                    backend: Some("kani-backend-1".to_owned()),
                    trace_position: Some(None),
                    source: Some(ReplaySource::Witness(witness)),
                    family_payload: Some(NoPayload),
                };
                WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default()).unwrap()
            };
        let one_arg = build_witness_envelope("one_arg_fn", "x=1");
        let two_arg = build_witness_envelope("two_arg_fn", "x=1;y=2");
        assert_eq!(one_arg.selected_function().segments().len(), 2);
        assert_eq!(two_arg.selected_function().segments().len(), 2);

        // FR-072: the replay result, built with exactly this module's own
        // types -- no fifth type is defined anywhere in this test.
        let result = ReplayResult::Witness(WitnessArmResult::settle(
            Verdict::from_category(Category::Violation),
            Verdict::from_category(Category::Violation),
            Category::Violation,
            Some(EvaluatedValue::Boolean(true)),
            agrees(record(vec!["x"])),
            regions(),
            charges(),
        ));
        assert!(matches!(result, ReplayResult::Witness(_)));
    }
}

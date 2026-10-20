// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185, FR-186: the S3 check of the `Probabilistic` family's
//! declarations -- the `random`, `reward` and `workload` forms of FR-311 and
//! the `probabilistic` claim of FR-392.
//!
//! FR-185's three declarations are resolved, typed and recorded on the
//! checked graph ([`model`]): a random parameter's operation, parameter,
//! typed support and probabilities; a reward's operation and its integer or
//! quantity expression over the operation's parameters; a workload's model
//! and one positive weight per operation of it. Each refusal is the
//! `invalid_probabilistic_model` cause FR-185 names, a `missing_declaration`
//! or `ambiguous_declaration` for a name that resolves to no single
//! declaration, or the typer's own refusal.
//!
//! An exact value is measured as written and bounded before it is reduced
//! ([`exact`]), and a distribution's weight sum is bounded the same way.
//! Past the 65,536-bit ceiling either refuses with a limit refusal; FR-255
//! has no rational-bits setting yet, so it reports the S3 work budget's.
//!
//! A `probabilistic` claim is checked in two steps ([`claim`]): its form
//! (thresholds, confidence parameters, a quantile over a fraction), then, once
//! well formed, its resolution and typing -- subject, profile, scheduler,
//! rewards, predicates, units and event -- recorded as a
//! [`CheckedProbabilisticClaim`] with its obligation identity (FR-186). The
//! forms FR-195 and the timed checks own refuse
//! `unsupported_construct`/`not-yet-implemented`, so no claim compiles
//! unchecked.

mod claim;
mod exact;
mod model;

pub use claim::{
    Bound, BoundedMeasure, CheckedPredicate, CheckedProbabilisticClaim, CheckedTemporalFormula,
    Confidence, Event, FractionMeasure, ModelSubjectRef, OverBinding, ProbForm, ProbScheduler,
    ProbabilisticObligationIdentity,
};
pub(crate) use model::{check, precondition_reads, site_spans, ProbabilisticInput};
pub use model::{
    CheckedProbabilisticModel, PreconditionSite, ProbabilisticFeature, ProbabilisticOperation,
    RandomParameter, Reward, SupportValue, Workload,
};

use qsl_forms::{
    ConfidenceItemForm, ConfidenceKey, ExactValueForm, ExactValueKind, ExprNode, MeasureForm,
    ProbabilisticBody, ProbabilisticForm, RandomForm, RewardForm, WorkloadForm,
};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::Span;
use quire_exact::Integer;
use quire_semantic_value::location::{Location, Origin};

use super::layer::visit_located;
use super::refusal::{CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, StageLimitCause};

/// The largest scale a decimal is expanded at when a confidence range is
/// compared. A larger scale leaves the range unchecked here.
const MAX_EXPANDED_SCALE: u64 = 4096;

/// One probabilistic declaration of the unit, as S2 built it (FR-311,
/// FR-392).
#[derive(Clone, Debug)]
pub enum ProbabilisticDeclaration {
    /// A `random` declaration.
    Random(Box<RandomForm>),
    /// A `reward` declaration.
    Reward(Box<RewardForm>),
    /// A `workload` declaration.
    Workload(Box<WorkloadForm>),
    /// A `probabilistic` claim.
    Claim(Box<ProbabilisticForm>),
}

/// Whose weight is not a positive exact rational.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum WeightOwner {
    /// A value of a random parameter's distribution.
    Parameter {
        /// The operation, as written.
        operation: String,
        /// The parameter.
        parameter: String,
    },
    /// An operation's weight in a workload.
    Workload {
        /// The workload.
        workload: String,
        /// The operation, as written.
        operation: String,
    },
}

/// How a confidence clause breaks the all-or-none rule of QSpec FR-407.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ConfidenceFault {
    /// A parameter the clause leaves out.
    Omitted(&'static str),
    /// A parameter the clause states a second time.
    Repeated(&'static str),
}

/// Why the `Probabilistic` family's check refused a declaration (FR-185,
/// FR-186). Each variant but [`Self::QuantityLiteral`] carries its own span,
/// which `region::DeclarationRegions::refusal_region` reads directly.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ProbabilisticCause {
    /// A weight that is not a positive exact rational.
    /// `invalid_probabilistic_model`/`invalid-weight`.
    InvalidWeight {
        /// Whose weight it is.
        owner: WeightOwner,
        /// The weight's span.
        span: Span,
    },
    /// An exact value with a zero denominator, such as `rational(1, 0)`.
    /// `undefined_expression`/`unproved-nonzero`.
    ZeroDenominator {
        /// The value's span.
        span: Span,
    },
    /// A value listed twice in one distribution.
    /// `invalid_probabilistic_model`/`repeated-support-value`.
    RepeatedSupportValue {
        /// The operation, as written.
        operation: String,
        /// The parameter.
        parameter: String,
        /// The second value's span.
        span: Span,
    },
    /// A second `random` declaration of one parameter.
    /// `invalid_probabilistic_model`/`duplicate-distribution`.
    DuplicateDistribution {
        /// The operation, as written.
        operation: String,
        /// The parameter.
        parameter: String,
        /// The second declaration's span.
        span: Span,
    },
    /// An operation weighted twice in one workload.
    /// `invalid_probabilistic_model`/`duplicate-weight`.
    DuplicateWeight {
        /// The workload.
        workload: String,
        /// The operation, as written.
        operation: String,
        /// The second weight's span.
        span: Span,
    },
    /// A second reward of one name on one operation.
    /// `invalid_probabilistic_model`/`duplicate-reward`.
    DuplicateReward {
        /// The reward.
        reward: String,
        /// The operation, as written.
        operation: String,
        /// The second declaration's span.
        span: Span,
    },
    /// A `quantile` claim over a `fraction` measure, which has no quantile.
    /// `invalid_probabilistic_model`/`quantile-over-fraction`.
    QuantileOverFraction {
        /// The claim.
        claim: String,
        /// The measure's span.
        span: Span,
    },
    /// A reward that reads state or an ambient value.
    /// `invalid_probabilistic_model`/`state-read-in-reward`.
    StateReadInReward {
        /// The reward.
        reward: String,
        /// The operation, as written.
        operation: String,
        /// The read's span.
        span: Span,
    },
    /// A probability threshold, quantile level, mean threshold or long-run
    /// threshold of 0, 1 or outside `(0, 1)`.
    /// `invalid_probabilistic_model`/`qualitative-threshold`.
    QualitativeThreshold {
        /// The claim.
        claim: String,
        /// The threshold's span.
        span: Span,
    },
    /// A confidence clause that omits or repeats a parameter.
    /// `invalid_probabilistic_model`/`partial-confidence`.
    PartialConfidence {
        /// The claim.
        claim: String,
        /// What the clause got wrong.
        fault: ConfidenceFault,
        /// The offending item's span, or the clause's.
        span: Span,
    },
    /// Confidence parameters that leave no indifference region, or a
    /// parameter outside its range. `invalid_probabilistic_model`/
    /// `invalid-confidence`.
    InvalidConfidence {
        /// The claim.
        claim: String,
        /// The parameter that breaks its range.
        parameter: &'static str,
        /// The parameter's span.
        span: Span,
    },
    /// A `random` or `reward` operation name that names no operation of a
    /// selected model (a workload weight's is [`Self::UnknownOperation`]).
    /// `missing_declaration`/`missing-name`.
    UnresolvedOperation {
        /// The operation, as written.
        name: String,
        /// The name's span.
        span: Span,
    },
    /// An operation name that names operations of several models or
    /// several declarations none of which is more derived.
    /// `ambiguous_declaration`/`ambiguous-name`.
    AmbiguousOperation {
        /// The operation, as written.
        name: String,
        /// The name's span.
        span: Span,
    },
    /// A `random` declaration naming no parameter of its operation.
    /// `missing_declaration`/`missing-name`.
    UnresolvedParameter {
        /// The operation, as written.
        operation: String,
        /// The parameter, as written.
        parameter: String,
        /// The parameter's span.
        span: Span,
    },
    /// A workload on a name no `model` declaration binds.
    /// `missing_declaration`/`missing-name`.
    UnresolvedModel {
        /// The workload.
        workload: String,
        /// The model alias, as written.
        model: String,
        /// The alias's span.
        span: Span,
    },
    /// A support value that is not a literal of the parameter's type.
    /// `invalid_probabilistic_model`/`value-outside-type`.
    ValueOutsideType {
        /// The operation, as written.
        operation: String,
        /// The parameter.
        parameter: String,
        /// The value's span.
        span: Span,
    },
    /// A precondition of an operation that reads one of its random
    /// parameters, located at the read.
    /// `invalid_probabilistic_model`/`random-read-in-precondition`.
    RandomReadInPrecondition {
        /// The operation's name.
        operation: String,
        /// The random parameter read.
        parameter: String,
        /// The precondition that reads it.
        precondition: PreconditionSite,
    },
    /// A workload that gives an operation of its model no weight.
    /// `invalid_probabilistic_model`/`incomplete-workload`.
    IncompleteWorkload {
        /// The workload.
        workload: String,
        /// The omitted operation, `Type::op`.
        operation: String,
        /// The workload's span.
        span: Span,
    },
    /// A workload weight for an operation that is not in the workload's
    /// model. `invalid_probabilistic_model`/`unknown-operation`.
    UnknownOperation {
        /// The workload.
        workload: String,
        /// The operation, as written.
        operation: String,
        /// The operation's span.
        span: Span,
    },
    /// A reward whose value type is neither an integer nor a quantity type.
    /// `invalid_probabilistic_model`/`reward-type`.
    RewardType {
        /// The reward.
        reward: String,
        /// The operation, as written.
        operation: String,
        /// The expression's span.
        span: Span,
    },
    /// A second `workload` of a name already declared on any model of the
    /// package: a workload name is package-wide (FR-185, FR-186).
    /// `conflicting_declaration`/`duplicate-declaration` (QSpec FR-272).
    DuplicateWorkload {
        /// The workload name: the refusal's `subject`.
        workload: String,
        /// The second declaration's name span.
        span: Span,
        /// Both declarations' name spans, in source order: the refusal's
        /// `related`.
        related: [Span; 2],
    },
    /// A quantity literal in a function body or clause: it types, but the
    /// checked package has no literal form for a quantity to emit it as.
    /// `unsupported_construct`/`not-yet-implemented`.
    QuantityLiteral,
    /// A claim's `on` subject that names no `model` alias and declares no
    /// type. `missing_declaration`/`missing-name`.
    UnresolvedSubject {
        /// The claim.
        claim: String,
        /// The subject, as written.
        subject: String,
        /// The subject's span.
        span: Span,
    },
    /// A claim over a trace subject, which is outside the kind
    /// `probabilistic-satisfaction` (ADR-024 PF-9): observed statistics are
    /// window aggregates (FR-194). `invalid_probabilistic_model`/
    /// `trace-subject` (QSL-proposed, STD-188).
    TraceSubject {
        /// The claim.
        claim: String,
        /// The subject, as written.
        subject: String,
        /// The subject's span.
        span: Span,
    },
    /// A claim's `using` alias that names no profile of the unit.
    /// `missing_declaration`/`missing-name`.
    UnresolvedProfile {
        /// The claim.
        claim: String,
        /// The alias, as written.
        alias: String,
        /// The alias's span.
        span: Span,
    },
    /// A claim's profile that is not a temporal profile (FR-326's catalog).
    /// `invalid_probabilistic_model`/`not-temporal-profile` (QSL-proposed,
    /// STD-188).
    NotTemporalProfile {
        /// The claim.
        claim: String,
        /// The selected profile's identity.
        profile: String,
        /// The alias's span.
        span: Span,
    },
    /// An event with an operator that has no closed interval, or whose
    /// profile is not bounded: a probabilistic event needs a finite window
    /// (ADR-024 PF-8). `invalid_probabilistic_model`/`unbounded-event`
    /// (QSL-proposed, STD-188).
    UnboundedEvent {
        /// The claim.
        claim: String,
        /// The operator's, or the event's, span.
        span: Span,
    },
    /// An `under W` that names no workload of the unit.
    /// `missing_declaration`/`missing-name`.
    UnresolvedWorkload {
        /// The claim.
        claim: String,
        /// The workload, as written.
        workload: String,
        /// The name's span.
        span: Span,
    },
    /// An `under W` naming a workload declared on another model than the
    /// claim's subject (ADR-024 PM-8). `invalid_probabilistic_model`/
    /// `foreign-workload` (QSpec FR-407).
    ForeignWorkload {
        /// The claim.
        claim: String,
        /// The workload.
        workload: String,
        /// The `model` alias the workload is declared on.
        model: String,
        /// The workload name's span.
        span: Span,
    },
    /// A reward a claim names that no operation of its subject's model
    /// declares. `missing_declaration`/`missing-name`.
    UnresolvedReward {
        /// The claim.
        claim: String,
        /// The reward, as written.
        reward: String,
        /// The name's span.
        span: Span,
    },
    /// A reward a claim names that operations of its subject's model declare
    /// with different value types. `ambiguous_declaration`/`ambiguous-name`.
    AmbiguousReward {
        /// The claim.
        claim: String,
        /// The reward, as written.
        reward: String,
        /// The name's span.
        span: Span,
    },
    /// A threshold whose dimension is not the reward's. `ill_typed`/
    /// `unit-mismatch` (QSpec FR-407).
    UnitMismatch {
        /// The claim.
        claim: String,
        /// The threshold's dimension, or `dimensionless`.
        threshold: String,
        /// The reward's dimension, or `dimensionless`.
        reward: String,
        /// The threshold's span.
        span: Span,
    },
    /// A construct of a claim that S3 does not check yet: a form FR-195 owns
    /// (`expected ... until`, `elapsed`), a timed window or an `on each`
    /// activation. `unsupported_construct`/`not-yet-implemented`.
    NotYetImplemented {
        /// The claim's name.
        claim: String,
        /// The construct, as the diagnostic names it.
        construct: &'static str,
        /// The construct's span.
        span: Span,
    },
}

impl ProbabilisticCause {
    /// The refusal's own span, when it has one; a quantity literal and a
    /// precondition read are located through their `Location`.
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::InvalidWeight { span, .. }
            | Self::ZeroDenominator { span }
            | Self::RepeatedSupportValue { span, .. }
            | Self::DuplicateDistribution { span, .. }
            | Self::DuplicateWeight { span, .. }
            | Self::DuplicateReward { span, .. }
            | Self::QuantileOverFraction { span, .. }
            | Self::StateReadInReward { span, .. }
            | Self::QualitativeThreshold { span, .. }
            | Self::PartialConfidence { span, .. }
            | Self::InvalidConfidence { span, .. }
            | Self::UnresolvedOperation { span, .. }
            | Self::AmbiguousOperation { span, .. }
            | Self::UnresolvedParameter { span, .. }
            | Self::UnresolvedModel { span, .. }
            | Self::ValueOutsideType { span, .. }
            | Self::IncompleteWorkload { span, .. }
            | Self::UnknownOperation { span, .. }
            | Self::RewardType { span, .. }
            | Self::DuplicateWorkload { span, .. }
            | Self::UnresolvedSubject { span, .. }
            | Self::TraceSubject { span, .. }
            | Self::UnresolvedProfile { span, .. }
            | Self::NotTemporalProfile { span, .. }
            | Self::UnboundedEvent { span, .. }
            | Self::UnresolvedWorkload { span, .. }
            | Self::ForeignWorkload { span, .. }
            | Self::UnresolvedReward { span, .. }
            | Self::AmbiguousReward { span, .. }
            | Self::UnitMismatch { span, .. }
            | Self::NotYetImplemented { span, .. } => Some(*span),
            Self::QuantityLiteral | Self::RandomReadInPrecondition { .. } => None,
        }
    }

    /// The declarations this refusal relates, in source order: both
    /// workloads of a repeated name; none for every other cause.
    pub fn related(&self) -> &[Span] {
        if let Self::DuplicateWorkload { related, .. } = self {
            related
        } else {
            &[]
        }
    }

    /// This cause's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::InvalidWeight { .. }
            | Self::RepeatedSupportValue { .. }
            | Self::DuplicateDistribution { .. }
            | Self::DuplicateWeight { .. }
            | Self::DuplicateReward { .. }
            | Self::QuantileOverFraction { .. }
            | Self::StateReadInReward { .. }
            | Self::QualitativeThreshold { .. }
            | Self::PartialConfidence { .. }
            | Self::InvalidConfidence { .. }
            | Self::ValueOutsideType { .. }
            | Self::RandomReadInPrecondition { .. }
            | Self::IncompleteWorkload { .. }
            | Self::UnknownOperation { .. }
            | Self::RewardType { .. }
            | Self::TraceSubject { .. }
            | Self::NotTemporalProfile { .. }
            | Self::UnboundedEvent { .. }
            | Self::ForeignWorkload { .. } => Code::InvalidProbabilisticModel,
            Self::UnitMismatch { .. } => Code::IllTyped,
            Self::ZeroDenominator { .. } => Code::UndefinedExpression,
            Self::UnresolvedOperation { .. }
            | Self::UnresolvedParameter { .. }
            | Self::UnresolvedModel { .. }
            | Self::UnresolvedSubject { .. }
            | Self::UnresolvedProfile { .. }
            | Self::UnresolvedWorkload { .. }
            | Self::UnresolvedReward { .. } => Code::MissingDeclaration,
            Self::AmbiguousOperation { .. } | Self::AmbiguousReward { .. } => {
                Code::AmbiguousDeclaration
            }
            Self::DuplicateWorkload { .. } => Code::ConflictingDeclaration,
            Self::QuantityLiteral | Self::NotYetImplemented { .. } => Code::UnsupportedConstruct,
        }
    }

    /// This cause's catalog tag.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::InvalidWeight { .. } => "invalid-weight",
            Self::ZeroDenominator { .. } => "unproved-nonzero",
            Self::RepeatedSupportValue { .. } => "repeated-support-value",
            Self::DuplicateDistribution { .. } => "duplicate-distribution",
            Self::DuplicateWeight { .. } => "duplicate-weight",
            Self::DuplicateReward { .. } => "duplicate-reward",
            Self::QuantileOverFraction { .. } => "quantile-over-fraction",
            Self::StateReadInReward { .. } => "state-read-in-reward",
            Self::QualitativeThreshold { .. } => "qualitative-threshold",
            Self::PartialConfidence { .. } => "partial-confidence",
            Self::InvalidConfidence { .. } => "invalid-confidence",
            Self::ValueOutsideType { .. } => "value-outside-type",
            Self::RandomReadInPrecondition { .. } => "random-read-in-precondition",
            Self::IncompleteWorkload { .. } => "incomplete-workload",
            Self::UnknownOperation { .. } => "unknown-operation",
            Self::RewardType { .. } => "reward-type",
            Self::UnresolvedOperation { .. }
            | Self::UnresolvedParameter { .. }
            | Self::UnresolvedModel { .. }
            | Self::UnresolvedSubject { .. }
            | Self::UnresolvedProfile { .. }
            | Self::UnresolvedWorkload { .. }
            | Self::UnresolvedReward { .. } => "missing-name",
            Self::AmbiguousOperation { .. } | Self::AmbiguousReward { .. } => "ambiguous-name",
            Self::TraceSubject { .. } => "trace-subject",
            Self::NotTemporalProfile { .. } => "not-temporal-profile",
            Self::UnboundedEvent { .. } => "unbounded-event",
            Self::ForeignWorkload { .. } => "foreign-workload",
            Self::UnitMismatch { .. } => "unit-mismatch",
            Self::DuplicateWorkload { .. } => "duplicate-declaration",
            Self::QuantityLiteral | Self::NotYetImplemented { .. } => "not-yet-implemented",
        }
    }
}

/// The limit refusal of an exact value, written or derived, whose measure
/// `bits` is past [`exact::RATIONAL_BITS_CEILING`] (FR-185 "Exact values").
/// FR-255 has no rational-bits setting yet (QSL-883 shipped none), so the
/// refusal reports the S3 work budget's setting with the fixed ceiling as
/// its bound.
fn limit_refusal(bits: u64, location: Location) -> CheckRefusal {
    CheckRefusal {
        location,
        cause: CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::WorkBudget,
            limit: exact::RATIONAL_BITS_CEILING,
            actual: u128::from(bits),
            region: None,
        })),
    }
}

/// The limit refusal of an event's horizon, a sum of interval bounds `actual`
/// past the largest `u64` (FR-186 "Events and measures"; ADR-014 TR-4).
fn horizon_refusal(actual: u128) -> CheckRefusal {
    CheckRefusal {
        location: Location::root(Origin::Expression),
        cause: CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::WorkBudget,
            limit: u64::MAX,
            actual,
            region: None,
        })),
    }
}

/// The refusal an exact value at `span` and `location` is.
fn exact_refusal(fault: exact::ExactFault, span: Span, location: Location) -> CheckRefusal {
    match fault {
        exact::ExactFault::ZeroDenominator => refusal(ProbabilisticCause::ZeroDenominator { span }),
        exact::ExactFault::AboveCeiling { bits } => limit_refusal(bits, location),
    }
}

/// A quantity literal's magnitude, reduced, or the refusal it is at
/// `location` (FR-185 "Exact values").
pub(crate) fn quantity_magnitude(
    magnitude: &ExactValueForm,
    location: &Location,
) -> Result<quire_exact::Rational, CheckRefusal> {
    exact::value(magnitude).map_err(|fault| exact_refusal(fault, magnitude.span, location.clone()))
}

/// The refusal of a quantity literal S4 cannot emit, at `location`.
pub(crate) fn quantity_literal(location: Location) -> CheckRefusal {
    CheckRefusal {
        location,
        cause: CheckCause::Probabilistic(Box::new(ProbabilisticCause::QuantityLiteral)),
    }
}

fn refusal(cause: ProbabilisticCause) -> CheckRefusal {
    CheckRefusal {
        // A declaration has no expression tree for a `Location` path to
        // walk; `region::DeclarationRegions::refusal_region` reads the
        // cause's own span, so this `Location` is never resolved.
        location: Location::root(Origin::Expression),
        cause: CheckCause::Probabilistic(Box::new(cause)),
    }
}

/// Whether an exact value is a weight or threshold in `(0, 1)`, `[1, ∞)`, or
/// neither.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Magnitude {
    /// Zero or negative.
    NonPositive,
    /// Greater than 0 and less than 1.
    UnitInterval,
    /// 1 or more.
    AtLeastOne,
    /// A rational with denominator 0.
    ZeroDenominator,
}

/// Where `value` lies, without expanding a power of ten: `c / 10^s` is below
/// one exactly when `c` has at most `s` decimal digits.
fn magnitude(value: &ExactValueForm) -> Magnitude {
    let below_one = |digits: &Integer, scale: &Integer| match scale.to_u64() {
        Some(scale) => digits.decimal_digits() <= scale,
        None => true,
    };
    match value.kind {
        ExactValueKind::Natural => {
            if value.first.is_zero() || value.first.is_negative() {
                Magnitude::NonPositive
            } else {
                Magnitude::AtLeastOne
            }
        }
        ExactValueKind::DecimalLiteral | ExactValueKind::Decimal => {
            if value.first.is_zero() || value.first.is_negative() {
                Magnitude::NonPositive
            } else if below_one(&value.first, &value.second) {
                Magnitude::UnitInterval
            } else {
                Magnitude::AtLeastOne
            }
        }
        ExactValueKind::Rational => {
            let (numerator, denominator) = (&value.first, &value.second);
            if denominator.is_zero() {
                return Magnitude::ZeroDenominator;
            }
            let (numerator, denominator) = if denominator.is_negative() {
                (numerator.neg(), denominator.neg())
            } else {
                (numerator.clone(), denominator.clone())
            };
            if numerator.is_zero() || numerator.is_negative() {
                Magnitude::NonPositive
            } else if numerator < denominator {
                Magnitude::UnitInterval
            } else {
                Magnitude::AtLeastOne
            }
        }
    }
}

/// `value` as `numerator / denominator` with a positive denominator, or
/// `None` for a zero denominator or a scale beyond [`MAX_EXPANDED_SCALE`].
fn ratio(value: &ExactValueForm) -> Option<(Integer, Integer)> {
    match value.kind {
        ExactValueKind::Natural => Some((value.first.clone(), Integer::one())),
        ExactValueKind::DecimalLiteral | ExactValueKind::Decimal => {
            let scale = value.second.to_u64().filter(|s| *s <= MAX_EXPANDED_SCALE)?;
            Some((value.first.clone(), Integer::power_of_ten(scale)))
        }
        ExactValueKind::Rational => {
            let (numerator, denominator) = (&value.first, &value.second);
            if denominator.is_zero() {
                None
            } else if denominator.is_negative() {
                Some((numerator.neg(), denominator.neg()))
            } else {
                Some((numerator.clone(), denominator.clone()))
            }
        }
    }
}

/// A reward reads the operation's parameters and constants only: `self`, a
/// reference, a population, a dispatched call, a pre-state or `result` is a
/// read of state or of an ambient value.
fn reward(form: &RewardForm) -> Vec<ProbabilisticCause> {
    let mut causes = Vec::new();
    visit_located(
        &form.expression.expression,
        &form.expression.spans,
        form.span,
        |node, span| {
            let reads_state = matches!(
                node,
                ExprNode::SelfRef
                    | ExprNode::Result
                    | ExprNode::Deref(_)
                    | ExprNode::AllInstances { .. }
                    | ExprNode::Lookup { .. }
                    | ExprNode::Dispatch { .. }
                    | ExprNode::Reaches { .. }
                    | ExprNode::Pre(_)
                    | ExprNode::ProtocolReference { .. }
            );
            reads_state.then(|| {
                causes.push(ProbabilisticCause::StateReadInReward {
                    reward: form.name.name.clone(),
                    operation: form.operation.name.clone(),
                    span,
                });
            })
        },
    );
    causes
}

fn confidence_name(key: ConfidenceKey) -> &'static str {
    match key {
        ConfidenceKey::Alpha => "alpha",
        ConfidenceKey::Beta => "beta",
        ConfidenceKey::Indifference => "indifference",
    }
}

fn claim(form: &ProbabilisticForm) -> Vec<ProbabilisticCause> {
    let mut causes = Vec::new();
    let claim = &form.name.name;
    let qualitative =
        |value: &ExactValueForm, causes: &mut Vec<ProbabilisticCause>| match magnitude(value) {
            Magnitude::UnitInterval => true,
            Magnitude::ZeroDenominator => {
                causes.push(ProbabilisticCause::ZeroDenominator { span: value.span });
                false
            }
            Magnitude::NonPositive | Magnitude::AtLeastOne => {
                causes.push(ProbabilisticCause::QualitativeThreshold {
                    claim: claim.clone(),
                    span: value.span,
                });
                false
            }
        };
    // The probability, level or fraction the confidence parameters are
    // measured against; `expected` claims have none.
    let theta = match &form.body {
        ProbabilisticBody::Probability { threshold, .. } => Some(threshold),
        ProbabilisticBody::Quantile { level, .. } => Some(level),
        ProbabilisticBody::MeanOfFraction { threshold, .. }
        | ProbabilisticBody::LongRunFraction { threshold, .. } => Some(threshold),
        ProbabilisticBody::ExpectedUntil { .. } => None,
    };
    if let ProbabilisticBody::Quantile {
        measure: MeasureForm::Fraction(fraction),
        ..
    } = &form.body
    {
        causes.push(ProbabilisticCause::QuantileOverFraction {
            claim: claim.clone(),
            span: fraction.span,
        });
    }
    let theta = theta.filter(|theta| qualitative(theta, &mut causes));
    confidence(form, theta, &mut causes);
    causes
}

/// A claim states `alpha`, `beta` and `indifference` each once, or none; when
/// stated they satisfy `0 < α < 1`, `0 < β < 1`, `0 < ι`, `θ − ι > 0` and
/// `θ + ι < 1`.
fn confidence(
    form: &ProbabilisticForm,
    theta: Option<&ExactValueForm>,
    causes: &mut Vec<ProbabilisticCause>,
) {
    let Some(first) = form.confidence.first() else {
        return;
    };
    let claim = &form.name.name;
    let mut partial = Vec::new();
    let mut seen: Vec<&ConfidenceItemForm> = Vec::new();
    for item in &form.confidence {
        if seen.iter().any(|earlier| earlier.key == item.key) {
            partial.push(ProbabilisticCause::PartialConfidence {
                claim: claim.clone(),
                fault: ConfidenceFault::Repeated(confidence_name(item.key)),
                span: item.key_span,
            });
        }
        seen.push(item);
    }
    let clause = Span {
        start: first.key_span.start,
        end: form
            .confidence
            .last()
            .map_or(first.value.span.end, |last| last.value.span.end),
    };
    for key in [
        ConfidenceKey::Alpha,
        ConfidenceKey::Beta,
        ConfidenceKey::Indifference,
    ] {
        if !seen.iter().any(|item| item.key == key) {
            partial.push(ProbabilisticCause::PartialConfidence {
                claim: claim.clone(),
                fault: ConfidenceFault::Omitted(confidence_name(key)),
                span: clause,
            });
        }
    }
    if !partial.is_empty() {
        causes.extend(partial);
        return;
    }
    let value = |key: ConfidenceKey| seen.iter().find(|item| item.key == key);
    let (Some(alpha), Some(beta), Some(indifference)) = (
        value(ConfidenceKey::Alpha),
        value(ConfidenceKey::Beta),
        value(ConfidenceKey::Indifference),
    ) else {
        return;
    };
    let invalid = |item: &ConfidenceItemForm| ProbabilisticCause::InvalidConfidence {
        claim: claim.clone(),
        parameter: confidence_name(item.key),
        span: item.key_span,
    };
    for item in [alpha, beta] {
        if magnitude(&item.value) != Magnitude::UnitInterval {
            causes.push(invalid(item));
        }
    }
    if matches!(
        magnitude(&indifference.value),
        Magnitude::NonPositive | Magnitude::ZeroDenominator
    ) {
        causes.push(invalid(indifference));
        return;
    }
    let (Some(theta), Some((iota_n, iota_d))) = (theta.and_then(ratio), ratio(&indifference.value))
    else {
        return;
    };
    let (theta_n, theta_d) = theta;
    // θ − ι > 0 and θ + ι < 1, over the common denominator θd · ιd.
    let below = theta_n.mul(&iota_d).sub(&iota_n.mul(&theta_d));
    let above = theta_n.mul(&iota_d).add(&iota_n.mul(&theta_d));
    if below.is_zero() || below.is_negative() || above >= theta_d.mul(&iota_d) {
        causes.push(invalid(indifference));
    }
}

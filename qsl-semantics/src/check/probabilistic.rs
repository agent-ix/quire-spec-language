// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-185, FR-186: the S3 check of the `Probabilistic` family's
//! declarations -- the `random`, `reward` and `workload` forms of FR-311 and
//! the `probabilistic` claim of FR-392.
//!
//! This stage checks what QSpec FR-405 and FR-407 let it refuse from the
//! forms alone, and refuses everything else `unsupported_construct`/
//! `not-yet-implemented`, so nothing probabilistic compiles unchecked:
//!
//! - a random parameter's weights (`invalid-weight`), a repeated support
//!   value (`repeated-support-value`) and a second distribution of one
//!   parameter (`duplicate-distribution`);
//! - a workload's weights (`invalid-weight`) and a repeated operation
//!   (`duplicate-weight`);
//! - a reward that reads state or an ambient value (`state-read-in-reward`);
//! - a claim's probability threshold, quantile level, mean threshold and
//!   long-run threshold outside `(0, 1)` (`qualitative-threshold`), a
//!   confidence clause that omits or repeats a parameter
//!   (`partial-confidence`) or whose parameters leave no indifference region
//!   (`invalid-confidence`);
//! - a second `reward` of one name on one operation (`duplicate-reward`) and
//!   a `quantile` over a `fraction` measure (`quantile-over-fraction`).
//!
//! An exact value is read as written and never reduced: a magnitude is
//! decided from digit counts and a comparison of the written integers, so
//! `decimal(1, 4294967295)` and `rational(n, n)` cost nothing to read. The
//! dedicated rational-bits ceiling waits on QSL-883.
//!
//! Resolving a model, an operation, a parameter, a profile or a workload,
//! typing a support value, a reward or a threshold, reading preconditions,
//! the units of a quantity and the checked claim with its obligation
//! identity are not checked yet, so a declaration that passes the checks
//! above still refuses `not-yet-implemented`.

use qsl_forms::{
    ConfidenceItemForm, ConfidenceKey, ExactValueForm, ExactValueKind, ExprNode, Expression,
    MeasureForm, ProbabilisticBody, ProbabilisticForm, RandomForm, RewardForm, WorkloadForm,
};
use qsl_foundation::diagnostic::Code;
use qsl_foundation::Span;
use quire_exact::Integer;
use quire_semantic_value::location::{Location, Origin};

use super::layer::visit_located;
use super::refusal::{CheckCause, CheckRefusal};

/// The largest scale a decimal is expanded at when a confidence range is
/// compared. A larger scale leaves the range unchecked here; the claim still
/// refuses `not-yet-implemented`.
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
    /// A quantity literal in a position no checker reads yet.
    /// `unsupported_construct`/`not-yet-implemented`.
    QuantityLiteral,
    /// A declaration that passed every check above but that no resolution,
    /// typing or emission covers yet. `unsupported_construct`/
    /// `not-yet-implemented`.
    NotYetImplemented {
        /// The declaration's keyword.
        construct: &'static str,
        /// The declaration's name.
        name: String,
        /// The whole declaration's span.
        span: Span,
    },
}

impl ProbabilisticCause {
    /// The refusal's own span, when it has one; a quantity literal is located
    /// through its `Location`.
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
            | Self::NotYetImplemented { span, .. } => Some(*span),
            Self::QuantityLiteral => None,
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
            | Self::InvalidConfidence { .. } => Code::InvalidProbabilisticModel,
            Self::ZeroDenominator { .. } => Code::UndefinedExpression,
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
            Self::QuantityLiteral | Self::NotYetImplemented { .. } => "not-yet-implemented",
        }
    }
}

/// The refusal of a quantity literal in a position no checker reads yet
/// (FR-393 is checked by a later requirement).
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

/// Checks `declarations`: every refusal their forms give, and for a
/// declaration that gives none, one `not-yet-implemented` refusal.
pub(crate) fn check(declarations: &[ProbabilisticDeclaration]) -> Vec<CheckRefusal> {
    let mut refusals = Vec::new();
    let mut distributions: Vec<(&str, &str)> = Vec::new();
    let mut rewards: Vec<(&str, &str)> = Vec::new();
    for declaration in declarations {
        let (mut causes, construct, name, span) = match declaration {
            ProbabilisticDeclaration::Random(form) => {
                let key = (form.operation.name.as_str(), form.parameter.name.as_str());
                let mut causes = random(form);
                if distributions.contains(&key) {
                    causes.insert(
                        0,
                        ProbabilisticCause::DuplicateDistribution {
                            operation: key.0.to_owned(),
                            parameter: key.1.to_owned(),
                            span: form.span,
                        },
                    );
                }
                distributions.push(key);
                (causes, "random", format!("{}({})", key.0, key.1), form.span)
            }
            ProbabilisticDeclaration::Reward(form) => {
                let key = (form.operation.name.as_str(), form.name.name.as_str());
                let mut causes = Vec::new();
                if rewards.contains(&key) {
                    causes.push(ProbabilisticCause::DuplicateReward {
                        reward: key.1.to_owned(),
                        operation: key.0.to_owned(),
                        span: form.span,
                    });
                }
                rewards.push(key);
                causes.extend(reward(form));
                (causes, "reward", form.name.name.clone(), form.span)
            }
            ProbabilisticDeclaration::Workload(form) => (
                workload(form),
                "workload",
                form.name.name.clone(),
                form.span,
            ),
            ProbabilisticDeclaration::Claim(form) => (
                claim(form),
                "probabilistic",
                form.name.name.clone(),
                form.span,
            ),
        };
        if causes.is_empty() {
            causes.push(ProbabilisticCause::NotYetImplemented {
                construct,
                name,
                span,
            });
        }
        refusals.extend(causes.into_iter().map(refusal));
    }
    refusals
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

/// A weight: a positive exact rational.
fn weight(
    value: &ExactValueForm,
    owner: impl FnOnce() -> WeightOwner,
) -> Option<ProbabilisticCause> {
    match magnitude(value) {
        Magnitude::UnitInterval | Magnitude::AtLeastOne => None,
        Magnitude::NonPositive => Some(ProbabilisticCause::InvalidWeight {
            owner: owner(),
            span: value.span,
        }),
        Magnitude::ZeroDenominator => {
            Some(ProbabilisticCause::ZeroDenominator { span: value.span })
        }
    }
}

fn random(form: &RandomForm) -> Vec<ProbabilisticCause> {
    let mut causes = Vec::new();
    let operation = &form.operation.name;
    let parameter = &form.parameter.name;
    for (index, pair) in form.support.iter().enumerate() {
        causes.extend(weight(&pair.weight, || WeightOwner::Parameter {
            operation: operation.clone(),
            parameter: parameter.clone(),
        }));
        let repeated = form.support[..index]
            .iter()
            .any(|earlier| same_literal(&earlier.value.expression, &pair.value.expression));
        if repeated {
            causes.push(ProbabilisticCause::RepeatedSupportValue {
                operation: operation.clone(),
                parameter: parameter.clone(),
                span: pair.value.spans.at(&[]).unwrap_or(pair.span),
            });
        }
    }
    causes
}

/// Whether two support values are the same literal, ignoring where they are
/// written. Only a leaf literal is compared: any other pair of expressions
/// is taken as different.
fn same_literal(left: &Expression, right: &Expression) -> bool {
    match (left.root().node(), right.root().node()) {
        (ExprNode::Quantity(left), ExprNode::Quantity(right)) => {
            left.magnitude.kind == right.magnitude.kind
                && left.magnitude.first == right.magnitude.first
                && left.magnitude.second == right.magnitude.second
                && left.unit.name == right.unit.name
        }
        (ExprNode::Boolean(left), ExprNode::Boolean(right)) => left == right,
        (ExprNode::Integer(left), ExprNode::Integer(right)) => left == right,
        (ExprNode::Rational(left_n, left_d), ExprNode::Rational(right_n, right_d)) => {
            left_n == right_n && left_d == right_d
        }
        (ExprNode::String(left), ExprNode::String(right)) => left == right,
        (ExprNode::Name(left), ExprNode::Name(right)) => left == right,
        _ => false,
    }
}

fn workload(form: &WorkloadForm) -> Vec<ProbabilisticCause> {
    let mut causes = Vec::new();
    for (index, weight_form) in form.weights.iter().enumerate() {
        let operation = &weight_form.operation.name;
        causes.extend(weight(&weight_form.weight, || WeightOwner::Workload {
            workload: form.name.name.clone(),
            operation: operation.clone(),
        }));
        if form.weights[..index]
            .iter()
            .any(|earlier| earlier.operation.name == *operation)
        {
            causes.push(ProbabilisticCause::DuplicateWeight {
                workload: form.name.name.clone(),
                operation: operation.clone(),
                span: weight_form.span,
            });
        }
    }
    causes
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

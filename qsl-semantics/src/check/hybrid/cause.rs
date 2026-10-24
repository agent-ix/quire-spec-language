// SPDX-License-Identifier: AGPL-3.0-or-later
//! Why S3 refused a `hybrid` declaration (FR-447), each cause located at the
//! span it names: a declaration's expressions are not the expression tree of
//! a function a `Location` path resolves in.

use qsl_foundation::diagnostic::Code;
use qsl_foundation::Span;

use crate::check::profile::TemporalProfile;
use crate::check::refusal::type_fault_class;
use crate::check::refusal::TimeUnitCause;
use crate::check::type_form::TypeFormFault;

/// What made a hybrid declaration malformed (`invalid_model_binding` /
/// `malformed-declaration`).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MalformedReason {
    /// An equation's left side is a qualified name with a `::` segment.
    QualifiedEquationName,
    /// A reset is neither `true` nor a conjunction of `x ~ e` conjuncts.
    ResetNotConjunction,
    /// A reset conjunct's left operand is not a continuous variable.
    ResetConjunctNotVariable,
    /// `pre(...)` outside a reset, or over something other than a continuous
    /// variable.
    PreMisplaced,
    /// A declared name is not identifier-shaped.
    InvalidName,
}

/// Why the S3 check refused a `hybrid` declaration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum HybridCause {
    /// A second hybrid declaration of one name:
    /// `conflicting_declaration` / `duplicate-declaration`, at the second.
    DuplicateHybrid {
        /// The name.
        name: String,
        /// The first declaration's name span.
        first: Span,
        /// The second declaration's name span.
        span: Span,
    },
    /// A second mode of one name: `conflicting_declaration` /
    /// `duplicate-declaration`, at the second mode.
    DuplicateMode {
        /// The mode name.
        name: String,
        /// The first mode's name span.
        first: Span,
        /// The second mode's name span.
        span: Span,
    },
    /// A transition target or initial entry names no mode of the
    /// declaration: `missing_declaration` / `missing-name`.
    MissingMode {
        /// The name as written.
        name: String,
        /// The name's span.
        span: Span,
    },
    /// A bare name that is no continuous variable and no `let` binder in
    /// scope: `missing_declaration` / `missing-name`.
    MissingName {
        /// The name as written.
        name: String,
        /// The name's span.
        span: Span,
    },
    /// A second equation for one variable in a mode:
    /// `conflicting_declaration` / `duplicate-declaration`.
    DuplicateEquation {
        /// The variable.
        variable: String,
        /// The first equation's left side.
        first: Span,
        /// The second equation's left side.
        span: Span,
    },
    /// A mode with no equation for a continuous variable:
    /// `invalid_model_binding` / `malformed-declaration`, at the mode.
    MissingEquation {
        /// The mode.
        mode: String,
        /// The variable with no equation.
        variable: String,
        /// The mode's name span.
        span: Span,
    },
    /// A variable with an `=` reset conjunct and a second conjunct:
    /// `conflicting_declaration` / `duplicate-declaration`, at the second.
    DuplicateReset {
        /// The variable.
        variable: String,
        /// The earlier conjunct.
        first: Span,
        /// The later conjunct.
        span: Span,
    },
    /// `invalid_model_binding` / `malformed-declaration`.
    Malformed {
        /// What is malformed.
        reason: MalformedReason,
        /// The span of the malformed form.
        span: Span,
    },
    /// An invariant, guard or initial condition that is not Boolean:
    /// `ill_typed` / `non-boolean-root`.
    NonBoolean {
        /// The expression's span.
        span: Span,
    },
    /// A form whose operand or result is neither real nor Boolean, or the
    /// wrong one of the two: `ill_typed` / `type-mismatch`.
    TypeMismatch {
        /// The form's span.
        span: Span,
    },
    /// `rational(n, 0)`: `undefined_expression` / `unproved-nonzero`.
    ZeroDenominator {
        /// The literal's span.
        span: Span,
    },
    /// A call of a function whose body holds a form the hybrid expression
    /// forms do not admit: `ill_typed` / `type-mismatch`, at the call,
    /// naming the form.
    CalleeForm {
        /// The foreign form.
        form: &'static str,
        /// The call's span.
        span: Span,
    },
    /// A second capture of one name: `conflicting_declaration` /
    /// `duplicate-declaration`, naming both.
    DuplicateCapture {
        /// The capture name.
        name: String,
        /// The first capture's name span.
        first: Span,
        /// The second capture's name span.
        span: Span,
    },
    /// A claim's `using` alias selects no temporal profile:
    /// `unknown_profile` / `unsupported-selection`.
    NotTemporalProfile {
        /// The alias as written.
        alias: String,
        /// The alias's span.
        span: Span,
    },
    /// A clock other than `model-time` and `model-steps`:
    /// `unsupported_construct` / `declaration-form`, at the clock string.
    UnsupportedClock {
        /// The clock string's span.
        span: Span,
    },
    /// A hyper expression in a hybrid claim's `holds`: `unsupported_construct`
    /// / `expression-form`, at the expression.
    HyperExpression {
        /// The expression's span.
        span: Span,
    },
    /// A fairness constraint on a claim whose profile is bounded:
    /// `unsupported_construct` / `expression-form`.
    FairnessUnderBoundedProfile {
        /// The constraint's span.
        span: Span,
    },
    /// An interval bound, end, `*` or missing interval the claim's profile
    /// does not admit: `unsupported_construct` / `expression-form`, at the
    /// interval or the operator, naming the profile.
    ProfileFit {
        /// The claim's profile.
        profile: TemporalProfile,
        /// The interval's span, or the operator's when none is written.
        span: Span,
    },
    /// A lower bound above the upper, or equal bounds with an open end:
    /// `invalid_timed_model` / `invalid-interval`.
    InvalidInterval {
        /// The interval's span.
        span: Span,
    },
    /// A quantity bound whose unit is no time unit, or names none:
    /// `ill_typed` / `type-mismatch`, or `missing_declaration` /
    /// `missing-name`.
    TimeQuantity {
        /// Why the unit is no time unit.
        cause: Box<TimeUnitCause>,
        /// The unit's span.
        span: Span,
    },
    /// A capture's type does not resolve.
    CaptureType {
        /// Why the type form did not resolve.
        fault: TypeFormFault,
        /// The type form's span.
        span: Span,
    },
    /// A claim's reach is past `u64` positions.
    HorizonOutOfRange {
        /// The formula's span.
        span: Span,
    },
    /// Every continuous variable is a clock: `invalid_timed_model` /
    /// `rate-one-hybrid`, naming the timed model form that declares it.
    RateOneHybrid {
        /// The declaration name.
        name: String,
        /// The declaration name's span.
        span: Span,
    },
}

impl HybridCause {
    /// The refusal's own span (FR-096).
    pub fn span(&self) -> Span {
        match self {
            Self::DuplicateHybrid { span, .. }
            | Self::DuplicateMode { span, .. }
            | Self::MissingMode { span, .. }
            | Self::MissingName { span, .. }
            | Self::DuplicateEquation { span, .. }
            | Self::MissingEquation { span, .. }
            | Self::DuplicateReset { span, .. }
            | Self::Malformed { span, .. }
            | Self::NonBoolean { span }
            | Self::TypeMismatch { span }
            | Self::ZeroDenominator { span }
            | Self::CalleeForm { span, .. }
            | Self::DuplicateCapture { span, .. }
            | Self::NotTemporalProfile { span, .. }
            | Self::UnsupportedClock { span }
            | Self::HyperExpression { span }
            | Self::FairnessUnderBoundedProfile { span }
            | Self::ProfileFit { span, .. }
            | Self::InvalidInterval { span }
            | Self::TimeQuantity { span, .. }
            | Self::CaptureType { span, .. }
            | Self::HorizonOutOfRange { span }
            | Self::RateOneHybrid { span, .. } => *span,
        }
    }

    /// This cause's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::DuplicateHybrid { .. }
            | Self::DuplicateMode { .. }
            | Self::DuplicateEquation { .. }
            | Self::DuplicateReset { .. }
            | Self::DuplicateCapture { .. } => Code::ConflictingDeclaration,
            Self::MissingMode { .. } | Self::MissingName { .. } => Code::MissingDeclaration,
            Self::MissingEquation { .. } | Self::Malformed { .. } => Code::InvalidModelBinding,
            Self::NonBoolean { .. } | Self::TypeMismatch { .. } | Self::CalleeForm { .. } => {
                Code::IllTyped
            }
            Self::NotTemporalProfile { .. } => Code::UnknownProfile,
            Self::UnsupportedClock { .. }
            | Self::HyperExpression { .. }
            | Self::FairnessUnderBoundedProfile { .. }
            | Self::ProfileFit { .. }
            | Self::HorizonOutOfRange { .. } => Code::UnsupportedConstruct,
            Self::InvalidInterval { .. } => Code::InvalidTimedModel,
            Self::TimeQuantity { cause, .. } => match &**cause {
                TimeUnitCause::NoTimeDimension { .. } | TimeUnitCause::UnknownUnit { .. } => {
                    Code::MissingDeclaration
                }
                TimeUnitCause::NotTimeDimension { .. } | TimeUnitCause::AffineUnit { .. } => {
                    Code::IllTyped
                }
            },
            Self::CaptureType { fault, .. } => type_fault_class(fault).0,
            Self::ZeroDenominator { .. } => Code::UndefinedExpression,
            Self::RateOneHybrid { .. } => Code::InvalidTimedModel,
        }
    }

    /// This cause's catalog tag.
    pub fn tag(&self) -> &'static str {
        match self {
            Self::DuplicateHybrid { .. }
            | Self::DuplicateMode { .. }
            | Self::DuplicateEquation { .. }
            | Self::DuplicateReset { .. }
            | Self::DuplicateCapture { .. } => "duplicate-declaration",
            Self::MissingMode { .. } | Self::MissingName { .. } => "missing-name",
            Self::MissingEquation { .. } | Self::Malformed { .. } => "malformed-declaration",
            Self::NonBoolean { .. } => "non-boolean-root",
            Self::TypeMismatch { .. } | Self::CalleeForm { .. } => "type-mismatch",
            Self::NotTemporalProfile { .. } => "unsupported-selection",
            Self::UnsupportedClock { .. } => "declaration-form",
            Self::HyperExpression { .. } => "expression-form",
            Self::FairnessUnderBoundedProfile { .. }
            | Self::ProfileFit { .. }
            | Self::HorizonOutOfRange { .. } => "expression-form",
            Self::InvalidInterval { .. } => "invalid-interval",
            Self::TimeQuantity { cause, .. } => match &**cause {
                TimeUnitCause::NoTimeDimension { .. } | TimeUnitCause::UnknownUnit { .. } => {
                    "missing-name"
                }
                TimeUnitCause::NotTimeDimension { .. } | TimeUnitCause::AffineUnit { .. } => {
                    "type-mismatch"
                }
            },
            Self::CaptureType { fault, .. } => type_fault_class(fault).1,
            Self::ZeroDenominator { .. } => "unproved-nonzero",
            Self::RateOneHybrid { .. } => "rate-one-hybrid",
        }
    }
}

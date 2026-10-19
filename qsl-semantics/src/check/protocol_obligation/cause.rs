// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-381 and FR-218: why S3 refused a protocol's roles, activation,
//! captures or compensation templates, with the members each refusal
//! retains. Every variant carries its own span, which
//! `region::DeclarationRegions::refusal_region` reads directly: a protocol
//! has no expression tree for a `Location` path to walk.

use qsl_foundation::diagnostic::Code;
use qsl_foundation::Span;
use quire_exact::ValueType;

use crate::check::refusal::{type_fault_class, CheckCause};
use crate::check::type_form::TypeFormFault;

/// The stage whose expression a refusal is about (FR-381 "Names a stage may
/// read", first column of the table).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ObligationStage {
    /// `activation on each ... when`.
    ProtocolActivation,
    /// A capture of the protocol.
    ProtocolCapture,
    /// A capture of a template written before `activate`.
    RegistrationCapture,
    /// `activate first ... when`.
    TriggerGuard,
    /// A capture of a template written after `activate`.
    ActivationCapture,
    /// A template's `retry` relation.
    Retry,
    /// A template's `recover` predicate.
    Recovery,
}

impl ObligationStage {
    /// The identifier the table gives the stage.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::ProtocolActivation => "protocol-activation",
            Self::ProtocolCapture => "protocol-capture",
            Self::RegistrationCapture => "registration-capture",
            Self::TriggerGuard => "trigger-guard",
            Self::ActivationCapture => "activation-capture",
            Self::Retry => "retry",
            Self::Recovery => "recovery",
        }
    }

    /// Whether the stage bars every read of model state (FR-218 "Model
    /// state", FR-381): the activation guard, the trigger guard, the retry
    /// relation and the recovery predicate. A capture may read model state.
    pub const fn bars_model_state(self) -> bool {
        match self {
            Self::ProtocolActivation | Self::TriggerGuard | Self::Retry | Self::Recovery => true,
            Self::ProtocolCapture | Self::RegistrationCapture | Self::ActivationCapture => false,
        }
    }
}

/// What owns the stage: the protocol, or one `compensate` template by name.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ObligationOwner {
    /// The protocol clause.
    Protocol,
    /// The template of this name.
    Compensation(String),
}

/// Why a name a stage may not read was refused (FR-381 "Names a stage may
/// read").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReadClass {
    /// A capture of the same stage written later than the read, or the
    /// capture itself.
    Forward,
    /// A name a template declares, read outside that template.
    Foreign,
    /// A name that exists but that the reading stage may not read.
    Stage,
    /// A name nothing declares.
    Unbound,
}

impl ReadClass {
    /// The `read` member's value.
    pub const fn identifier(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Foreign => "foreign",
            Self::Stage => "stage",
            Self::Unbound => "unbound",
        }
    }
}

/// Why S3 refused a role, an activation, a capture or a compensation
/// template.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ObligationCause {
    /// `missing_declaration`/`missing-name`: a stage read a name it may not.
    StageRead {
        /// The name read.
        name: String,
        /// The stage that read it.
        stage: ObligationStage,
        /// What owns the stage.
        owner: ObligationOwner,
        /// Why the stage may not read it.
        read: ReadClass,
        /// The read.
        span: Span,
    },
    /// `ill_typed`/`operator-ineligible`: a stage that bars model state read
    /// the `over` input or a model-state value.
    ModelStateRead {
        /// The stage that read it.
        stage: ObligationStage,
        /// What owns the stage.
        owner: ObligationOwner,
        /// The read.
        span: Span,
    },
    /// `ill_typed`/`type-mismatch`: a capture's initializer is not of the
    /// capture's declared type.
    CaptureType {
        /// The capture.
        capture: String,
        /// The declared type.
        expected: ValueType,
        /// The initializer's type.
        actual: ValueType,
        /// The initializer.
        span: Span,
    },
    /// A refusal of the clause checker inside a stage's expression: a
    /// non-Boolean root, an undischarged obligation, an operator-ineligible
    /// operand, a missing qualified name. Its code and cause are the
    /// wrapped cause's.
    Clause {
        /// The stage whose expression it is.
        stage: ObligationStage,
        /// What owns the stage.
        owner: ObligationOwner,
        /// The clause checker's cause.
        cause: Box<CheckCause>,
        /// The located node.
        span: Span,
    },
    /// `ill_typed`/`type-mismatch`: a template's forward binder is not the
    /// record type of the effect it registers for.
    ForwardType {
        /// The template.
        compensation: String,
        /// The effect's record type.
        expected: ValueType,
        /// The binder's declared type.
        actual: ValueType,
        /// The binder's type.
        span: Span,
    },
    /// `missing_declaration`/`missing-name`: a template's `by` role is no
    /// role of the protocol.
    MissingRole {
        /// The template.
        compensation: String,
        /// The role as written.
        name: String,
        /// The role name.
        span: Span,
    },
    /// `missing_declaration`/`missing-name`: a template's operation, or the
    /// type that declares it, resolves to nothing.
    MissingOperation {
        /// The template.
        compensation: String,
        /// The unresolved name: the type, or the operation.
        name: String,
        /// The unresolved part.
        span: Span,
    },
    /// `ambiguous_declaration`/`ambiguous-name`: a template's operation is
    /// inherited from more than one declaring type.
    AmbiguousOperation {
        /// The template.
        compensation: String,
        /// The operation as written.
        name: String,
        /// The operation name.
        span: Span,
    },
    /// `ill_typed`/`type-mismatch`: `attempts 0`.
    ZeroAttempts {
        /// The template.
        compensation: String,
        /// The count.
        span: Span,
    },
    /// A template's `attempts n of T` names a type that does not resolve:
    /// `missing_declaration`/`missing-name`, or `ambiguous_declaration`/
    /// `ambiguous-name`, as a declared type is refused.
    AttemptTypeName {
        /// The template.
        compensation: String,
        /// Why the form did not resolve.
        fault: TypeFormFault,
        /// The unresolved name.
        span: Span,
    },
    /// `invalid_model_binding`/`malformed-declaration`: a template's
    /// attempt type is not a record type or an event type.
    AttemptType {
        /// The template.
        compensation: String,
        /// The type as written, when it is written as a name.
        name: Option<String>,
        /// The type.
        span: Span,
    },
    /// `ill_typed`/`type-mismatch`: a `retry` binder is not of the attempt
    /// type.
    RetryBinder {
        /// The template.
        compensation: String,
        /// The binder.
        binder: String,
        /// The attempt type.
        expected: ValueType,
        /// The binder's declared type.
        actual: ValueType,
        /// The binder's type.
        span: Span,
    },
    /// `ill_typed`/`type-mismatch`: a `within` interval with `a > b` or an
    /// open upper bound.
    Deadline {
        /// The template.
        compensation: String,
        /// The lower bound as written.
        lower: u64,
        /// The upper bound as written; `None` for `*`.
        upper: Option<u64>,
        /// The interval.
        span: Span,
    },
}

impl ObligationCause {
    /// The refusal's own span.
    pub fn span(&self) -> Span {
        match self {
            Self::StageRead { span, .. }
            | Self::ModelStateRead { span, .. }
            | Self::CaptureType { span, .. }
            | Self::Clause { span, .. }
            | Self::ForwardType { span, .. }
            | Self::MissingRole { span, .. }
            | Self::MissingOperation { span, .. }
            | Self::AmbiguousOperation { span, .. }
            | Self::ZeroAttempts { span, .. }
            | Self::AttemptTypeName { span, .. }
            | Self::AttemptType { span, .. }
            | Self::RetryBinder { span, .. }
            | Self::Deadline { span, .. } => *span,
        }
    }

    /// This cause's catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::StageRead { .. } | Self::MissingRole { .. } | Self::MissingOperation { .. } => {
                Code::MissingDeclaration
            }
            Self::ModelStateRead { .. }
            | Self::CaptureType { .. }
            | Self::ForwardType { .. }
            | Self::ZeroAttempts { .. }
            | Self::RetryBinder { .. }
            | Self::Deadline { .. } => Code::IllTyped,
            Self::Clause { cause, .. } => cause.code(),
            Self::AmbiguousOperation { .. } => Code::AmbiguousDeclaration,
            Self::AttemptTypeName { fault, .. } => type_fault_class(fault).0,
            Self::AttemptType { .. } => Code::InvalidModelBinding,
        }
    }

    /// This cause's catalog tag.
    pub fn tag(&self) -> Option<&'static str> {
        match self {
            Self::StageRead { .. } | Self::MissingRole { .. } | Self::MissingOperation { .. } => {
                Some("missing-name")
            }
            Self::ModelStateRead { .. } => Some("operator-ineligible"),
            Self::CaptureType { .. }
            | Self::ForwardType { .. }
            | Self::ZeroAttempts { .. }
            | Self::RetryBinder { .. }
            | Self::Deadline { .. } => Some("type-mismatch"),
            Self::Clause { cause, .. } => cause.cause(),
            Self::AmbiguousOperation { .. } => Some("ambiguous-name"),
            Self::AttemptTypeName { fault, .. } => Some(type_fault_class(fault).1),
            Self::AttemptType { .. } => Some("malformed-declaration"),
        }
    }
}

// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-043/FR-044: the orthogonal dimensions a temporal report names.
//!
//! Both progress/closure axes, assessment execution and input completeness are
//! independent. Closing one axis never closes or completes another, and no
//! refusal is reported as a Boolean.

/// Which obligation, position and temporal node a result or stop belongs to.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Subject {
    /// Declaration index in the admitted package.
    pub declaration: usize,
    /// Obligation instance ordinal within this evaluation. The instance's
    /// semantic identity is carried by `Assessment::instance`, not by this
    /// ordinal, which exists only to locate a stop.
    pub instance: usize,
    /// Temporal arena node under evaluation, when one was selected.
    pub node: Option<usize>,
    /// Trace position under evaluation, when one was selected.
    pub position: Option<usize>,
    /// Declared capture ordinal under evaluation, when one was selected. This
    /// is what lets an incomplete or refused activation name *which* capture
    /// could not be established rather than only that one could not.
    pub capture: Option<usize>,
}

/// Independent progress/closure state of one axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Closure {
    /// The axis has not yet progressed to its final state.
    Open,
    /// The axis has reached its final state; nothing further can revise it.
    Closed,
}

/// Independent assessment-execution disposition, separate from truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Execution {
    /// The assessment ran to completion.
    Completed,
    /// The assessment could not run to completion.
    Failed,
}

/// Independent input completeness, separate from both closure axes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completeness {
    /// Every required input was supplied.
    Complete,
    /// At least one required input was absent.
    Incomplete,
}

/// A dimension that did not match the declaration's admitted selection, or that
/// a requested mapping could not express.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Dimension {
    /// Selected temporal profile identity.
    Profile,
    /// Clock binding name.
    Clock,
    /// Declared fixed-sample period clock parameter.
    SamplePeriod,
    /// Declared fixed-sample epoch clock parameter.
    Epoch,
    /// Declared timestamp unit clock parameter.
    TimestampUnit,
    /// Declared sequence authority clock parameter.
    SequenceAuthority,
    /// Fixed-sample unit selected by the authenticated artifact.
    ClockUnit,
    /// Exact parameter-key inventory selected by the authenticated artifact.
    ClockParameters,
    /// Admitted order key over positions sharing a clock coordinate.
    AdmittedOrder,
    /// Declared interval bound.
    Interval,
    /// A declared capture and its initializer.
    Capture,
    /// Activation guard valuation.
    Guard,
    /// Semantic trigger or execution-origin identity.
    TriggerIdentity,
    /// Declared activation anchor.
    Anchor,
    /// An atomic valuation at a trace position.
    Valuation,
    /// Retained history available to a past operator.
    History,
    /// A bounded or unbounded past operator.
    PastOperator,
    /// A finite-window bounded future operator.
    FiniteWindow,
    /// The open, unbounded prefix of an execution.
    OpenPrefix,
    /// Progress in the profile's clock domain, asserted by a watermark.
    Watermark,
    /// Decision-scope closure asserted at the progress watermark.
    DecisionScope,
    /// The input-completeness assertion, independent of both closure axes.
    Completeness,
}

/// A located refusal. No refusal is ever reported as a temporal Boolean.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// A trace dimension differs from the declaration's admitted selection.
    #[error("temporal binding mismatch on {dimension:?}")]
    Binding {
        /// Which trace dimension mismatched.
        dimension: Dimension,
        /// Which declaration, instance, node and position the mismatch was located at.
        subject: Subject,
    },
    /// Order-sensitive operator over positions sharing a clock coordinate with
    /// no admitted order key.
    #[error("order-sensitive temporal operator without an admitted order")]
    Order {
        /// Which declaration, instance, node and position the refusal was located at.
        subject: Subject,
    },
    /// A capture could not be established at the activation anchor.
    #[error("temporal capture could not be established: {dimension:?}")]
    Capture {
        /// Which capture-related dimension could not be established.
        dimension: Dimension,
        /// Which declaration, instance, node and position the refusal was located at.
        subject: Subject,
    },
    /// Two deliveries asserted one semantic trigger or execution-origin
    /// identity with conflicting payloads.
    #[error("conflicting temporal deliveries under identity {identity}")]
    Contradiction {
        /// The semantic identity the conflict was asserted under.
        identity: String,
        /// Which declaration, instance, node and position the conflict was located at.
        subject: Subject,
    },
    /// A progress assertion contradicts the progress already retained under one
    /// binding: a regressing watermark, or a completeness assertion revised in
    /// conflict. The retained progress is not rolled back, the retained closure
    /// is not restamped and no earlier result is rewritten.
    #[error("temporal progress contradiction on {dimension:?} under clock {clock}")]
    Progress {
        /// Which retained dimension the assertion contradicts.
        dimension: Dimension,
        /// Clock binding name the contradiction was asserted under.
        clock: String,
        /// Which declaration, instance, node and position the contradiction was located at.
        subject: Subject,
    },
    /// A valuation-evidence row failed one of the checks QSL makes against
    /// values it holds (FR-043-AC-16). No truth and no partial support follow.
    #[error("temporal valuation row {row} refused: {cause:?}")]
    Valuation {
        /// The check the row failed.
        cause: super::valuation::ValuationCause,
        /// The row's index in admission order.
        row: usize,
    },
    /// The emitted graph did not match the shape this evaluator admits.
    #[error("temporal graph reference is not admissible")]
    Reference {
        /// Which declaration, instance, node and position the refusal was located at.
        subject: Subject,
    },
}

/// A single bounded outcome. A refusal is never reported as a temporal Boolean.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A located refusal; no refusal is ever reported as a temporal Boolean.
    #[error(transparent)]
    Refused(Refusal),
}

impl From<Refusal> for Error {
    fn from(value: Refusal) -> Self {
        Self::Refused(value)
    }
}

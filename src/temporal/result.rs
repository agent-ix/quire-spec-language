// SPDX-License-Identifier: AGPL-3.0-only
//! FR-043/FR-044: the orthogonal dimensions a temporal assessment reports.
//!
//! Truth, settlement basis, activation, both progress/closure axes, assessment
//! execution and input completeness are independent. Closing one axis never
//! closes or completes another, and no resource stop, missing observation or
//! unsupported mapping is reported as a Boolean.

use std::collections::BTreeMap;

use super::budget::{Exhaustion, Usage};
use super::profile::Profile;

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
}

/// Bounded temporal truth. `Pending` is a distinct outcome, never a Boolean.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Truth {
    True,
    False,
    Pending,
}

/// Why a truth was settled, using the shared settlement vocabulary. A basis is
/// accepted only with its valid truth and scope combination.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Basis {
    /// A complete closed decision scope authorized the closed-boundary rule.
    ClosedScope,
    /// An open scope settled true from a witness preserved by every admitted
    /// continuation.
    DecisiveWitness,
    /// An open scope settled false from a counterexample preserved by every
    /// admitted continuation.
    DecisiveCounterexample,
    /// An open future whose admitted continuations do not preserve a Boolean.
    Unsettled,
    /// A fact inside the decision-support set is missing, so the truth is not
    /// available. This is not `false` and not `pending`.
    Unavailable,
}

/// Independent progress/closure state of one axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Closure {
    Open,
    Closed,
}

/// Independent assessment-execution disposition, separate from truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Execution {
    Completed,
    Failed,
}

/// Independent input completeness, separate from both closure axes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Completeness {
    Complete,
    Incomplete,
}

/// Activation is separate from truth. Only a closed-complete trigger scope with
/// no admitted trigger is `Inactive`; every other non-active shape is `Unknown`
/// carrying its own completeness and execution disposition.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Activation {
    /// Closed-complete trigger scope, no admitted trigger. Not a true
    /// obligation, and not evidence that recovery behavior was exercised.
    Inactive,
    /// Open scope, or missing or refused trigger evidence.
    Unknown {
        completeness: Completeness,
        execution: Execution,
    },
    /// One admitted semantic trigger, or a whole-execution origin.
    Active,
}

/// The exact premises a result depends on. Two results with different premises
/// are different results; equal formula bytes never merge them.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Premises {
    /// Selected temporal profile, retained by identity.
    pub profile: Profile,
    /// Exact admitted profile revision, distinct from the language edition.
    pub profile_revision: String,
    /// Exact clock binding name the declaration selected.
    pub clock: String,
    /// Declared clock parameters the trace asserted. The emitted body carries
    /// none of them, so they are retained premises: changing one changes result
    /// identity, but this evaluator does not authenticate them.
    pub clock_parameters: BTreeMap<String, String>,
    /// Progress watermark in the profile's clock domain.
    pub watermark: i64,
    /// Decision-scope progress and closure.
    pub decision_scope: Closure,
    /// Surrounding-execution progress and closure, independent of the above.
    pub surrounding_execution: Closure,
    /// Assessment-execution disposition.
    pub execution: Execution,
    /// Input completeness, independent of both closure axes.
    pub completeness: Completeness,
    /// Whether the trace's lower boundary is an authoritative execution origin.
    /// A mere history cutoff is not, and cannot decide a past operator.
    pub authoritative_origin: bool,
    /// Effective ceilings in force, which participate in result identity.
    pub limits: super::budget::Limits,
}

/// Trace positions whose valuations established the reported truth.
pub type Support = Vec<usize>;

/// One retained immutable capture record. The public API exposes no mutable
/// path to this value after activation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Capture {
    /// Index of the capture's declared initializer handle in the authored list.
    pub declared: usize,
    /// The anchor the value was established at.
    pub anchor: String,
    /// The established value, retained verbatim.
    pub value: String,
}

/// One obligation instance's assessed temporal outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Assessment {
    pub subject: Subject,
    /// Semantic instance identity: the admitted trigger or execution-origin
    /// identity this obligation was keyed by. Absent where no instance was
    /// created.
    pub instance: Option<String>,
    /// Receipt identities delivered for this instance, in delivery order. A
    /// repeated delivery adds provenance here and creates no second instance.
    pub receipts: Vec<String>,
    /// Retained immutable captures, in authored source order.
    pub captures: Vec<Capture>,
    /// How many times this instance's capture initializers were evaluated.
    /// Exactly one for an activated instance, across incremental
    /// re-evaluation, restoration and replay.
    pub capture_evaluations: usize,
    pub activation: Activation,
    /// Absent when activation carried no obligation to assess.
    pub truth: Option<Truth>,
    /// Absent where no obligation was assessed for this instance.
    pub basis: Option<Basis>,
    /// A required fact inside the decision support that was missing. The
    /// assessment is still reported so a sibling's established value stays
    /// inspectable beside it.
    pub incomplete: Option<Incomplete>,
    /// Exact decision support; a missing fact inside this set makes the truth
    /// unavailable, while a missing fact outside it is a completeness gap that
    /// does not falsify or delay the settled truth.
    pub support: Support,
    pub premises: Premises,
}

/// A dimension that did not match the declaration's admitted selection, or that
/// a requested mapping could not express.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Dimension {
    Profile,
    ProfileRevision,
    Clock,
    SamplePeriod,
    Epoch,
    TimestampUnit,
    SequenceAuthority,
    AdmittedOrder,
    Interval,
    Capture,
    Guard,
    TriggerIdentity,
    Anchor,
    Valuation,
    History,
    PastOperator,
    FiniteWindow,
    OpenPrefix,
}

/// A located refusal. No refusal is ever reported as a temporal Boolean.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// A trace dimension differs from the declaration's admitted selection.
    #[error("temporal binding mismatch on {dimension:?}")]
    Binding {
        dimension: Dimension,
        subject: Subject,
    },
    /// Order-sensitive operator over positions sharing a clock coordinate with
    /// no admitted order key.
    #[error("order-sensitive temporal operator without an admitted order")]
    Order { subject: Subject },
    /// A capture could not be established at the activation anchor.
    #[error("temporal capture could not be established: {dimension:?}")]
    Capture {
        dimension: Dimension,
        subject: Subject,
    },
    /// A monotonic progress assertion regressed, or a completeness assertion
    /// was revised in conflict, under one binding.
    #[error("temporal progress or completeness contradiction")]
    Contradiction { subject: Subject },
    /// The emitted graph did not match the shape this evaluator admits.
    #[error("temporal graph reference is not admissible")]
    Reference { subject: Subject },
}

/// A required input was absent. Distinct from `false`, from `Pending` and from
/// a refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("required temporal {dimension:?} is missing")]
pub struct Incomplete {
    pub dimension: Dimension,
    pub subject: Subject,
}

/// One obligation's outcome. Activation failing for one instance leaves every
/// sibling instance inspectable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Obligation {
    Assessed(Box<Assessment>),
    /// Activation could not be established for this instance.
    Unactivated {
        subject: Subject,
        /// Semantic instance identity, where one was known before the failure.
        instance: Option<String>,
        error: Error,
    },
}

/// A single bounded outcome. Exhausted work never returns a partial Boolean.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error(transparent)]
    Refused(Refusal),
    #[error(transparent)]
    Incomplete(Incomplete),
    #[error(transparent)]
    Exhausted(Exhaustion),
}

impl From<Exhaustion> for Error {
    fn from(value: Exhaustion) -> Self {
        Self::Exhausted(value)
    }
}

impl From<Incomplete> for Error {
    fn from(value: Incomplete) -> Self {
        Self::Incomplete(value)
    }
}

impl From<Refusal> for Error {
    fn from(value: Refusal) -> Self {
        Self::Refused(value)
    }
}

/// Effective limits and successful work accompany either outcome.
#[derive(Clone, Debug)]
pub struct Report {
    pub(super) result: Result<Vec<Obligation>, Error>,
    pub(super) limits: super::budget::Limits,
    pub(super) usage: Usage,
}

impl Report {
    /// The assessed obligations, or the single bounded outcome that stopped the
    /// evaluation. A stop is never rendered as a Boolean.
    pub fn result(&self) -> Result<&[Obligation], &Error> {
        match &self.result {
            Ok(value) => Ok(value),
            Err(error) => Err(error),
        }
    }
    /// Effective ceilings after clamping, which participate in result identity.
    pub fn limits(&self) -> super::budget::Limits {
        self.limits
    }
    /// Successfully charged work; refused charges do not increase usage.
    pub fn usage(&self) -> Usage {
        self.usage
    }
}

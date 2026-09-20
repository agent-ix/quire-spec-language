// SPDX-License-Identifier: AGPL-3.0-or-later
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
    /// Declared capture ordinal under evaluation, when one was selected. This
    /// is what lets an incomplete or refused activation name *which* capture
    /// could not be established rather than only that one could not.
    pub capture: Option<usize>,
}

/// Bounded temporal truth. `Pending` is a distinct outcome, never a Boolean.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Truth {
    /// The obligation's formula is settled true.
    True,
    /// The obligation's formula is settled false.
    False,
    /// The truth is not yet settled by any admitted continuation.
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
        /// Input completeness toward establishing activation.
        completeness: Completeness,
        /// Assessment-execution disposition toward establishing activation.
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
    /// Which declaration, instance, node and position this assessment belongs to.
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
    /// Whether the instance is active, inactive, or unknown, and why.
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
    /// The exact premises this assessment depends on.
    pub premises: Premises,
}

/// A dimension that did not match the declaration's admitted selection, or that
/// a requested mapping could not express.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Dimension {
    /// Selected temporal profile identity.
    Profile,
    /// Exact admitted profile revision.
    ProfileRevision,
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
    /// The emitted graph did not match the shape this evaluator admits.
    #[error("temporal graph reference is not admissible")]
    Reference {
        /// Which declaration, instance, node and position the refusal was located at.
        subject: Subject,
    },
}

/// A required input was absent. Distinct from `false`, from `Pending` and from
/// a refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("required temporal {dimension:?} is missing")]
pub struct Incomplete {
    /// Which dimension was required but absent.
    pub dimension: Dimension,
    /// Which declaration, instance, node and position the missing input was
    /// required at.
    pub subject: Subject,
}

/// One obligation's outcome. Activation failing for one instance leaves every
/// sibling instance inspectable.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Obligation {
    /// Activation was established and the instance was assessed.
    Assessed(Box<Assessment>),
    /// Activation could not be established for this instance.
    Unactivated {
        /// Which declaration, instance, node and position the failure was located at.
        subject: Subject,
        /// Semantic instance identity. Known whenever the trigger was admitted,
        /// which is every case an activation can fail in.
        instance: String,
        /// The bounded outcome that stopped activation.
        error: Error,
    },
}

/// A single bounded outcome. Exhausted work never returns a partial Boolean.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A located refusal; no refusal is ever reported as a temporal Boolean.
    #[error(transparent)]
    Refused(Refusal),
    /// A required input was absent.
    #[error(transparent)]
    Incomplete(Incomplete),
    /// A resource ceiling was reached before the work could complete.
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
    // Present only when a strict version-2 entry point authenticated the trace.
    pub(super) authenticated: Option<super::progress::AuthenticatedBinding>,
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
    /// The clock identity a strict version-2 entry point authenticated before
    /// running, retained so a caller can tell an authenticated result from an
    /// unauthenticated version-1 one without re-deriving it.
    ///
    /// This is `None` for every version-1 entry point, and also for a version-2
    /// evaluation refused by authentication itself: nothing was authenticated.
    pub fn authenticated(&self) -> Option<&super::progress::AuthenticatedBinding> {
        self.authenticated.as_ref()
    }
}

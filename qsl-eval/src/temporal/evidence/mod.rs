// SPDX-License-Identifier: AGPL-3.0-or-later
//! The temporal evidence call's clock binding, result identity and work
//! counters (FR-522, FR-523).
//!
//! * [`TraceBinding`] and [`ClauseSelection`]: a request's asserted profile,
//!   clock name and clock parameters are compared with the checked clause's
//!   own rows before any position is visited, and the agreeing binding is
//!   retained as an [`AuthenticatedBinding`] (FR-522).
//! * [`IdentityBuilder`]: the result identity is an equality of the members
//!   a result was computed from (FR-522).
//! * [`TemporalEvidenceLimits`], [`CallMeter`] and [`EvidenceUsage`]: the
//!   thirteen counters, their ceilings and the `temporal_evidence.*`
//!   settings, with the stop scope of one call (FR-523).

mod activation;
mod binding;
mod call;
mod counters;
mod identity;
mod ledger;
mod limits;
mod package;
mod request;
mod trigger;
mod work;

pub use super::facts::{ClauseFacts, ClauseFormula};
pub use activation::{
    activate, scope_disposition, Activation, ActivationFault, ActivationInput, ActivationStop,
    ClauseShape, Completeness, Contradiction, Disposition, ExecutionDisposition, FaultKind,
    FaultSubject, Instance, InstanceState, SurplusCapture, UnknownKind,
};
pub use binding::{
    authenticate_clock, AuthenticatedBinding, BindingMismatch, ClauseSelection,
    MismatchedDimension, TraceBinding, ADMIT_STAGE,
};
pub use call::evaluate_temporal_evidence;
pub use counters::{CallMeter, ClauseStop, EvidenceStop, LimitReached, Visit};
pub use identity::{
    IdentityBuilder, IdentityMember, InputIdentity, MemberEncoder, MemberFault, ResultIdentity,
};
pub use ledger::{
    CaptureEntry, CaptureEnvironment, EvidenceLedger, ObligationKey, ResultEntry, RetainedInstance,
    SourceBinding,
};
pub use limits::{CounterKind, EvidenceCounter, EvidenceUsage, TemporalEvidenceLimits};
pub use package::{bound_atom_indices, ClauseName, EvidencePackage, PackageMismatch, PackageRows};
pub use request::{
    ActivationRow, ActivationState, CompletenessRecord, EvidenceIncomplete, EvidencePosition,
    EvidenceRef, EvidenceRequest, EvidenceResult, EvidenceSubject, Indeterminate, NonValue,
    NonValueKind, Relation, Settlement, SubjectOccurrence, SupportRef, Truth,
};
pub use trigger::{
    CaptureFact, CapturedValue, EmptyIdentity, Evicted, ExecutionOrigin, Payload, Receipt,
    SemanticTriggerIdentity, TriggerDelivery, TriggerEvidence, TriggerRecord, TriggerScope,
};
pub(super) use work::shape;
pub use work::{ClauseWork, NodeReach, NodeShape, ReachFault};

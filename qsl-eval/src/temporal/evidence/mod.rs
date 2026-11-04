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

mod binding;
mod counters;
mod identity;
mod limits;
mod work;

pub use binding::{
    authenticate_clock, AuthenticatedBinding, BindingMismatch, ClauseSelection,
    MismatchedDimension, TraceBinding, ADMIT_STAGE,
};
pub use counters::{CallMeter, ClauseStop, EvidenceStop, LimitReached, Visit};
pub use identity::{
    IdentityBuilder, IdentityMember, InputIdentity, MemberEncoder, MemberFault, ResultIdentity,
};
pub use limits::{CounterKind, EvidenceCounter, EvidenceUsage, TemporalEvidenceLimits};
pub use work::{ClauseWork, NodeReach, NodeShape, ReachFault};

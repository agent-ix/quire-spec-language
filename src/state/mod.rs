// SPDX-License-Identifier: AGPL-3.0-only
//! FR-046/047/049: bounded evaluation of admitted composed artifacts.

mod evaluation;
mod input;
mod work;

pub use evaluation::{evaluate, evaluate_v2};
pub use input::{
    AssessmentAuthority, AuthorityAdapter, AuthorityEvidence, BinderInput, CanonicalDigest,
    ContextualSlot, ContextualValue, ContextualValueKind, EvaluationOutcome, EvaluationReport,
    EvaluationRequest, FieldInput, FieldValue, InputSlot, MissingInput, ObjectInput, ObjectKey,
    ObservationDigest, ObservationIdentity, ObservationKey, PopulationInput, Refusal, StateView,
    StaticAuthority, Value, ValueKind, ValuePathSegment, OBSERVATION_CONTRACT_REVISION,
    PRODUCER_CONTRACT_REVISION,
};
pub use work::{Dimension, Exhaustion, ExhaustionCause, Limits, Usage, ACCOUNTING_VERSION};

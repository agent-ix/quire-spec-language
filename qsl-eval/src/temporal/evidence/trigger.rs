// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-524, FR-053: what an observation owner supplies to activate a temporal
//! clause: the opaque semantic trigger identity, one delivery of a trigger,
//! one execution origin, and the facts that capture the clause's
//! initializers.
//!
//! Every byte string here is the owner's and is opaque to the evaluator: it
//! is compared byte for byte, echoed byte for byte and never interpreted
//! (FR-053).

use crate::temporal::ledger::ProgressClosure;

/// An identity of zero bytes was supplied where an opaque identity is
/// required (FR-053, FR-526-AC-18).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("an opaque trigger identity has at least one byte")]
pub struct EmptyIdentity;

/// The opaque identity of one semantic trigger or one admitted execution
/// origin, selected by the observation owner (FR-053). It holds at least one
/// byte and is equal to another identity exactly when every byte is.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticTriggerIdentity(Box<[u8]>);

impl SemanticTriggerIdentity {
    /// The identity of `bytes`.
    ///
    /// # Errors
    ///
    /// [`EmptyIdentity`] for an empty byte string.
    pub fn new(bytes: impl Into<Box<[u8]>>) -> Result<Self, EmptyIdentity> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            Err(EmptyIdentity)
        } else {
            Ok(Self(bytes))
        }
    }

    /// The identity's bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

macro_rules! opaque_bytes {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(Box<[u8]>);

        impl $name {
            /// The value of `bytes`.
            pub fn new(bytes: impl Into<Box<[u8]>>) -> Self {
                Self(bytes.into())
            }

            /// The bytes, as supplied.
            pub fn as_bytes(&self) -> &[u8] {
                &self.0
            }
        }
    };
}

opaque_bytes! {
    /// The receipt one relay stamped on a delivery: provenance only, never
    /// identity (FR-524 "Instance identity").
    Receipt
}

opaque_bytes! {
    /// The payload a delivery carries. Two deliveries of one identity are
    /// one instance when their payloads are equal and a contradiction when
    /// they differ (FR-044, FR-524).
    Payload
}

opaque_bytes! {
    /// The value of a capture in the clause's declared type, in the owner's
    /// canonical encoding of it.
    CapturedValue
}

/// What a delivery says about one capture of the clause (FR-524 "Inputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureFact {
    /// The capture has a value, read at `anchor`.
    Value {
        /// The identity of the position the value was read at.
        anchor: String,
        /// The value.
        value: CapturedValue,
    },
    /// The capture's source has no value.
    Missing,
    /// The capture's source is null.
    Null,
    /// The supplied value is not of the capture's declared type.
    WrongType,
    /// The supplied value is stale for the activation.
    Stale,
}

/// One delivery of a semantic trigger to a clause that activates `on each`
/// (FR-524 "Inputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TriggerDelivery {
    /// The trigger's identity.
    pub identity: SemanticTriggerIdentity,
    /// The receipt this delivery arrived under.
    pub receipt: Receipt,
    /// The identity of the position the delivery activates at.
    pub anchor: String,
    /// The payload.
    pub payload: Payload,
    /// The value of the clause's `when` condition for this delivery, when
    /// the owner established one.
    pub guard: Option<bool>,
    /// The capture facts, in the clause's authored order.
    pub captures: Vec<CaptureFact>,
}

/// One admitted execution origin of a clause that activates `on origin`
/// (FR-524 "Inputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionOrigin {
    /// The origin's identity.
    pub identity: SemanticTriggerIdentity,
    /// The payload.
    pub payload: Payload,
    /// The capture facts, in the clause's authored order, which a clause
    /// with captures establishes from exactly as a delivery does.
    pub captures: Vec<CaptureFact>,
}

/// A retained record the caller reports as no longer available (FR-524
/// "Inputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Evicted {
    /// The record of one capture of one instance.
    Capture {
        /// The instance.
        instance: SemanticTriggerIdentity,
        /// The capture's declared name.
        capture: String,
    },
    /// The record of one valuation of one atom at one position, shared
    /// (`instance` is `None`) or bound to one instance.
    Valuation {
        /// The instance the valuation is bound to, when it is.
        instance: Option<SemanticTriggerIdentity>,
        /// The admitted position's stamp coordinate in the request.
        position: u64,
        /// The atom's node index in the clause's formula.
        atom: u64,
    },
}

/// Whether the trigger evidence the owner supplied was admitted (FR-516
/// `trigger_evidence`, retained `Evidence` vocabulary).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TriggerEvidence {
    /// The evidence was admitted.
    Admitted,
    /// The evidence is missing.
    Missing,
    /// The evidence was refused.
    Refused,
}

/// Whether the trigger record is complete (FR-516 `completeness` state).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum TriggerRecord {
    /// Every trigger of the scope is in the record.
    Complete,
    /// The record may lack triggers.
    Incomplete,
}

/// The trigger scope of an activation (FR-524 "Inputs"), independent of the
/// decision scope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TriggerScope {
    /// Whether more triggers may still arrive.
    pub closure: ProgressClosure,
    /// What the owner supplied as evidence of the scope.
    pub evidence: TriggerEvidence,
    /// Whether the trigger record is complete.
    pub record: TriggerRecord,
}

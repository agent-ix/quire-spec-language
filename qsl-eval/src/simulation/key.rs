// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101: the simulator state key, its typed canonical form and its digest.
//!
//! A `TransitionSystem` hands the engine a `Serialize` view of a state (or a
//! transition identity); this module is the one place that turns that view
//! into RFC 8785 JCS bytes and, for a state, its `quire.simulation.state-key/
//! v1` digest. It never interprets the value beyond that: what the view
//! contains is the `TransitionSystem` implementer's concern (QSpec FR-181's
//! typed canonical form, in a real caller).

use qsl_foundation::digest::{DigestDomain, DigestRecord};
use serde::Serialize;

/// The limits every simulation preimage encodes under: `quire-canonical`'s
/// own depth ceiling and no byte ceiling of this module's own, matching the
/// established identity-preimage convention (`qsl_semantics::value::
/// semantic_node::IDENTITY_LIMITS`; ADR-013 §2, ADR-013:113).
const LIMITS: quire_canonical::Limits =
    match quire_canonical::Limits::new(u64::MAX, quire_canonical::Limits::MAX_DEPTH) {
        Ok(limits) => limits,
        // `Limits::MAX_DEPTH` is by definition within `Limits::MAX_DEPTH`;
        // this arm is evaluated at compile time and is unreachable.
        Err(_) => panic!("Limits::MAX_DEPTH is within Limits::MAX_DEPTH"),
    };

/// The full canonical state-key bytes a `TransitionSystem` implies for one
/// state, through `quire-canonical`. Two states are the same state to the
/// engine exactly when their full key bytes are equal (FR-101); the digest
/// alone is never compared for coalescing.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub(crate) struct StateKey(Vec<u8>);

impl StateKey {
    /// The wrapped canonical key bytes.
    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

/// `value`'s canonical key bytes and its `quire.simulation.state-key/v1`
/// digest, from one `TransitionSystem::key` result.
///
/// `value` is always a bounded, schema-shaped view a `TransitionSystem`
/// implementer builds fresh for this call, never caller-supplied bytes from
/// outside the process; an RFC 8785 encoding failure is therefore an
/// implementer invariant break, not a request to refuse gracefully --
/// exactly the reasoning `qsl_semantics::model::key::sha256_and_len` already
/// applies to every other typed identity preimage in this codebase.
pub(crate) fn state_key(value: &impl Serialize) -> (StateKey, DigestRecord) {
    let bytes = quire_canonical::to_vec(value, LIMITS)
        .unwrap_or_else(|error| panic!("a simulation state key encodes: {error}"));
    let digest = quire_canonical::sha256(value, LIMITS)
        .unwrap_or_else(|error| panic!("a simulation state key encodes: {error}"));
    (
        StateKey(bytes),
        DigestRecord::mint(DigestDomain::SimulationStateKeyV1, *digest.as_bytes()),
    )
}

/// `value`'s RFC 8785 canonical bytes, used to order transition identities
/// and sampler draw preimages (FR-101). See [`state_key`] on why an encoding
/// failure panics rather than refuses.
pub(crate) fn canonical_bytes(value: &impl Serialize) -> Vec<u8> {
    quire_canonical::to_vec(value, LIMITS)
        .unwrap_or_else(|error| panic!("a simulation preimage encodes: {error}"))
}

/// The SHA-256 digest of `value`'s RFC 8785 bytes, with no domain label in
/// the preimage (the sampler draw digest, FR-101). See [`state_key`] on why
/// an encoding failure panics rather than refuses.
pub(crate) fn plain_digest(value: &impl Serialize) -> [u8; 32] {
    *quire_canonical::sha256(value, LIMITS)
        .unwrap_or_else(|error| panic!("a simulation preimage encodes: {error}"))
        .as_bytes()
}

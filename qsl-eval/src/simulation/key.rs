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
use qsl_foundation::ByteDigest;
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

/// A `TransitionSystem::Key` or `TransitionId` has no RFC 8785 encoding
/// under `quire-canonical` (FR-101-AC-11): for example an integer outside
/// the exact-double range, a non-finite float, a non-string map key, or
/// nesting past `quire_canonical::Limits::MAX_DEPTH`. `TransitionSystem` is
/// a public trait any downstream crate implements, so this is a caller
/// defect the engine refuses, not an internal invariant break: unlike the
/// sampler's own draw preimage (bounded decimal strings this crate builds
/// itself), a `Key` or `TransitionId` is implementer-supplied.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("no RFC 8785 encoding: {0}")]
pub struct EncodingRefusal(String);

/// `value`'s canonical key bytes and its `quire.simulation.state-key/v1`
/// digest, from one `TransitionSystem::key` result. The digest hashes the
/// already-produced canonical bytes directly (`ByteDigest::of`), rather
/// than re-encoding `value` a second time, exactly as
/// `qsl_semantics::model::key::sha256_and_len` hashes its own preimage
/// bytes once.
///
/// # Errors
///
/// [`EncodingRefusal`] when `value` has no RFC 8785 encoding.
pub(crate) fn state_key(
    value: &impl Serialize,
) -> Result<(StateKey, DigestRecord), EncodingRefusal> {
    let bytes = quire_canonical::to_vec(value, LIMITS)
        .map_err(|error| EncodingRefusal(error.to_string()))?;
    let digest = ByteDigest::of(&bytes);
    Ok((
        StateKey(bytes),
        DigestRecord::mint(DigestDomain::SimulationStateKeyV1, digest.as_bytes()),
    ))
}

/// `value`'s RFC 8785 canonical bytes, used to order transition identities
/// (FR-101).
///
/// # Errors
///
/// [`EncodingRefusal`] when `value` has no RFC 8785 encoding.
pub(crate) fn canonical_bytes(value: &impl Serialize) -> Result<Vec<u8>, EncodingRefusal> {
    quire_canonical::to_vec(value, LIMITS).map_err(|error| EncodingRefusal(error.to_string()))
}

/// The SHA-256 digest of `value`'s RFC 8785 bytes, with no domain label in
/// the preimage (the sampler draw digest, FR-101). `value` is always a
/// `DrawPreimage` this crate builds itself from bounded decimal strings, so
/// -- unlike [`state_key`] and [`canonical_bytes`] -- an encoding failure
/// here is an internal invariant break, not an implementer defect: exactly
/// the reasoning `qsl_semantics::model::key::sha256_and_len` applies to
/// every other engine-built identity preimage in this codebase.
pub(crate) fn plain_digest(value: &impl Serialize) -> [u8; 32] {
    *quire_canonical::sha256(value, LIMITS)
        .unwrap_or_else(|error| panic!("a sampler draw preimage encodes: {error}"))
        .as_bytes()
}

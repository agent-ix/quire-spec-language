// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101: the simulator state key, its typed canonical form and its digest.
//!
//! A `TransitionSystem` hands the engine a `quire_canonical::Encode` view of
//! a state (or a transition identity); this module is the one place that turns that view
//! into RFC 8785 JCS bytes and, for a state, its `quire.simulation.state-key/
//! v1` digest. It never interprets the value beyond that: what the view
//! contains is the `TransitionSystem` implementer's concern (QSpec FR-181's
//! typed canonical form, in a real caller).

use qsl_foundation::diagnostic::LimitExceeded;
use qsl_foundation::digest::{DigestDomain, DigestRecord};
use qsl_foundation::{ByteDigest, IdentityLimits, Setting};
use quire_canonical::{Encode, Limits};

/// The limit the sampler's own draw preimage encodes under. A draw preimage
/// is five bounded decimal strings this crate builds itself, so it needs no
/// caller limit.
const DRAW_LIMITS: Limits = Limits::new(u64::MAX);

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

/// A `TransitionSystem::Key` or `TransitionId` was not encoded under
/// `quire-canonical`: it has no RFC 8785 encoding (FR-101-AC-11), or its
/// canonical bytes reached the caller's `identity.input_bytes` limit
/// (FR-259 B4). `TransitionSystem` is a public trait any downstream crate
/// implements, so both are caller defects the engine refuses, not internal
/// invariant breaks: unlike the sampler's own draw preimage (bounded decimal
/// strings this crate builds itself), a `Key` or `TransitionId` is
/// implementer-supplied.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum EncodingRefusal {
    /// No RFC 8785 encoding exists for the value: for example an integer
    /// outside the exact-double range, a non-finite float, a non-string map
    /// key, or events written out of order.
    #[error("no RFC 8785 encoding: {0}")]
    NoEncoding(String),
    /// The value's canonical bytes would exceed `identity.input_bytes`.
    #[error("{0}")]
    Limit(LimitExceeded),
    /// A heap reservation for the value's canonical bytes failed (FR-259
    /// Behavior 6): `resource_exhausted`/`allocation-failed`, with no limit
    /// and no setting, never a value with no encoding.
    #[error("resource_exhausted/allocation-failed: {requested} bytes")]
    Allocation {
        /// The size in bytes of the reservation that failed.
        requested: usize,
    },
}

impl EncodingRefusal {
    /// `error` from encoding under `identity`.
    fn of(error: &quire_canonical::Error, identity: IdentityLimits) -> Self {
        match error {
            quire_canonical::Error::Limit(limit) => Self::Limit(LimitExceeded::new(
                Setting::IdentityInputBytes,
                identity.input_bytes,
                u128::from(limit.required),
            )),
            quire_canonical::Error::Allocation { requested } => Self::Allocation {
                requested: *requested,
            },
            other => Self::NoEncoding(other.to_string()),
        }
    }
}

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
    value: &impl Encode,
    identity: IdentityLimits,
) -> Result<(StateKey, DigestRecord), EncodingRefusal> {
    let bytes = quire_canonical::to_vec(value, Limits::new(identity.input_bytes))
        .map_err(|error| EncodingRefusal::of(&error, identity))?;
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
pub(crate) fn canonical_bytes(
    value: &impl Encode,
    identity: IdentityLimits,
) -> Result<Vec<u8>, EncodingRefusal> {
    quire_canonical::to_vec(value, Limits::new(identity.input_bytes))
        .map_err(|error| EncodingRefusal::of(&error, identity))
}

/// The SHA-256 digest of `value`'s RFC 8785 bytes, with no domain label in
/// the preimage (the sampler draw digest, FR-101). `value` is always a
/// `DrawPreimage` this crate builds itself: five decimal strings of `u64`
/// values, at most 20 digits each, so its canonical form is under 200 bytes
/// and is encoded with no byte bound of its own. Unlike [`state_key`]
/// and [`canonical_bytes`], whose values an implementer supplies, an
/// encoding failure here is an internal invariant break.
pub(crate) fn plain_digest(value: &impl Encode) -> [u8; 32] {
    *quire_canonical::sha256(value, DRAW_LIMITS)
        .unwrap_or_else(|error| panic!("a sampler draw preimage encodes: {error}"))
        .as_bytes()
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_canonical::{Encode, Sink, Writer};

    use super::*;

    /// A recursive list `[1, [2, [3, ...]]]` `depth` levels deep, written as
    /// events from a counter, never as a nested value.
    struct NestedList {
        depth: usize,
    }

    impl Encode for NestedList {
        fn encode_into<S: Sink + ?Sized>(
            &self,
            writer: &mut Writer<'_, S>,
        ) -> Result<(), quire_canonical::Error> {
            for level in 0..self.depth {
                writer.begin_array()?;
                writer.integer(i128::try_from(level).unwrap_or_default())?;
            }
            for _ in 0..self.depth {
                writer.end_array()?;
            }
            Ok(())
        }
    }

    fn on_stack<T: Send + 'static>(bytes: usize, run: impl FnOnce() -> T + Send + 'static) -> T {
        std::thread::Builder::new()
            .stack_size(bytes)
            .spawn(run)
            .unwrap()
            .join()
            .unwrap()
    }

    /// FR-259-AC-3: the state key of a 100,000-long recursive list value,
    /// minted on a 512 KiB stack, equals the one minted on an 8 MiB stack.
    #[trace("TC-729", "FR-259-AC-3")]
    #[test]
    fn a_state_key_of_a_100_000_long_recursive_list_needs_no_deep_stack() {
        let mint = || {
            let (key, digest) =
                state_key(&NestedList { depth: 100_000 }, IdentityLimits::default()).unwrap();
            (key, digest)
        };
        let small = on_stack(512 * 1024, mint);
        let large = on_stack(8 * 1024 * 1024, mint);
        assert_eq!(small, large);
        assert!(small.0.as_bytes().starts_with(b"[0,["));
    }

    /// FR-255-AC-1 for `identity.input_bytes`: a state key past the caller's
    /// bound refuses naming the setting, the bound and the length reached,
    /// and the same value encodes once the bound is raised through the
    /// limits' builder.
    #[trace("TC-720", "FR-255-AC-1")]
    #[trace("TC-721", "FR-255-AC-4")]
    #[test]
    fn a_state_key_past_identity_input_bytes_names_the_setting() {
        let value = NestedList { depth: 4 };
        let exact = quire_canonical::to_vec(&value, DRAW_LIMITS).unwrap().len();
        let bound = u64::try_from(exact - 1).unwrap();
        let tight = IdentityLimits::default().with_input_bytes(bound);
        let EncodingRefusal::Limit(limit) = state_key(&value, tight).unwrap_err() else {
            panic!("a limit refusal, not a missing encoding");
        };
        assert_eq!(limit.setting(), Setting::IdentityInputBytes);
        assert_eq!(limit.configured_bound(), bound);
        assert!(limit.actual() > u128::from(bound));
        assert!(state_key(&value, tight.with_input_bytes(bound + 1)).is_ok());
    }

    /// FR-259 Behavior 6: a failed reservation for a key's bytes refuses as
    /// `resource_exhausted`/`allocation-failed` carrying the size, never as
    /// a value with no encoding.
    #[test]
    fn a_failed_reservation_is_allocation_failed_not_no_encoding() {
        let refusal = EncodingRefusal::of(
            &quire_canonical::Error::Allocation { requested: 4096 },
            IdentityLimits::default(),
        );
        assert_eq!(refusal, EncodingRefusal::Allocation { requested: 4096 });
        let code = crate::simulation::NotSimulated::KeyEncoding(refusal)
            .catalog_code()
            .expect("an allocation failure has a catalog code");
        assert_eq!(code.code(), "resource_exhausted");
        assert_eq!(code.cause(), "allocation-failed");
    }
}

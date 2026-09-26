// SPDX-License-Identifier: AGPL-3.0-or-later
//! Seeded sampling over the same successor relation the exploration engine
//! walks, with the pinned `quire.simulation.sampler/v1` `1-draft.1`
//! generator (FR-101; QSpec `proposals/quire-v1/definitions/
//! simulation-sampler.md`).

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::selection::DefinitionRef;
use qsl_semantics::value::declaration::TypeEnvironment;
use quire_exact::ValueType;
use serde::Serialize;

use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::plain_digest;
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated};
use crate::simulation::order::{ordered_successors, sorted_initial};
use crate::simulation::trace::{SampleProvenance, Step, StopReason, Trace};

/// The one generator `sample_request` runs (FR-101).
const SAMPLER_IDENTITY: &str = "quire.simulation.sampler/v1";
/// See [`SAMPLER_IDENTITY`].
const SAMPLER_REVISION: &str = "1-draft.1";

/// A source of draws for sampled exploration.
///
/// A sampler is deterministic in the sense the engine requires: the same
/// sampler state, driven the same way, produces the same sequence of
/// indices.
pub(crate) trait Sampler {
    /// Draw an index in `0..n`. `n` is always at least 1; the caller never
    /// asks a sampler to choose among zero candidates.
    fn next_index(&mut self, n: usize) -> usize;
}

/// An unsigned 256-bit integer, big-endian limbs, wide enough to hold one
/// SHA-256 digest read as a number (FR-101's `v`).
#[derive(Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct U256([u64; 4]);

impl U256 {
    const MAX: Self = Self([u64::MAX; 4]);

    fn from_be_bytes(bytes: [u8; 32]) -> Self {
        let mut limbs = [0u64; 4];
        for (index, limb) in limbs.iter_mut().enumerate() {
            let start = index * 8;
            let mut octet = [0u8; 8];
            octet.copy_from_slice(&bytes[start..start + 8]);
            *limb = u64::from_be_bytes(octet);
        }
        Self(limbs)
    }

    /// `(self / n, self % n)`, by schoolbook long division limb by limb,
    /// most significant first. Each step's partial remainder stays below
    /// `n`, so `remainder << 64 | limb` always fits `u128` exactly (at most
    /// `(n - 1) * 2^64 + (2^64 - 1) == u128::MAX`).
    fn divmod_u64(self, n: u64) -> (Self, u64) {
        let mut quotient = [0u64; 4];
        let mut remainder: u64 = 0;
        for (index, limb) in self.0.iter().enumerate() {
            let dividend = (u128::from(remainder) << 64) | u128::from(*limb);
            let n128 = u128::from(n);
            quotient[index] = (dividend / n128) as u64;
            remainder = (dividend % n128) as u64;
        }
        (Self(quotient), remainder)
    }

    /// `self * n`, valid only when the true product is representable in 256
    /// bits (the one call site below establishes this before calling).
    fn mul_u64(self, n: u64) -> Self {
        let mut limbs = [0u64; 4];
        let mut carry: u128 = 0;
        for index in (0..4).rev() {
            let product = u128::from(self.0[index]) * u128::from(n) + carry;
            limbs[index] = product as u64;
            carry = product >> 64;
        }
        Self(limbs)
    }
}

/// The pinned `quire.simulation.sampler/v1` `1-draft.1` generator: a
/// counter-mode SHA-256 rejection sampler over `{seed, trace, step, draw}`.
pub(crate) struct PinnedSampler {
    seed: u64,
    trace: u64,
    step: u64,
    /// How many draw digests this sampler has computed so far (TC-454
    /// step 3b): `n = 1` never computes one, so this is the observable
    /// signal that a call actually drew rather than taking the no-digest
    /// shortcut. Test-only instrumentation, not part of the pinned
    /// generator's behaviour.
    #[cfg(test)]
    digests_computed: u64,
}

/// One draw's JCS preimage: `{"draw":"<decimal>","seed":"<decimal>",
/// "step":"<decimal>","trace":"<decimal>"}`.
#[derive(Serialize)]
struct DrawPreimage {
    draw: String,
    seed: String,
    step: String,
    trace: String,
}

impl PinnedSampler {
    pub(crate) fn new(seed: u64, trace: u64) -> Self {
        Self {
            seed,
            trace,
            step: 0,
            #[cfg(test)]
            digests_computed: 0,
        }
    }

    /// How many draw digests this sampler has computed so far.
    #[cfg(test)]
    pub(crate) fn digests_computed(&self) -> u64 {
        self.digests_computed
    }
}

impl Sampler for PinnedSampler {
    fn next_index(&mut self, n: usize) -> usize {
        let step = self.step;
        self.step = self.step.wrapping_add(1);
        if n == 1 {
            return 0;
        }
        // `n` is a successor count: a `Vec::len()`, always representable in
        // `u64` on every target Rust supports.
        let n64 = n as u64;
        // `floor(2^256 / n)`: `floor((MAX + 1) / n)`, which is
        // `floor(MAX / n)` unless `n` divides `MAX + 1` exactly (i.e. `MAX %
        // n == n - 1`), in which case it is one more and the threshold below
        // is exactly `2^256` -- unrepresentable in 256 bits, so that case
        // always accepts instead of multiplying. Fixed for this draw's `n`,
        // so computed once, not per rejected draw.
        let (quotient, remainder) = U256::MAX.divmod_u64(n64);
        let always_accepts = remainder == n64 - 1;
        let mut draw: u64 = 0;
        loop {
            let preimage = DrawPreimage {
                draw: draw.to_string(),
                seed: self.seed.to_string(),
                step: step.to_string(),
                trace: self.trace.to_string(),
            };
            let digest = plain_digest(&preimage);
            #[cfg(test)]
            {
                self.digests_computed += 1;
            }
            let v = U256::from_be_bytes(digest);
            let (_, v_remainder) = v.divmod_u64(n64);
            if always_accepts || v < quotient.mul_u64(n64) {
                // `v_remainder < n64 == n as u64`, so it fits `usize`.
                return v_remainder as usize;
            }
            draw += 1;
        }
    }
}

/// Draw one sampled run from `system`, starting at trace `trace_index mod m`
/// among `system`'s `m` distinct initial states (no draw selects the
/// start), stopping after `max_steps` transitions or at the first state
/// with no successors, whichever comes first.
///
/// Returns `None` when `system.initial()` returns no states.
pub(crate) fn sample<S: TransitionSystem>(
    system: &S,
    sampler: DefinitionRef,
    seed: u64,
    trace_index: u64,
    max_steps: usize,
) -> Option<Trace<S::TransitionId>> {
    let initial = sorted_initial(system);
    if initial.is_empty() {
        return None;
    }
    let m = initial.len() as u64;
    let start = (trace_index % m) as usize;
    let chosen = initial.into_iter().nth(start)?;
    let mut current = chosen.state;
    let initial_digest = chosen.digest;

    let mut generator = PinnedSampler::new(seed, trace_index);
    let mut steps = Vec::new();
    let mut stopped = StopReason::StepLimit;
    for _ in 0..max_steps {
        let successors = ordered_successors(system, &current);
        if successors.is_empty() {
            stopped = StopReason::NoSuccessors;
            break;
        }
        let n = successors.len();
        let index = generator.next_index(n);
        let Some(chosen) = successors.into_iter().nth(index) else {
            // `index < n`, by `Sampler::next_index`'s own contract.
            unreachable!("sampler drew an index within 0..n");
        };
        current = chosen.state;
        steps.push(Step {
            transition: chosen.transition,
            key: chosen.digest,
        });
    }

    Some(Trace {
        initial: initial_digest,
        steps,
        provenance: Some(SampleProvenance {
            seed,
            trace: trace_index,
            sampler,
            stopped,
        }),
    })
}

/// Sample `system`, first refusing a `sampler` other than the pinned
/// `quire.simulation.sampler/v1` `1-draft.1` generator and an unbounded
/// `domains` request, before any `TransitionSystem` method is called or any
/// draw runs (FR-101).
///
/// `sample` and `Sampler` stay `pub(crate)`; this and `explore_request` are
/// the only entries that explore or sample.
///
/// # Errors
///
/// [`NotSimulated::GeneratorMismatch`] when `sampler`'s identity or version
/// is not the pinned generator's; [`NotSimulated::RequiresBound`] or
/// [`NotSimulated::Extent`] as `explore_request`; [`NotSimulated::EmptyInitial`]
/// when `system.initial()` returns no states.
#[allow(
    clippy::too_many_arguments,
    reason = "FR-101 pins this exact signature; the parameters are the request's own fields, not an accretion of unrelated flags"
)]
pub fn sample_request<S: TransitionSystem>(
    system: &S,
    domains: &[(WireNodeId, &ValueType)],
    types: &TypeEnvironment,
    position_limit: u64,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    max_steps: usize,
) -> Result<Trace<S::TransitionId>, NotSimulated> {
    if sampler.identity() != SAMPLER_IDENTITY || sampler.version() != SAMPLER_REVISION {
        return Err(NotSimulated::GeneratorMismatch {
            supplied: sampler.clone(),
        });
    }
    check_requires_bound(domains, types, position_limit)?;
    sample(system, sampler.clone(), seed, trace, max_steps).ok_or(NotSimulated::EmptyInitial)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// TC-454 step 1b: the pinned sampler's step-0 preimage bytes and digest
    /// at seed 424242, trace 0, `n = 5` -- QSpec TC-210's own vector.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_step_0_preimage_and_digest_match_the_pinned_vector() {
        let preimage = DrawPreimage {
            draw: "0".to_owned(),
            seed: "424242".to_owned(),
            step: "0".to_owned(),
            trace: "0".to_owned(),
        };
        let limits =
            quire_canonical::Limits::new(u64::MAX, quire_canonical::Limits::MAX_DEPTH).unwrap();
        let bytes = quire_canonical::to_vec(&preimage, limits).expect("a draw preimage encodes");
        assert_eq!(
            bytes,
            br#"{"draw":"0","seed":"424242","step":"0","trace":"0"}"#
        );
        let digest = plain_digest(&preimage);
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            hex,
            "cb7d4b3b8b8491d8310ccc1e07c3ee236d3fe86cf713d1851533a85ef6682622"
        );
    }

    /// TC-454 step 3b: the sampler computes no digest at all over a chain
    /// where every state has exactly one successor (`n = 1` throughout).
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_one_successor_chain_computes_no_digest() {
        let mut sampler = PinnedSampler::new(424_242, 0);
        for _ in 0..5 {
            assert_eq!(sampler.next_index(1), 0);
        }
        assert_eq!(sampler.digests_computed(), 0);

        // A draw with more than one candidate does compute at least one
        // digest, confirming the counter is wired to the real draw path,
        // not merely never incremented.
        sampler.next_index(5);
        assert!(sampler.digests_computed() > 0);
    }
}

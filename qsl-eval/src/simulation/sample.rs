// SPDX-License-Identifier: AGPL-3.0-or-later
//! Seeded sampling over the same successor relation the exploration engine
//! walks, with the `quire.simulation.sampler/v1`
//! generator (FR-101; QSpec `proposals/quire-v1/definitions/
//! simulation-sampler.md`).

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::selection::DefinitionRef;
use quire_exact::ValueType;
use quire_semantic_value::declaration::TypeEnvironment;

use crate::simulation::choice::{choose, ChoicePoint, Weights};
use crate::simulation::expansion::StateFindings;
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::EncodingRefusal;
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated};
use crate::simulation::order::{expand, sorted_initial, Expanded};
use crate::simulation::trace::{SampleProvenance, Step, StopReason, Trace};
use qsl_foundation::IdentityLimits;

/// The one generator `sample_request` runs (FR-101).
const SAMPLER_IDENTITY: &str = "quire.simulation.sampler/v1";

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

/// The `quire.simulation.sampler/v1` generator without a workload: each step
/// makes one uniform choice over its enabled successors, at choice index 0
/// (QSpec FR-181), which is FR-188's weighted choice over unit weights.
pub(crate) struct PinnedSampler {
    seed: u64,
    trace: u64,
    step: u64,
    /// The last step's uniform weights, kept while the successor count
    /// stays the same, so a step does not recompute `⌊2^256 / n⌋`.
    uniform: Option<Weights>,
    /// How many draw digests this sampler has computed so far (TC-454
    /// step 3b): `n = 1` never computes one, so this is the observable
    /// signal that a call actually drew rather than taking the no-digest
    /// shortcut. Test-only instrumentation, not part of the pinned
    /// generator's behaviour.
    #[cfg(test)]
    digests_computed: u64,
}

impl PinnedSampler {
    pub(crate) fn new(seed: u64, trace: u64) -> Self {
        Self {
            seed,
            trace,
            step: 0,
            uniform: None,
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
        let weights = match self.uniform.take() {
            Some(weights) if weights.len() == n => weights,
            _ => Weights::uniform(n).expect("next_index is called with n >= 1"),
        };
        let at = ChoicePoint {
            seed: self.seed,
            trace: self.trace,
            step,
            choice: 0,
        };
        let (index, _digests) = choose(at, &weights);
        self.uniform = Some(weights);
        #[cfg(test)]
        {
            self.digests_computed += _digests;
        }
        index
    }
}

/// [`sample`]'s result: the trace, `None` for a system with no initial
/// state, or the first encoding refusal reached.
type SampleResult<T, F> = Result<Option<Trace<T, F>>, EncodingRefusal>;

/// Draw one sampled run from `system`, starting at trace `trace_index mod m`
/// among `system`'s `m` distinct initial states (no draw selects the
/// start). The run ends at the first state where `end` holds, unexpanded
/// ([`StopReason::Reached`]); at the first state whose expansion stops;
/// after `max_steps` transitions; or at the first state with no successors,
/// checked in that order. Every other state on the trace is expanded, the
/// last included, and its findings recorded at its step index.
///
/// Returns `Ok(None)` when `system.initial()` returns no states.
///
/// # Errors
///
/// [`EncodingRefusal`] when a state or transition identity reached during
/// the run has no RFC 8785 encoding.
pub(crate) fn sample<S: TransitionSystem>(
    system: &S,
    sampler: DefinitionRef,
    seed: u64,
    trace_index: u64,
    max_steps: usize,
    identity: IdentityLimits,
    mut end: impl FnMut(&S::State) -> bool,
) -> SampleResult<S::TransitionId, S::Finding> {
    let mut initial = sorted_initial(system, identity)?;
    if initial.is_empty() {
        return Ok(None);
    }
    let m = initial.len() as u64;
    let start = (trace_index % m) as usize;
    // `start < m == initial.len()`, so `swap_remove` never panics.
    let chosen = initial.swap_remove(start);
    let mut current = chosen.state;
    let mut current_digest = chosen.digest;
    let initial_digest = chosen.digest;

    let mut generator = PinnedSampler::new(seed, trace_index);
    let mut steps = Vec::new();
    let mut findings = Vec::new();
    let stopped = loop {
        if end(&current) {
            break StopReason::Reached;
        }
        let (mut successors, state_findings) = match expand(system, &current, identity)? {
            Expanded::Successors {
                successors,
                findings,
            } => (successors, findings),
            Expanded::Stopped(stop) => break StopReason::Stopped(stop.cause),
        };
        if !state_findings.is_empty() {
            findings.push(StateFindings {
                state: current_digest,
                depth: steps.len(),
                findings: state_findings,
            });
        }
        if steps.len() >= max_steps {
            break StopReason::StepLimit;
        }
        if successors.is_empty() {
            break StopReason::NoSuccessors;
        }
        let n = successors.len();
        // `next_index(n)` always returns an index `< n` (its own contract),
        // so `swap_remove` never panics: no fallback branch is needed.
        let index = generator.next_index(n);
        let chosen = successors.swap_remove(index);
        current = chosen.state;
        current_digest = chosen.digest;
        steps.push(Step {
            transition: chosen.transition,
            key: chosen.digest,
        });
    };

    Ok(Some(Trace {
        initial: initial_digest,
        steps,
        findings,
        provenance: Some(SampleProvenance {
            seed,
            trace: trace_index,
            sampler,
            stopped,
        }),
    }))
}

/// Whether `sampler` is the one `quire.simulation.sampler/v1` generator
/// (FR-101), by identity. The only site that compares a sampler's identity
/// string.
#[qsl_attrs::string_edge]
pub(crate) fn is_pinned_sampler(sampler: &DefinitionRef) -> bool {
    sampler.identity() == SAMPLER_IDENTITY
}

/// Sample `system`, first refusing a `sampler` other than the
/// `quire.simulation.sampler/v1` generator and an unbounded
/// `domains` request, before any `TransitionSystem` method is called or any
/// draw runs (FR-101).
///
/// `sample` and `Sampler` stay `pub(crate)`; this and `explore_request` are
/// the only entries that explore or sample.
///
/// # Errors
///
/// [`NotSimulated::GeneratorMismatch`] when `sampler`'s identity is not the
/// generator's; [`NotSimulated::RequiresBound`] or
/// [`NotSimulated::Extent`] as `explore_request`; [`NotSimulated::EmptyInitial`]
/// when `system.initial()` returns no states; [`NotSimulated::KeyEncoding`]
/// when a state or transition identity reached during the run has no RFC
/// 8785 encoding.
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
    identity: IdentityLimits,
) -> Result<Trace<S::TransitionId, S::Finding>, NotSimulated> {
    sample_request_until(
        system,
        SampleRequest {
            domains,
            types,
            position_limit,
            sampler,
            seed,
            trace,
            max_steps,
            identity,
        },
        |_| false,
    )
}

/// [`sample_request`]'s inputs.
pub struct SampleRequest<'r> {
    /// The requires-bound roots.
    pub domains: &'r [(WireNodeId, &'r ValueType)],
    /// The type environment the roots are classified under.
    pub types: &'r TypeEnvironment,
    /// The requires-bound pre-check's position budget.
    pub position_limit: u64,
    /// The sampler's `DefinitionRef`.
    pub sampler: &'r DefinitionRef,
    /// The run's seed.
    pub seed: u64,
    /// The trace's index within the run.
    pub trace: u64,
    /// The most steps the trace takes.
    pub max_steps: usize,
    /// The identity limits states and transitions encode under.
    pub identity: IdentityLimits,
}

/// [`sample_request`], ending the trace at the first state, position 0
/// included, where `end` holds, before that state is expanded:
/// [`StopReason::Reached`]. The draws are those of `sample_request` for the
/// same seed and trace index, so the trace is the prefix of the one
/// `sample_request` draws, up to that state (FR-166: a phase 0 walk ends at
/// its first target state).
///
/// # Errors
///
/// As [`sample_request`].
pub fn sample_request_until<S: TransitionSystem>(
    system: &S,
    request: SampleRequest<'_>,
    end: impl FnMut(&S::State) -> bool,
) -> Result<Trace<S::TransitionId, S::Finding>, NotSimulated> {
    if !is_pinned_sampler(request.sampler) {
        return Err(NotSimulated::GeneratorMismatch {
            supplied: request.sampler.clone(),
        });
    }
    check_requires_bound(request.domains, request.types, request.position_limit)?;
    sample(
        system,
        request.sampler.clone(),
        request.seed,
        request.trace,
        request.max_steps,
        request.identity,
        end,
    )?
    .ok_or(NotSimulated::EmptyInitial)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::simulation::choice::DrawPreimage;
    use crate::simulation::key::plain_digest;

    /// TC-454 step 1b: the sampler's step-0 preimage bytes and digest at
    /// seed 424242, trace 0, choice 0, `n = 5` -- QSpec TC-210's vector
    /// under FR-181's `choice` preimage member.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_step_0_preimage_and_digest_match_the_pinned_vector() {
        let preimage = DrawPreimage {
            choice: "0".to_owned(),
            draw: "0".to_owned(),
            seed: "424242".to_owned(),
            step: "0".to_owned(),
            trace: "0".to_owned(),
        };
        let bytes = quire_canonical::to_vec(&preimage, quire_canonical::Limits::new(u64::MAX))
            .expect("a draw preimage encodes");
        assert_eq!(
            bytes,
            br#"{"choice":"0","draw":"0","seed":"424242","step":"0","trace":"0"}"#
        );
        let digest = plain_digest(&preimage);
        let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
        assert_eq!(
            hex,
            "d5160380d7495443315376de306a5d3613f3e010df74a5db92853829205995f2"
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

    /// SR-672 FND-005: `n = 3` and `n = 5` alone cannot tell a big-endian
    /// reading of the draw digest from a little-endian one, because
    /// `256 ≡ 1 (mod 3)` and `256 ≡ 1 (mod 5)` make `v mod n` invariant
    /// under byte reversal for those two moduli specifically. `n = 2` and
    /// `n = 7` do not share that property, and a little-endian reading
    /// gives a different sequence at each (recomputed independently in
    /// Python with `hashlib`, under the `choice` preimage member): seed
    /// 424242, trace 0, `n = 2` is `0, 1, 0, 1, 1` (little-endian
    /// `1, 0, 1, 1, 0`), and `n = 7` is `6, 2, 6, 6, 1` (little-endian
    /// `1, 5, 6, 5, 2`).
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_n2_and_n7_vectors_distinguish_byte_order() {
        let mut two = PinnedSampler::new(424_242, 0);
        let indices: Vec<usize> = (0..5).map(|_| two.next_index(2)).collect();
        assert_eq!(indices, vec![0, 1, 0, 1, 1]);

        let mut seven = PinnedSampler::new(424_242, 0);
        let indices: Vec<usize> = (0..5).map(|_| seven.next_index(7)).collect();
        assert_eq!(indices, vec![6, 2, 6, 6, 1]);
    }
}

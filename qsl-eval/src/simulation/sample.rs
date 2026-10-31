// SPDX-License-Identifier: AGPL-3.0-or-later
//! Seeded sampling over the same successor relation the exploration engine
//! walks, with the `quire.simulation.sampler/v1`
//! generator (FR-101; QSpec `proposals/quire-v1/definitions/
//! simulation-sampler.md`).

use qsl_foundation::selection::DefinitionRef;
use quire_exact::NodeKey;
use quire_exact::ValueType;
use quire_semantic_value::declaration::TypeEnvironment;

use crate::simulation::choice::{choose, ChoicePoint, Weights};
use crate::simulation::expansion::StateFindings;
use crate::simulation::explore::TransitionSystem;
use crate::simulation::key::EncodingRefusal;
use crate::simulation::not_simulated::{check_requires_bound, NotSimulated, Root};
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
    let mut current_key = chosen.key.clone();
    let initial_key = chosen.key;

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
                state: current_key.clone(),
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
        // The ordinal is counted before `swap_remove` reorders `successors`.
        let ordinal = successors[..index]
            .iter()
            .filter(|earlier| earlier.transition == successors[index].transition)
            .count();
        let chosen = successors.swap_remove(index);
        current = chosen.state;
        current_key = chosen.key.clone();
        steps.push(Step {
            transition: chosen.transition,
            ordinal,
            key: chosen.key,
        });
    };

    Ok(Some(Trace {
        initial: initial_key,
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
#[ix_trace_rs::string_edge]
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
pub fn sample_request<S: TransitionSystem, R: Root>(
    system: &S,
    domains: &[R],
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
pub struct SampleRequest<'r, R: Root = (NodeKey, &'r ValueType)> {
    /// The requires-bound roots.
    pub domains: &'r [R],
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
pub fn sample_request_until<S: TransitionSystem, R: Root>(
    system: &S,
    request: SampleRequest<'_, R>,
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

    /// SHA-256 of `bytes`, by `sha2` directly: a path independent of
    /// `quire_canonical`, which the sampler's own digest goes through.
    fn independent_sha256(bytes: &[u8]) -> [u8; 32] {
        use sha2::{Digest, Sha256};
        Sha256::digest(bytes).into()
    }

    /// The preimage JSON of a draw, written by hand from FR-181's member list
    /// (sorted keys, decimal strings), independent of the serializer.
    fn written_preimage(seed: u64, trace: u64, step: u64, choice: u32, draw: u64) -> String {
        format!(
            r#"{{"choice":"{choice}","draw":"{draw}","seed":"{seed}","step":"{step}","trace":"{trace}"}}"#
        )
    }

    /// The draw digest's big-endian 256-bit value reduced mod `n`, folded a
    /// byte at a time with no big-integer type.
    fn reduce_big_endian(digest: &[u8; 32], n: usize) -> usize {
        digest
            .iter()
            .fold(0_usize, |rest, byte| (rest * 256 + usize::from(*byte)) % n)
    }

    /// The sampler's first `steps` indices over `n` candidates.
    fn indices(seed: u64, trace: u64, n: usize, steps: usize) -> Vec<usize> {
        let mut sampler = PinnedSampler::new(seed, trace);
        (0..steps).map(|_| sampler.next_index(n)).collect()
    }

    /// QSpec's TC-210 "Sampler index-sequence vector" section, read from the
    /// checkout `QSPEC_DIR` names; `None` when it is unset (`make
    /// conformance` requires it). Nothing from it is copied into QSL.
    fn tc_210_section() -> Option<String> {
        let qspec = std::env::var_os("QSPEC_DIR")?;
        let path = std::path::Path::new(&qspec)
            .join("spec/test-cases/TC-210-finite-simulation-reproducibility.md");
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("{} is readable: {error}", path.display()));
        let start = text
            .find("## Sampler index-sequence vector")
            .expect("TC-210 has the sampler index-sequence section");
        let section = &text[start..];
        let end = section[3..]
            .find("\n## ")
            .map_or(section.len(), |at| at + 3);
        Some(section[..end].to_owned())
    }

    /// TC-454 step 1b: the sampler's step-0 preimage bytes and digest at
    /// seed 424242, trace 0, choice 0 are what the hand-written FR-181
    /// preimage hashes to under an independent SHA-256, and the same seed gives the
    /// same successor sequence.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_step_0_preimage_and_digest_are_deterministic() {
        let preimage = DrawPreimage::at(
            ChoicePoint {
                seed: 424_242,
                trace: 0,
                step: 0,
                choice: 0,
            },
            0,
        );
        let bytes = quire_canonical::to_vec(&preimage, quire_canonical::Limits::new(u64::MAX))
            .expect("a draw preimage encodes");
        let written = written_preimage(424_242, 0, 0, 0, 0);
        assert_eq!(bytes, written.as_bytes());
        assert_eq!(
            plain_digest(&preimage),
            independent_sha256(written.as_bytes())
        );

        assert_eq!(indices(424_242, 0, 5, 8), indices(424_242, 0, 5, 8));
    }

    /// TC-454 step 1c: the published index sequence for seed 424242,
    /// trace 0, `n = 5` is the sampler's (read from QSpec's TC-210).
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_published_index_sequence_is_the_samplers() {
        let Some(section) = tc_210_section() else {
            println!("skipped: QSPEC_DIR not set");
            return;
        };
        let published: Vec<usize> = section
            .split("index `")
            .skip(1)
            .map(|rest| {
                rest.split('`')
                    .next()
                    .and_then(|digits| digits.parse().ok())
                    .expect("TC-210 index is a decimal")
            })
            .collect();
        assert_eq!(published.len(), 5, "TC-210 publishes five steps");
        assert_eq!(indices(424_242, 0, 5, published.len()), published);
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

    /// SR-672 FND-005: the draw is the digest read big-endian, reduced mod
    /// `n`. `n = 3` and `n = 5` cannot tell big-endian from little-endian
    /// (`256 ≡ 1 mod 3` and `mod 5` make `v mod n` invariant under byte
    /// reversal), so this is checked at moduli that can, against an
    /// independent fold of the independently hashed preimage, and the two
    /// readings are required to differ somewhere in the sample so the check
    /// can fail on a byte-order flip.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_draw_is_the_big_endian_digest_mod_n() {
        let mut readings_differ = false;
        for n in [2_usize, 4, 7, 9, 11] {
            let got = indices(424_242, 0, n, 32);
            for (step, index) in got.iter().enumerate() {
                let preimage = written_preimage(424_242, 0, step as u64, 0, 0);
                let digest = independent_sha256(preimage.as_bytes());
                assert_eq!(*index, reduce_big_endian(&digest, n), "n={n} step={step}");
                let mut reversed = digest;
                reversed.reverse();
                readings_differ |= reduce_big_endian(&reversed, n) != *index;
            }
        }
        assert!(
            readings_differ,
            "byte order must be observable in the sample"
        );
    }

    /// Same seed and trace give the same sequence; a different seed, trace
    /// or step changes the draw digest (domain separation), and every index
    /// is below `n`.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_sequences_are_deterministic_and_separated() {
        assert_eq!(indices(7, 3, 6, 64), indices(7, 3, 6, 64));
        assert_ne!(indices(7, 3, 6, 64), indices(8, 3, 6, 64));
        assert_ne!(indices(7, 3, 6, 64), indices(7, 4, 6, 64));
        let digests: std::collections::BTreeSet<[u8; 32]> = (0..4_u64)
            .flat_map(|seed| (0..4_u64).map(move |trace| (seed, trace)))
            .flat_map(|(seed, trace)| (0..4_u64).map(move |step| (seed, trace, step)))
            .map(|(seed, trace, step)| {
                plain_digest(&DrawPreimage::at(
                    ChoicePoint {
                        seed,
                        trace,
                        step,
                        choice: 0,
                    },
                    0,
                ))
            })
            .collect();
        assert_eq!(digests.len(), 64, "no two choice points share a draw");
        for n in 1..=12 {
            assert!(indices(1, 1, n, 50).iter().all(|index| *index < n));
        }
    }

    /// Choice behaviour: over many steps every index of `n = 4` is drawn and
    /// none is drawn far from its uniform share.
    #[trace("TC-454", "FR-101-AC-3")]
    #[test]
    fn tc_454_choice_is_uniform_over_the_candidates() {
        let n = 4;
        let draws = 4_000;
        let mut counts = [0_usize; 4];
        for index in indices(99, 0, n, draws) {
            counts[index] += 1;
        }
        let share = draws / n;
        for (index, count) in counts.iter().enumerate() {
            assert!(
                count.abs_diff(share) < share / 10,
                "index {index} drawn {count} times of {draws}"
            );
        }
    }
}

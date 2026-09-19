// SPDX-License-Identifier: AGPL-3.0-or-later
//! Seeded sampling over the same successor relation the exploration engine
//! walks.

use crate::simulation::explore::TransitionSystem;
use crate::simulation::trace::{SampleProvenance, Step, StopReason, Trace};

/// A source of draws for sampled exploration.
///
/// A sampler is deterministic in the sense the engine requires: the same
/// sampler state, driven the same way, produces the same sequence of
/// indices. What makes a draw reproducible across processes (a seed, a
/// counter) is the implementation's concern, not the trait's.
pub trait Sampler {
    /// Draw an index in `0..n`. `n` is always at least 1; the caller never
    /// asks a sampler to choose among zero candidates.
    fn next_index(&mut self, n: usize) -> usize;

    /// The seed this sampler draws from, recorded in every trace it
    /// samples.
    fn seed(&self) -> u64;

    /// The exact identity of this generator, recorded in every trace it
    /// samples so the draw can be attributed to the code that produced it.
    fn version(&self) -> &str;
}

/// A deterministic counter-based `Sampler`, for tests and as a reference
/// implementation.
///
/// This is not the pinned production sampler (`quire.simulation.sampler/v1`,
/// QSpec#129 Q2); `CounterSampler` exists so this slice's tests, and any
/// caller that needs a reproducible draw today, do not have to invent one.
///
/// Each draw mixes the seed with a monotonic counter through SplitMix64, a
/// well-known bijective 64-bit mixer, then reduces the result into `0..n`
/// with `draw % n`. That reduction is biased toward the low end of `0..n`
/// whenever `n` does not evenly divide 2^64; a reference/test generator
/// accepts that bias rather than rejection-sampling it away.
#[derive(Clone, Debug)]
pub struct CounterSampler {
    seed: u64,
    counter: u64,
}

impl CounterSampler {
    /// A sampler that draws deterministically from `seed`.
    pub fn new(seed: u64) -> Self {
        Self { seed, counter: 0 }
    }
}

impl Sampler for CounterSampler {
    fn next_index(&mut self, n: usize) -> usize {
        assert!(n > 0, "next_index requires at least one candidate");
        let draw = split_mix_64(self.seed, self.counter);
        self.counter = self.counter.wrapping_add(1);
        (draw % n as u64) as usize
    }

    fn seed(&self) -> u64 {
        self.seed
    }

    fn version(&self) -> &str {
        "quire.simulation.counter-sampler/1"
    }
}

/// SplitMix64: a fast, deterministic bijective mix of a seed and a counter
/// into a 64-bit draw.
fn split_mix_64(seed: u64, counter: u64) -> u64 {
    let mut z = seed.wrapping_add(counter.wrapping_mul(0x9E37_79B9_7F4A_7C15));
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// `TransitionSystem::initial` returned no states; there is nothing to
/// sample from.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("TransitionSystem::initial returned no states to sample from")]
pub struct EmptyInitial;

/// Draw one sampled run from `system`, stopping after `max_steps`
/// transitions or at the first state with no successors, whichever comes
/// first.
///
/// The returned trace's `provenance` records `sampler`'s `seed()` and
/// `version()`, and which of the two stopping conditions ended the run.
///
/// # Errors
///
/// Returns `EmptyInitial` if `system.initial()` returns no states.
pub fn sample<S: TransitionSystem>(
    system: &S,
    max_steps: usize,
    sampler: &mut impl Sampler,
) -> Result<Trace<S::TransitionId>, EmptyInitial> {
    let mut initials = system.initial();
    if initials.is_empty() {
        return Err(EmptyInitial);
    }
    let index = sampler.next_index(initials.len());
    let mut current = initials.swap_remove(index);
    let initial_key = system.key(&current);

    let mut steps = Vec::new();
    let mut stopped = StopReason::StepLimit;
    for _ in 0..max_steps {
        let mut successors = system.successors(&current);
        if successors.is_empty() {
            stopped = StopReason::NoSuccessors;
            break;
        }
        let index = sampler.next_index(successors.len());
        let (transition, next) = successors.swap_remove(index);
        let key = system.key(&next);
        steps.push(Step { transition, key });
        current = next;
    }

    Ok(Trace {
        initial: initial_key,
        steps,
        provenance: Some(SampleProvenance {
            seed: sampler.seed(),
            sampler_version: sampler.version().to_owned(),
            stopped,
        }),
    })
}

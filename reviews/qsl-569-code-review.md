---
id: SR-12209
title: "Code review (Rust lane) of QSL-569 E17b part 1 FR-188 weighted choice (PR #1060)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@5463c8a4734107683f5bf956d937b217515bcd24; qsl-eval/src/simulation/choice.rs, qsl-eval/src/simulation/mod.rs, qsl-eval/src/simulation/sample.rs, qsl-eval/tests/it/main.rs, qsl-eval/tests/it/weighted_sampler.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: reviews
---
# SR-12209: Code review (Rust lane) of QSL-569 E17b part 1 FR-188 weighted choice (PR #1060)

## Summary

Ticket: QSL-569 (E17b), part 1. PR quire-spec-language#1060 at head 5463c8a47, diff `origin/main...HEAD` only (5 files). Reviewed as code-review with the rust-review lane folded in. Narrow tests run: `cargo test -p qsl-eval --lib simulation::` (22 passed) and `cargo test -p qsl-eval --test it -- weighted_sampler sampl tc_454` (16 passed). No gates were run.

Bit-exactness against the QSpec generator definition (`quire.simulation.sampler/v1`, sections "Digests" and "Uniform and weighted selection"):

- **Preimage.** `DrawPreimage` serialises `{choice, draw, seed, step, trace}` as decimal strings through `plain_digest` (JCS, then SHA-256). The members are already in JCS key order. `draw` starts at 0 for each choice point and increments after every digest, accepted or rejected.
- **`v`.** `big_endian` folds the 32 digest bytes most significant first, so `v` is the big-endian `u256`.
- **Acceptance.** `accept_below = ⌊2^256 / n⌋ · n`, and the test is strictly `v < accept_below`. When `n` divides `2^256` the bound is `2^256` and every draw is accepted. The old `always_accepts` special case is no longer needed because `Integer` is unbounded.
- **Selection.** `v mod n` uses `div_mod_floor` on a non-negative `v`, and `select` returns the least `i` with `v mod n < w_0 + … + w_i`.
- **Scaling.** The LCM of the reduced, positive denominators is folded as `m / gcd(m, d) · d`, and each weight is scaled as `numerator · (m / d)`. This matches "least common multiple of their denominators". quire-exact's `Rational` documents that its denominator is positive and reduced, so the `NotPositive` check on the numerator's sign is complete.
- **`n = 1`.** Selects index 0 with no digest, as the definition states.
- **FR-101 routing.** `PinnedSampler::next_index` is now `choose` over `Weights::uniform(n)` at choice 0. `digests_computed` still counts every digest, rejected ones included. The step counter still advances on `n = 1` steps. The TC-210/TC-454 vectors (`n = 2, 3, 5, 7`; trace 1; the step that computes no digest) pass through the new path unchanged.
- **Termination.** `TotalAboveDrawRange` refuses any `n > 2^256`, the only case in which `⌊2^256 / n⌋ = 0` and the loop would never end. `n = 2^256` is admitted.
- **Reference test.** `weighted_sampler::reference` is written independently of the code: hand-written JCS, `sha2`, `v mod n` folded in `u128`, and the rejection test `v >= 2^256 − (2^256 mod n)` taken through the byte complement. I checked the equivalence: `v ≥ 2^256 − e ⇔ (2^256 − 1) − v < e`. It agrees with the code at 60,000 points for choice 0 and 100 points for choice 1.

The deleted `U256` helper and its two tests were the only users of that arithmetic. The removed path's replacement (`choose`) works and is tested, so nothing is left behind.

No major finding. All three findings are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-101's uniform step draw got slower. Before, it was O(1) `u64` limb arithmetic. Now every `next_index(n)` call allocates a `Vec` of `n` big `Integer` ones, computes `2^256` and `⌊2^256 / n⌋` with big-integer division, and `select` scans all `n` running sums with big-integer additions. The result is still correct, but a step with many enabled successors pays O(n) allocations and big-integer operations. Fix: in `select`, return `remainder` directly for unit weights (or a `Uniform` variant), and cache `Weights::uniform(n)` per `n`, or keep the old `u64` fast path for uniform choices. | qsl-eval/src/simulation/choice.rs:131-133,173-183; qsl-eval/src/simulation/sample.rs:73-83 |
| FND-002 | low | `next_index` hides a broken contract. `let Ok(weights) = Weights::uniform(n) else { return 0; }` returns index 0 for `n = 0`, an index into an empty successor list. Before this PR the same call failed loudly (division by zero). The caller in `sample` returns `NoSuccessors` before it calls, so the path is unreachable today, but a future caller would get a silent wrong index. Fix: use `.expect("next_index is called with n >= 1")` or a `debug_assert!`, as `proven_positive` already does for its own invariant. | qsl-eval/src/simulation/sample.rs:73-76 |
| FND-003 | low | `proven_positive` spells `expect` the long way: `.unwrap_or_else(\|_\| panic!("…"))` on a `Result`. `.expect("a weight denominator, gcd or total is positive")` does the same with a clearer intent. | qsl-eval/src/simulation/choice.rs:189-192 |

## Verdict

**PASS: mergeable. No major or high finding.** The weighted choice is bit-exact to the QSpec generator definition: preimage, big-endian `v`, the `v < n · ⌊2^256 / n⌋` acceptance, least-covering-index selection, LCM scaling, and `n = 1` with no digest. FR-101's uniform draw now goes through it with every existing vector unchanged. A total above `2^256` is refused, so the draw loop always terminates. The three low findings are performance and idiom, and can be fixed in this PR or recorded.

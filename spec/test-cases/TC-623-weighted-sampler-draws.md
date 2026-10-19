---
id: TC-623
title: "The QSpec sampler draws weighted choices exactly and reproducibly"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-188
    type: verifies
---
# TC-623: The QSpec sampler draws weighted choices exactly and reproducibly

## Description

Verify `weighted_choice` against the generator's vectors and FR-101's uniform vectors, the integer scaling, the choice index, and `sample_probabilistic` on `Service`.

Scope: FR-188-AC-1 to FR-188-AC-6.

## Test Procedure

Fixtures: the weighted-sampler conformance vectors of the generator definition; QSpec TC-210's uniform vectors and TC-354's one-item vectors (SM-05, SM-07); `Service` under `Steady`; the non-unique `Health` variant; a test-only digest counter, incremented once per SHA-256 digest the sampler computes and read by the test; three models under a unit workload: `Lone` (object `m`, operation `ping`, no parameters), `Coin` (operation `flip` with `random heads: Bool ~ { true: 1, false: 3 }` (weights `(3, 1)` in canonical value order, `false` first)) and `Wide` (operation `spin` with `random x: Bool ~ { true: 1, false: 115792089237316195423570985008687907853269984665640564039457584007913129639936 }` (that decimal is 2^256)).

1. Run every weighted vector and every TC-210 vector as unit weights at choice index 0; call `sample_probabilistic` with a `DefinitionRef` of another identity.
2. Draw over weights `(1/3, 1/6, 1/2)` at 60,000 trace indices; draw choices 0 and 1 at the first 100 points.
3. Sample `Lone` and `Coin` (seed `424242`, trace 0, step 0); call `weighted_choice` over one weight, over a scaled total of `2^256` and over `(2^256, 1)`; sample `Wide`; check the public items of `qsl-eval` and `quire-semantic-value`.
4. Sample `Service` from seed 7, trace 12, 11 positions, twice; sample the `Health` variant.

Tag the tests `#[trace("TC-623", "FR-188-AC-n")]`.

## Expected Results

- Step 1: every selected index equals the vector's; the sample returns `SampleStop::GeneratorMismatch` with the digest counter at zero.
- Step 2: scaled `(2, 1, 3)`, `N = 6`; every selection equals the reference; choices 0 and 1 differ at some point.
- Step 3: `Lone` selects item 0 with the digest counter at zero and `choices = []`; `Coin`'s choice has index 0, the TC-210 step-0 preimage and `choices = [0]` and `random = [false]`; one weight returns 0 with the digest counter at zero; the total `2^256` selects, `2^256 + 1` returns `TotalAboveDrawRange` with the digest counter at zero; `Wide` stops with `SampleStop::TotalAboveDrawRange`, with `total` `2^256 + 1` (QSpec TC-354 SM-09, the in-range SM-08 total selecting); the settled wire form is FR-189-AC-7's (TC-624); the three types are `qsl-eval` items and no `quire-semantic-value` item.
- Step 4: equal behaviours with recorded selected item indices, in-support values, FR-187 probabilities and rewards; `SampleStop::NotMarkov`.

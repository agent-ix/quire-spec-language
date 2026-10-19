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

Scope: FR-188-AC-1 to FR-188-AC-5.

## Test Procedure

Fixtures: the weighted-sampler conformance vectors of the generator definition; QSpec TC-210's uniform vectors and TC-354's one-item vectors (SM-05, SM-07); `Service` under `Steady`; the non-unique `Health` variant.

1. Run every weighted vector and every TC-210 vector as unit weights at choice index 0; call `sample_probabilistic` with a `DefinitionRef` of another identity.
2. Draw over weights `(1/3, 1/6, 1/2)` at 60,000 trace indices; draw choices 0 and 1 at the first 100 points.
3. Sample the TC-354 one-item workload steps; call `weighted_choice` over one weight, over a scaled total of `2^256` and over `(1, 2^256)`.
4. Sample `Service` from seed 7, trace 12, 11 positions, twice; sample the `Health` variant.

Tag the tests `#[trace("TC-623", "FR-188-AC-n")]`.

## Expected Results

- Step 1: every selected index equals the vector's; the sample returns `SampleStop::NotSimulated(GeneratorMismatch)` with no draw.
- Step 2: scaled `(2, 1, 3)`, `N = 6`; every selection equals the reference; choices 0 and 1 differ at some point.
- Step 3: no draw and no index for one-item choices; the parameter choice has index 0 and the TC-210 step-0 preimage; the total `2^256` selects, `2^256 + 1` returns `TotalAboveDrawRange` with no draw and stops a sample.
- Step 4: equal behaviours with recorded choices, in-support values, FR-187 probabilities and rewards; `SampleStop::NotMarkov`.

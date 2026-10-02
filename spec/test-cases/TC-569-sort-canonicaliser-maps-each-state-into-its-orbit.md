---
id: TC-569
title: "The sort canonicaliser maps each state into its orbit and coalesces orbits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: verifies
---
# TC-569: The sort canonicaliser maps each state into its orbit and coalesces orbits

## Description

Verify canonical forms and permutations, the orbit counts of ADR-021 §7.1, duplicates, and determinism.

Scope: FR-152-AC-1 to FR-152-AC-4.

## Test Procedure

Fixtures: ADR-021 §7.1's subject; the `link` subject of FR-152-AC-3.

1. Canonicalise `(0,1,0)` and `(0,0,1)` under `[b, c]`, and `(2,0,1)` under `[a, b, c]`.
2. Explore §7.1's subject under `[[a, b, c]]` and under the stabiliser of `a`; for each stored state, check it is `π(s)` for some reached `s` and its returned `π`.
3. Canonicalise the two `link` states of FR-152-AC-3 and run the subject.
4. Canonicalise one state twice; canonicalise with the empty group.

Tag the tests `#[trace("TC-569", "FR-152-AC-n")]`.

## Expected Results

- Step 1: `(0,0,1)` with `(b c)`; itself with the identity; `(0,1,2)`.
- Step 2: 10 and 18 stored model states; every check holds.
- Step 3: each result is in the orbit with its permutation; one or two stored states for the orbit.
- Step 4: byte-equal keys and equal permutations; the state and the identity.

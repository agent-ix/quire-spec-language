---
id: TC-203
title: "Existing Kani lowering corpus output is byte-identical before and after the registry swap"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-079
    type: verifies
---
# TC-203: Existing Kani lowering corpus output is byte-identical before and after the registry swap

## Description

Corpus-differential test: for every case in the existing Kani lowering
corpus, compare the lowering output produced against a recording captured
before the registry swap with the output produced against the same inputs
after the swap. Scope: FR-079-AC-1.

Catches an implementation that changes lowering output incidentally while
migrating target selection (for example, a different IR node ordering, a
changed target-name string embedded in an artifact, or a dropped field)
that a "the corpus still passes" check would miss if the corpus's expected-
output fixtures were regenerated in the same change rather than compared
against the pre-swap recording.

## Test Procedure

1. Before applying the registry-swap implementation, run the full existing
   Kani lowering corpus and record each case's output bytes to a
   pre-swap snapshot, without modifying the corpus's checked-in expected-
   output fixtures.
2. Apply the registry-swap implementation (FR-075).
3. Re-run the identical corpus, unchanged, against the post-swap code and
   record each case's output bytes.
4. Diff each case's pre-swap recording (step 1) against its post-swap
   output (step 3), byte for byte.

## Expected Results

Every corpus case's post-swap output is byte-identical to its pre-swap
recording. No case's diff is empty only because its expected-output
fixture was updated alongside the implementation change; the comparison is
against the independently captured pre-swap recording.

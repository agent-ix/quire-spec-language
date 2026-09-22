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
against the pre-swap recording. It also catches a pre-swap recording that
is only a local, unpinned snapshot: without a recording tied to a specific
pre-swap commit, a later contributor can silently re-capture "pre-swap"
output from an already-changed tree, which would make this test compare
the new implementation against itself and never fail.

## Test Procedure

1. Before applying the registry-swap implementation, on the last commit
   that still has the fixed `ProjectionTarget` catalog, run the full
   existing Kani lowering corpus and record each case's output bytes to a
   pre-swap snapshot file. Commit that snapshot file into the repository
   (in a commit that precedes the registry-swap implementation commit, and
   whose commit identifies the pre-swap revision it was captured from) as
   a checked-in fixture, without modifying the corpus's own checked-in
   expected-output fixtures.
2. Apply the registry-swap implementation (FR-075) in a later commit.
3. Re-run the identical corpus, unchanged, against the post-swap code and
   record each case's output bytes.
4. Diff each case's committed pre-swap recording (step 1) against its
   post-swap output (step 3), byte for byte.
5. Confirm the pre-swap snapshot file's commit is an ancestor of, and
   distinct from, the registry-swap implementation commit, so the
   recording cannot have been captured from post-swap code.

## Expected Results

Every corpus case's post-swap output is byte-identical to its pre-swap
recording. No case's diff is empty only because its expected-output
fixture was updated alongside the implementation change; the comparison is
against the independently captured, committed pre-swap recording. Step 5
confirms the recording is pinned to a pre-swap commit and could not have
been silently re-captured from the post-swap tree.

## Status

Planned. The registry swap in step 2 is #217's catalog replacement (FR-079
Description; ADR-011 §7.3 M-6b). Step 1's snapshot is captured on the last
commit before #217's replacement lands.

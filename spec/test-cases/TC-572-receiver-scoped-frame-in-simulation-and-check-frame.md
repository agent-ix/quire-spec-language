---
id: TC-572
title: "A receiver-scoped modifies entry limits candidates and check_frame to the receiver"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: verifies
---
# TC-572: A receiver-scoped modifies entry limits candidates and check_frame to the receiver

## Description

Verify candidate generation and `check_frame` under `modifies self.value` and `modifies value`.

Scope: FR-154-AC-1, FR-154-AC-2, FR-154-AC-5.

## Test Procedure

Fixtures: FR-120's `test/counters` package; state `w` with `c1` and `c2` at `value` 0.

1. Expand `increment` on `c1` from `w` under each frame.
2. Call `check_frame` with each effect and receivers `c1` and `c2` on a post with `c2.value` 1.

Tag the tests `#[trace("TC-572", "FR-154-AC-n")]`.

3. Add `drop()` framed `deletes self`, then `deletes Counter`; call `check_frame` with receiver `c1` on posts without `c1`, without `c2`, and without either, and emit both frame nodes.

## Expected Results

- Step 1: three successors changing only `c1.value`; nine successors.
- Step 2: `frame_violation`/`unauthorized-change` naming `c2` and `value` for receiver `c1` under `modifies self.value`; a delta otherwise.
- Step 3: under `deletes self`, the post without `c1` is admitted and the post without `c2` is `frame_violation`/`unauthorized-change` naming `c2`; under `deletes Counter` all three are admitted; the two frame nodes differ.

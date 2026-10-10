---
id: TC-734
title: "A 100,000-deep recursive call completes on work fuel"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-262
    type: verifies
---
# TC-734: A 100,000-deep recursive call completes on work fuel

## Description

Verify that call depth is bounded by `work_units` only.

Scope: FR-262-AC-1.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Call `function count using v(n: Int[0, 1000000]): Integer pure
   decreases(n) { if n = 0 then 0 else count(n - 1) + 1 }` with `100000`
   under a `work_units` budget sized for it, and record the spend `w`.
2. Call it again with `work_units` at `w - 1`, then at `w`.

Tag the tests `#[trace("TC-734", "FR-262-AC-1")]`.

## Expected Results

- Step 1: completes with `100000`.
- Step 2: `incomplete { limit_kind: work_units }` at the denied charge; then
  completes with `100000`.


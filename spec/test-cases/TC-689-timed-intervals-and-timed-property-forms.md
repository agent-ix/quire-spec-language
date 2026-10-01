---
id: TC-689
title: "S3 checks timed intervals with open or closed ends and classifies TT-1 to TT-4, with the punctual-interval boundary"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-234
    type: verifies
---
# TC-689: S3 checks timed intervals with open or closed ends and classifies TT-1 to TT-4, with the punctual-interval boundary

## Description

Verify timed interval checking, form classification, the punctual-interval disposition and pointwise evaluation with end openness.

Scope: FR-234-AC-1 to FR-234-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the timed traces of FR-234-AC-4.

1. Classify the four claims of FR-234-AC-1.
2. Check `[0 ms, 3 ms)`, `(1 ms, 3 ms]`, `(2 ms, 2 ms]` and `[3 ms, 2 ms]`.
3. Write the request for the punctual liveness claim, check the bounded punctual claim, and evaluate the liveness claim on a timed trace.
4. Evaluate the three formulas of FR-234-AC-4 on the trace with stamps `0, 1, 3`, then with a repeated step at stamp 1.

Tag the tests `#[trace("TC-689", "FR-234-AC-n")]`.

## Expected Results

- Step 1: `TimedSafety`, `TimedInvariant`, `BoundedWindow{horizon: 3}`, `TimedLiveness`.
- Step 2: openness recorded and keys different from `[0 ms, 3 ms]`; the last two refuse `invalid_model_binding`/`malformed-declaration`.
- Step 3: `Unsupported`, `PunctualInterval`, naming the interval; `BoundedWindow{horizon: 2}`; a value.
- Step 4: `true`, `false`, `true`; unchanged after the insertion.

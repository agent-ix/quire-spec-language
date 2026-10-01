---
id: TC-691
title: "Timed counterexamples carry exact rational delays, a final delay for time-locks, and a time-divergent lasso"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-236
    type: verifies
---
# TC-691: Timed counterexamples carry exact rational delays, a final delay for time-locks, and a time-divergent lasso

## Description

Verify the delay member of each step, the post-state digest over clocks, the time-lock final delay, timed lasso closure, and untimed counterexamples unchanged.

Scope: FR-236-AC-1 to FR-236-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the strict-guard variant; the untimed `Counter` unit.

1. Produce `NoLateReply`'s counterexample with `T = 3 ms` and inspect its steps and digests.
2. Produce the strict-guard time-lock counterexample and a `Formula` counterexample.
3. Produce a refutation of `always eventually holds(c.phase = Replied)` with `T = 3 ms`.
4. Produce the deadlock-freedom counterexample over `Counter`.

Tag the tests `#[trace("TC-691", "FR-236-AC-n")]`.

## Expected Results

- Step 1: delays `0, 3, 0`, `trace_position` 3, length 3; digests change exactly with the post-state including clocks.
- Step 2: one step, `final_delay` 3, `kind: TimeLock`; `final_delay: None`.
- Step 3: a lasso with positive loop delay, closed discrete state, each unreset clock above 3 at entry.
- Step 4: `delay: None` on every step; bytes equal FR-126's counterexample.

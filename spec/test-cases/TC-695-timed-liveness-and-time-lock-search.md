---
id: TC-695
title: "Timed liveness under time divergence and fairness, and time-lock search, on the symbolic graph"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-240
    type: verifies
---
# TC-695: Timed liveness under time divergence and fairness, and time-lock search, on the symbolic graph

## Description

Verify that Zeno cycles do not refute, that fairness splits decide exactly, that local and non-local time-locks are found and confirmed, and the vacuity rule.

Scope: FR-240-AC-1 to FR-240-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; its Zeno `reply` loop variant; the `Serve` model; the strict-guard variant; the `Stall` model.

1. Check the response claim of FR-240-AC-1 over `Rpc` and over the Zeno variant.
2. Check `eventually holds(done)` over `Serve` with and without `fair weak serve`.
3. Check the time-lock-freedom item over the strict-guard variant, `Rpc` and `Stall`.
4. Check `always eventually holds(true)` and `always holds(true)` under the timed profile over `Stall`.

Tag the tests `#[trace("TC-695", "FR-240-AC-n")]`.

## Expected Results

- Step 1: `Holds`; `Holds`, with no Zeno lasso reported.
- Step 2: `Holds`; `Violated` with an `idle` lasso of positive loop delay.
- Step 3: `Violated`, `TimeLock`, stem `send` at 0, final delay 3; `Holds`; `Violated`, `TimeLock`, confirmed by fresh exploration.
- Step 4: `Undecided(NoAdmittedBehaviour)` for both.

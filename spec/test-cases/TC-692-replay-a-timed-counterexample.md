---
id: TC-692
title: "The replay facade replays timed counterexamples and time-locks in exact arithmetic"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: verifies
---
# TC-692: The replay facade replays timed counterexamples and time-locks in exact arithmetic

## Description

Verify replay of timed prefixes and lassos, the refusals, local time-lock replay and non-local time-lock replay by fresh exploration.

Scope: FR-237-AC-1 to FR-237-AC-4.

## Test Procedure

Fixtures: ADR-026 §11's `Rpc` unit with `T = 3 ms` and `T = 4 ms`, universe `{c}`; the strict-guard variant and its `x >= 3 ms` variant; the `Stall` model and its resetting variant.

1. Replay FR-236-AC-1's counterexample and the `[0 ms, 3 ms)` refutation with `T = 4 ms`.
2. Replay each tampered payload of FR-237-AC-2.
3. Replay the local time-lock counterexample against the strict-guard variant and against its `x >= 3 ms` variant.
4. Replay the `Stall` non-local time-lock payload against `Stall`, against the resetting variant, and with an exploration limit of one state; replay one envelope twice.

Tag the tests `#[trace("TC-692", "FR-237-AC-n")]`.

## Expected Results

- Step 1: each `reproduced-with-evaluated-witness`.
- Step 2: each refuses with the code FR-237 names and settles no result.
- Step 3: `reproduced-with-evaluated-witness`; `inconclusive`, `ReplayParity`.
- Step 4: `reproduced-with-evaluated-witness`; `inconclusive`, `ReplayParity`; V-7; equal results.

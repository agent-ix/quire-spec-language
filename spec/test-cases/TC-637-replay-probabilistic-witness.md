---
id: TC-637
title: "replay_probabilistic_witness reproduces path-set and subsystem refutations and refuses altered witnesses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: verifies
---
# TC-637: replay_probabilistic_witness reproduces path-set and subsystem refutations and refuses altered witnesses

## Description

Verify path-set replay, scheduler, support, post-state and prefix checks, subsystem evidence through the checker, the fairness check, and the path-count setting.

Scope: FR-202-AC-1 to FR-202-AC-4, FR-202-AC-6 and FR-202-AC-7.

## Test Procedure

Fixtures: §15.4's and §15.2's one-path refutations; §15.3's per-window subsystem refutation; `Coin2`'s fair witness; altered copies.

1. Replay §15.4's and §15.2's path sets.
2. Replay §15.4's path with a non-scheduler step, an out-of-support value, and duplicated.
3. Replay §15.3's subsystem evidence; replay `Coin2`'s witness and the copy whose scheduler waits forever.
4. Refute §15.4 with `max_witness_paths` 0 and replay; replay one envelope twice.
5. Set the caller's `ReplayLimits.input_bytes` to one less than the measured encoded size of a `ReplayRequestWire`, then replay with that wire and otherwise valid inputs.
6. Replay the FR-202-AC-6 quantile path sets over the two-point model and its half-activated variant.

Tag the tests `#[trace("TC-637", "FR-202-AC-n")]`.
Tag the caller-configured input-limit assertion `#[trace("TC-637", "FR-202-AC-7")]`.

## Expected Results

- Step 1: `refuted` with `1/25` and `343/5000`.
- Step 2: `invalid-value` twice; `inconclusive`, `ReplayParity`.
- Step 3: `refuted`; `refuted`; `inconclusive`, `ReplayParity`.
- Step 4: subsystem evidence that replays `refuted`; equal outcomes.
- Step 5: `ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(BoundExceeded { bound: B - 1, actual: B }))`; byte-provision lookup and recompile are not reached. Trace this assertion as `FR-202-AC-7`.
- Step 6: the two-point model: `refuted`, then `inconclusive`, `ReplayParity`; its half-activated variant: `refuted` with `N` counted, then `inconclusive`, `ReplayParity` without the not-activated path.

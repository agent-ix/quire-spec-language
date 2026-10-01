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

Verify path-set replay, scheduler, support, digest and prefix checks, subsystem evidence through the checker, the fairness check, and the path-count setting.

Scope: FR-202-AC-1 to FR-202-AC-4.

## Test Procedure

Fixtures: §15.4's and §15.2's one-path refutations; §15.3's per-window subsystem refutation; `Coin2`'s fair witness; altered copies.

1. Replay §15.4's and §15.2's path sets.
2. Replay §15.4's path with a non-scheduler step, an out-of-support value, an altered digest, and duplicated.
3. Replay §15.3's subsystem evidence; replay `Coin2`'s witness and the copy whose scheduler waits forever.
4. Refute §15.4 with `max_witness_paths` 0 and replay; replay one envelope twice.

Tag the tests `#[trace("TC-637", "FR-202-AC-n")]`.

## Expected Results

- Step 1: `refuted` with `1/25` and `343/5000`.
- Step 2: `invalid-value` twice; `revision-mismatch`; `inconclusive`, `ReplayParity`.
- Step 3: `refuted`; `refuted`; `inconclusive`, `ReplayParity`.
- Step 4: subsystem evidence that replays `refuted`; equal outcomes.

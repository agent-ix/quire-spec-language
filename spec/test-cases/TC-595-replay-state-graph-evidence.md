---
id: TC-595
title: "The replay facade replays witnesses, traps and path pairs through ModelSystem"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-170
    type: verifies
---
# TC-595: The replay facade replays witnesses, traps and path pairs through ModelSystem

## Description

Verify `replay_model_graph` on each `GraphEvidence` arm: reproduction,
disagreement, refusal, a stopped trap replay, and determinism.

Scope: FR-170-AC-1 to FR-170-AC-4 and FR-170-AC-6.

## Test Procedure

Fixtures: the envelopes FR-168's worked examples produce, over ADR-022 §7's
units, and the edited envelopes FR-170-AC-3 and AC-4 name.

1. Replay `ReachesTwo`'s sampled and explored witnesses, with no sampler in
   the request.
2. Replay `ReachesThree`'s trap, `CanStillWin`'s trap and `InOneWay`'s path
   pair.
3. Replay the truncated witness, the trap whose stem ends at `Mid`, the path
   pair with equal sequences, and the `CanStillWin` trap in an envelope for
   its `from` variant.
4. Replay a witness with an altered digest, one with a step that is not
   enabled, one missing a path for an initial state, and one with an
   `initial` index of 1 over a one-snapshot subject; replay `ReachesThree`'s
   trap with `max_states` 2; replay one envelope twice.
5. Replay FR-170-AC-6's lasso witness, its broken variant and a product
   trap.

Tag the tests `#[trace("TC-595", "FR-170-AC-n")]`.

## Expected Results

- Step 1: both reproduce.
- Step 2: the first explores 9 nodes and reproduces; the second re-executes
  `play, lose`, explores `{Lost}` and reproduces; the third reproduces.
- Step 3: each settles `inconclusive`, `Verdicts`.
- Step 4: four refusals with no result; `stage_limit_exceeded` naming
  `max_states`; equal results.
- Step 5: the lasso replays its loop; the moved loop entry refuses; the
  product trap goes to FR-181's replay.

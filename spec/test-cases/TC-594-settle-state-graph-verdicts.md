---
id: TC-594
title: "State-graph outcomes settle as terminal records that state their settlement method"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-169
    type: verifies
---
# TC-594: State-graph outcomes settle as terminal records that state their settlement method

## Description

Verify the verdict map for V-9, V-10, V-4, V-1 and V-5 to V-8 over
state-graph outcomes, the `Witness` proof basis and its sources, the
settlement method and seed in the record, and that no evidence settles
before it replays.

Scope: FR-169-AC-1 to FR-169-AC-5.

## Test Procedure

1. Map one input of each table row.
2. Settle §7.1's `ReachesTwo` with default limits and with `witness_samples`
   0; settle `ReachesThree`.
3. Settle §7.2's `CanStillWin` and its `from` variant; §7.3's `InOneWay`.
4. Settle FR-168-AC-4's two outcomes; a run stopped by `max_states`; a run
   with an undecided node and no decisive evidence; a subject with no
   initial state.
5. Settle a `Witnessed` outcome whose replay settles `inconclusive`, and a
   `Trapped` outcome whose replay a limit stops.

Tag the tests `#[trace("TC-594", "FR-169-AC-n")]`.

## Expected Results

- Step 1: each row's `TerminalValue`, label, basis and category as FR-169's
  table states; `Proved{Witness{…}}` maps to success.
- Step 2: `proved`, `decisive-witness`, `Witness{[Sampled(…)]}`, `Certified`, success,
  with seed and trace index in the record; the same with `[Explored]` and a
  record naming exploration; `refuted`, `closed-scope`.
- Step 3: `refuted`, `closed-scope`; `proved`, `closed-scope`,
  `Exhaustive`, `Certified`; `refuted`, `decisive-counterexample`.
- Step 4: `refuted` (V-10); `inconclusive`, `BoundReached{depth: 3}`,
  execution `completed`, truth `pending`; `failed`, `resource-incomplete`,
  naming `max_states` and its value; `inconclusive`, `UndecidedSuccessor`;
  `inconclusive`, `NoInitialState`.
- Step 5: `inconclusive`, `ReplayParity`; `inconclusive`, `ReplayRefused`,
  naming the limit.

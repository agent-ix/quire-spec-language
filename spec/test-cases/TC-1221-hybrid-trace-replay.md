---
id: TC-1221
title: "Hybrid traces replay statically and exactly against the checked hybrid model"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-450
    type: verifies
---
# TC-1221: Hybrid traces replay statically and exactly against the checked hybrid model

## Description

Verify static replay, exact replay and the settlement each gives, for a
trace from a `solver-inconclusive` cause and from a provider's `refuted`
entry.

Scope: FR-450-AC-1 to FR-450-AC-8.

## Test Procedure

Fixtures: FR-447's `Braking` and `SlowEnough`; the `Bounce` declaration of
FR-450-AC-6; AC-8's `descending` wrapper, recursive `descend` function and
`Bounce` variant; a plugin fixture provider (FR-290) and an in-process fixture
provider, each returning a scripted entry with a scripted `HybridTrace`.

1. Return each `solver-inconclusive` trace FR-450-AC-1 lists, then the trace
   with `delta` `-1/100`.
2. Return AC-2's point trace as a `solver-inconclusive` trace and as a
   provider's `refuted` entry, against `always [0, 12] holds(h.v > 0)`; then
   against `SlowEnough`.
3. Return each altered trace FR-450-AC-3 lists, as a provider's `refuted`
   entry.
4. Replay AC-2's trace over each changed flow FR-450-AC-4 lists.
5. Replay AC-2's trace against each claim FR-450-AC-5 lists, and statically replay the guard calling the recursive function.
6. Replay each `Bounce` trace FR-450-AC-6 lists, with and without
   `fair Up;`.
7. Replay the `loop_to` 0 point lasso from AC-6 against
   `eventually [999999, 1000000] holds(h.x = 2)` from a provider's `refuted`
   entry with `work_units` 100000.
8. Statically replay the `Up`-to-`Down` point trace over AC-8's `Bounce`
   variant, after S3 admits `descending(x)` as a hybrid guard call, first with
   `work_units` 100 and then with `work_units` `u64::MAX`.

Run each step through both fixture providers.

Tag the tests `#[trace("TC-1221", "FR-450-AC-n")]`.

## Expected Results

- Step 1: each static outcome and `inconclusive` record FR-450-AC-1 states;
  the negative `delta` refused at `/delta`.
- Step 2: `Reproduced`, kind `Formula`, the item `refuted` with the trace as
  its counterexample and one `counterexamples` entry, from both entries;
  `NotReproduced` at `Claim` against `SlowEnough`, the item `inconclusive`.
- Step 3: `NotReproduced` at `Dwell` for each; `inconclusive`,
  `ReplayParity`, with no `counterexamples` entry.
- Step 4: `NotExact` and `inconclusive`; exact replay with the stated
  polynomials and `NotReproduced` at `Dwell`; `NotExact`.
- Step 5: `Reproduced`, kind `UndefinedEvaluation` at time 15/2, the item
  `refuted` with cause `UndefinedEvaluation{where, cause}`; `Reproduced`,
  kind `Formula`, decisive at time 1499/200; `NotExact`; static replay
  terminating with the call's result range.
- Step 6: `Reproduced` with and without the fairness constraint;
  `NotReproduced` at `Loop` for `loop_to` 1 and at `Jump` for the `x = 1/2`
  trace; the `loop_to` 2 trace refused at `/loop`.
- Step 7: the 2-second lasso requires at least 500,000 `ClaimSegment`s to
  reach the interval's upper endpoint at 1,000,000 seconds. At
  `work_units` 100000, the meter cannot cover all segment charges and
  exhausts before deciding the formula; replay therefore returns
  `HybridReplayRefusal::ResourceExhausted{counter: WorkUnits}` and the item
  settles `incomplete`, `LimitReached{limit: work_units, value: 100000,
  setting: accounting.work_units}`, with no verdict or counterexample.
- Step 8: S3 admits the hybrid guard call `descending(x)` because the call
  boundary takes a rational argument and `descending` has a rational
  parameter; the wrapper's body calls `descend(20000)`, so the recursive
  integer call remains outside the hybrid-term boundary. At `work_units` 100,
  static replay returns
  `HybridReplayRefusal::ResourceExhausted{counter: WorkUnits}` and the item
  settles `incomplete`, `LimitReached{limit: work_units, value: 100,
  setting: accounting.work_units}`, with no verdict or counterexample; under
  `u64::MAX`, QSpec FR-146 admits the `Int[0, 20000]` measure, the call takes
  20,000 recursive edges (20,001 recursive `descend` calls plus the one
  `descending` wrapper call) and completes on the explicit stack,
  static replay returns `Consistent`, and no stack overflow occurs.
- Both fixture providers give the same outcomes.

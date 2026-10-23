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

Scope: FR-450-AC-1 to FR-450-AC-6.

## Test Procedure

Fixtures: FR-447's `Braking` and `SlowEnough`; the `Bounce` declaration of
FR-450-AC-6; a plugin fixture provider (FR-290) and an in-process fixture
provider, each returning a scripted entry with a scripted `HybridTrace`.

1. Return each `solver-inconclusive` trace FR-450-AC-1 lists, then the trace
   with `delta` `-1/100`.
2. Return AC-2's point trace as a `solver-inconclusive` trace and as a
   provider's `refuted` entry, against `always [0, 12] holds(h.v > 0)`; then
   against `SlowEnough`.
3. Return each altered trace FR-450-AC-3 lists, as a provider's `refuted`
   entry.
4. Replay AC-2's trace over each changed flow FR-450-AC-4 lists.
5. Replay AC-2's trace against the claim FR-450-AC-5 lists.
6. Replay each `Bounce` trace FR-450-AC-6 lists, with and without
   `fair Up;`.

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
  `refuted` with cause `UndefinedEvaluation{where, cause}`.
- Step 6: `Reproduced` with and without the fairness constraint;
  `NotReproduced` at `Loop` for the two altered traces.
- Both fixture providers give the same outcomes.

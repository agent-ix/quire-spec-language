---
id: TC-628
title: "Sampled witnesses are kept in trace order and replay through the model"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: verifies
---
# TC-628: Sampled witnesses are kept in trace order and replay through the model

## Description

Verify witness keeping, replay with support and draw checks, the refusal and parity outcomes, and conversion to a qualitative counterexample.

Scope: FR-193-AC-1 to FR-193-AC-3.

## Test Procedure

Fixtures: `Service` and the `2 ms` claim, seed 7, `max_witnesses` 3.

1. Run the claim and read the witnesses.
2. Replay each witness; replay altered copies (value outside the support, changed seed, changed path probability).
3. Convert a witness to a `TemporalCounterexample` for the TP-2 claim and replay it.

Tag the tests `#[trace("TC-628", "FR-193-AC-n")]`.

## Expected Results

- Step 1: three witnesses in trace order with latency above `2 ms` and exact path probabilities.
- Step 2: `Reproduced`; `invalid_runtime_input`/`invalid-value`; `Inconclusive(ReplayParity)` twice; the item stays `measured`, `rejected`.
- Step 3: the TP-2 claim settles `refuted`.

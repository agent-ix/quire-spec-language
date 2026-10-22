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

Scope: FR-193-AC-1 to FR-193-AC-3, FR-193-AC-5.

## Test Procedure

Fixtures: `Service` and the `2 ms` claim, seed 7, `max_witnesses` 3; `Health` with the `<= 1/100` fault claim of FR-193-AC-3; `LongRun`.

1. Run the claim and read the witnesses.
2. Replay each witness; replay altered copies (value outside the support, changed seed, changed path probability).
3. Replay a `Service` witness with `replay_sampled_witness`, which converts it to a counterexample for the TP-2 claim and replays that in the one call; do the same for a witness of the `<= 1/100` fault claim; replay three copies of that witness: one with the first step that reaches `n.healthy` false replaced by the other choice of the same draw and that choice's successor digest, later steps dropped; one with a step's choice changed and its post-state digest kept; one with a step changed to a transition not enabled at its pre-state (in `Health` an `attemptUpdate` with a receiver `z` whose identity lies outside the universe, built as in FR-128-AC-3); run `LongRun`.
4. Refuse `Service` and `Health` at S4, then run step 2's replays of `Service` witnesses and step 3's replays of `Service` and `Health` witnesses over their checked subjects.

Tag the tests `#[trace("TC-628", "FR-193-AC-n")]`.

## Expected Results

- Step 1: three witnesses in trace order with latency above `2 ms` and exact path probabilities.
- Step 2: `Reproduced`; `invalid_runtime_input`/`invalid-value`; `Inconclusive(ReplayParity)` twice; the item stays `measured`, `rejected`.
- Step 3: `Reproduced` for the `Service` witness and for the `Health` fault-claim witness, each of which includes the refutation of its TP-2 claim by the converted counterexample; `Inconclusive(ReplayParity)` for the first copy, `stale_dependency`/`content-mismatch` for the second, `invalid_runtime_input`/`invalid-value` for the third; `LongRun` keeps no witness.
- Step 4: the S4 refusals of `Service` (FR-185-AC-7) and of `Health`; the same outcomes as steps 2 and 3 for the `Service` witnesses and as step 3 for the `Health` witness, although `emit_checked` refuses each package; the replay call takes only the checked subject, the provenance and the witness.

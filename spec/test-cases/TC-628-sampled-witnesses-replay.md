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
3. Convert a witness to a `TemporalCounterexample` for the TP-2 claim and replay it; do the same for a witness of the `<= 1/100` fault claim; run `LongRun`.
4. Refuse `Service` and `Health` at S4, then run steps 2 and 3's `Service` replays and step 3's `Health` replay over their checked subjects.

Tag the tests `#[trace("TC-628", "FR-193-AC-n")]`.

## Expected Results

- Step 1: three witnesses in trace order with latency above `2 ms` and exact path probabilities.
- Step 2: `Reproduced`; `invalid_runtime_input`/`invalid-value`; `Inconclusive(ReplayParity)` twice; the item stays `measured`, `rejected`.
- Step 3: the TP-2 claim settles `refuted`; the negated claim settles `refuted`; `LongRun` keeps no witness.
- Step 4: the S4 refusal of FR-185-AC-7; the same outcomes as steps 2 and 3 for `Service`, and the `refuted` outcome for the `Health` fault claim, although `emit_checked` refuses the package; the replay call takes only the checked subject, the provenance and the witness.

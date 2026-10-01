---
id: TC-710
title: "An undefined timed claim evaluation refutes with a timed prefix and replays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-235
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: verifies
---
# TC-710: An undefined timed claim evaluation refutes with a timed prefix and replays

## Description

Verify that the zone search refutes a timed claim that evaluates undefined,
with a timed prefix of exact rational delays ending at the undefined
position, that the counterexample replays, and that the item settles
`refuted` with cause `UndefinedEvaluation`.

Scope: FR-239-AC-5, FR-237-AC-5, FR-235-AC-5.

## Test Procedure

Fixture: the `Ticker` model and claim of FR-239-AC-5.

1. Run the zone search.
2. Replay the counterexample, then the same counterexample with its last
   step removed.
3. Settle the outcome with step 2's first replay result.

Tag the tests `#[trace("TC-710", "<AC id>")]`.

## Expected Results

- Step 1: `Violated`, prefix `step` after delay 1 and `step` after delay 1,
  `kind: UndefinedEvaluation{where: 2, cause: division-by-zero}`.
- Step 2: `reproduced-with-evaluated-witness`; then `inconclusive`,
  `ReplayParity`.
- Step 3: `refuted`, `decisive-counterexample`, category violation, cause
  `UndefinedEvaluation`.

---
id: TC-640
title: "A sampled undefined evaluation is a rejection and replays"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-189
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-192
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-193
    type: verifies
---
# TC-640: A sampled undefined evaluation is a rejection and replays

## Description

Verify that a sample on which a probabilistic claim evaluates undefined
rejects its test at once with `UndefinedEvaluation` naming the sample, that
the result settles "measured: rejected" and fails the pipeline, and that the
undefined sample is kept as a witness that replays.

Scope: FR-189-AC-6, FR-192-AC-5, FR-193-AC-4.

## Test Procedure

Fixture: the `Coin` model and claim of FR-189-AC-6.

1. Run EN-4.
2. Settle the outcome and run the pipeline gate over it.
3. Replay the kept undefined witness, then the same witness with its cause
   changed to `precondition-false`.

Tag the tests `#[trace("TC-640", "<AC id>")]`.

## Expected Results

- Step 1: `Completed` Rejected; the test stops at the first sample in
  trace-index order that reaches `v = 1`, with `UndefinedEvaluation` naming
  test 0, that trace index and the first position with `v = 1`, cause
  `division-by-zero`.
- Step 2: `measured`, `rejected`, category violation, the record carrying
  the `UndefinedEvaluation`; the gate fails as on a violation.
- Step 3: `Reproduced`; then `Inconclusive(ReplayParity)`.

---
id: TC-1392
title: "monitor produces each applicable outcome kind of the lifecycle matrix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: verifies
---
# TC-1392: monitor produces each applicable outcome kind of the lifecycle matrix

## Description

Run the `monitor` row of FR-564's matrix through the library.

Scope: FR-564-AC-5.

## Test Procedure

Run `monitor` over each applicable cell's input: FR-283-AC-2's lasso with
`q` at a loop position; FR-283-AC-1's trace; FR-283-AC-3's trace whose
second snapshot fails admission; FR-283-AC-6's fairness clause alone;
FR-283-AC-1's request with a handle cancelled `Deadline`, then `Requested`,
before the call; FR-283-AC-12's `work_units` 0.

Tag the tests `#[trace("TC-1392", "FR-564-AC-5")]`.

## Expected Results

`tested` exit 0; violation exit 10; the admission refusal exit 20;
unsupported `missing-fairness-premise` exit 21;
`StageFailure::Cancelled(Deadline)` and `StageFailure::Cancelled(Requested)`,
exit 22; `Incomplete` naming `work_units`, exit 22.

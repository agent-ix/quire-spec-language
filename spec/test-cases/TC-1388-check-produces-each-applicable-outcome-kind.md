---
id: TC-1388
title: "check produces each applicable outcome kind of the lifecycle matrix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: verifies
---
# TC-1388: check produces each applicable outcome kind of the lifecycle matrix

## Description

Run the `check` row of FR-564's matrix through the library.

Scope: FR-564-AC-1.

## Test Procedure

Run `check` over each applicable cell's input as FR-564's table names it:
success, refusal, unsupported, timeout (`Deadline`), cancellation
(`Requested`) and incomplete (`s3.nodes` one below). Map each outcome's
category through FR-285's function.

Tag the tests `#[trace("TC-1388", "FR-564-AC-1")]`.

## Expected Results

Success exit 0; `ill_typed` refusal exit 20; unsupported exit 21;
`StageFailure::Cancelled(Deadline)` exit 22;
`StageFailure::Cancelled(Requested)` exit 22; `LimitExceeded` naming
`s3.nodes` exit 22.

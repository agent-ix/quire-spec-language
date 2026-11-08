---
id: TC-1391
title: "analyze produces each applicable outcome kind of the lifecycle matrix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: verifies
---
# TC-1391: analyze produces each applicable outcome kind of the lifecycle matrix

## Description

Run the `analyze` row of FR-564's matrix through the library.

Scope: FR-564-AC-4.

## Test Procedure

Run `analyze` over each applicable cell's input: FR-281-AC-1's item;
FR-281-AC-2's item; FR-281-AC-9's request with the undeclared population;
FR-281-AC-3's unsupported item alone; FR-281-AC-1's item with a finite
`deadline_ms` that expires before it settles; the same with caller
cancellation first; FR-281-AC-6's state budget 1.

Tag the tests `#[trace("TC-1391", "FR-564-AC-4")]`.

## Expected Results

`proved` exit 0; `refuted` exit 10;
`StageFailure::Refused(AnalyzeRefusal::Binding)` exit 20; `unsupported`
exit 21; `incomplete` with `cancelled{source: deadline}` and with
`cancelled{source: requested}`, exit 22; `incomplete` with the state-budget
cause, exit 22.

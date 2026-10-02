---
id: TC-867
title: "The layering comparison returns the table's result for every parent and child class"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-345
    type: verifies
---
# TC-867: The layering comparison returns the table's result for every parent and child class

## Description

Verify FR-345's comparison as a pure function over every (`R`, `C`) class
pair, and its report for a regression and a prohibited parent, over compile
results produced by spine `compile`.

Scope: FR-345-AC-1 to FR-345-AC-3.

## Test Procedure

1. Enumerate every `R` and `C` over FR-341's six classes and compare each
   with the expected result, written out in the test as a literal table
   derived by hand from FR-345's rows.
2. Take as the parent side the compile result of a unit with an ill-typed
   function body (`refused`) and as the child side the compile result of
   the ConfigVersion unit (`admitted`), naming the case by the two units'
   `RawSourceRef`s and the edge `quire.state.core/v1` to
   `quire.state.queries/v1`. Compare and report.
3. Take as the parent side a refusal whose every cause is
   `unsupported_construct`/`declaration-form` (`prohibited`) and as the
   child an admitted compile. Compare.

Tag the tests `#[trace("TC-867", "FR-345-AC-n")]`.

## Expected Results

- Step 1: every pair returns its literal expected result.
- Step 2: `regression` naming both `RawSourceRef`s, both edge identities and
  the parent's `ill_typed` code; verdict violation, exit 10.
- Step 3: `not applicable`.

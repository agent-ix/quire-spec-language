---
id: TC-863
title: "The versioning comparison returns the table's result for every class pair"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-343
    type: verifies
---
# TC-863: The versioning comparison returns the table's result for every class pair

## Description

Verify FR-343's comparison as a pure function over every combination of
prior class and stage, superseding class and stage, and selection kind. The
exhaustive table catches a `tool failure` hidden by `not applicable` and
any row reordering, with no fault injected into a compile.

Scope: FR-343-AC-1.

## Test Procedure

1. Enumerate every `P` in (each of the seven classes) x (`compile`,
   `select`, `admit`, `evaluate`), every `S` likewise, and each selection
   kind (`Clause`, `Function`, `Frame`).
2. For each combination, compare it with the expected result, written out
   in the test as a literal table derived by hand from FR-343's rows. The
   test does not call the gate's classification or comparison code to
   compute an expectation.

Tag the test `#[trace("TC-863", "FR-343-AC-1")]`.

## Expected Results

- Every combination returns its literal expected result. In particular:
  `P` = `admitted`/`evaluate` with `S` = `absent`/`select` gives `holds`
  for `Clause` and `Frame` and `regression` for `Function`; `P` =
  `refused`/`compile` gives `tool failure`; `P` = `incomplete`/`select`
  gives `unresolved (incomplete)`; `P` = `refused`/`evaluate` with `S` =
  `tool failure` gives `tool failure`.

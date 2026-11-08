---
id: TC-1390
title: "execute produces each applicable outcome kind of the lifecycle matrix"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-564
    type: verifies
---
# TC-1390: execute produces each applicable outcome kind of the lifecycle matrix

## Description

Run the `execute` row of FR-564's matrix through the library.

Scope: FR-564-AC-3.

## Test Procedure

Run `execute` over each applicable cell's input: `seven` on the interpreter;
`id(12)`; `seven` choosing `jit` with `SuppliedBackends::default()`;
FR-276-AC-3's long function cancelled from a second thread with `Deadline`,
then with `Requested`; `seven` with `work_units` 0; and an entry on a
supplied test backend made to return `Fault`.

Tag the tests `#[trace("TC-1390", "FR-564-AC-3")]`.

## Expected Results

Completed 7, exit 0; `CallFailure::Input`, exit 20;
`CallFailure::Unsupported` with `NotSupplied`, exit 21;
`CallFailure::Cancelled(Deadline)` and `CallFailure::Cancelled(Requested)`,
exit 22; `Incomplete` naming `work_units`, exit 22; `Fault`, exit 30, no
value.

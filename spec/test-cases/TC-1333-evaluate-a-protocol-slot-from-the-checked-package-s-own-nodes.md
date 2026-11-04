---
id: TC-1333
title: "Evaluate a protocol slot from the checked package's own nodes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-533
    type: verifies
---
# TC-1333: Evaluate a protocol slot from the checked package's own nodes

## Description

Verify protocol-slot evaluation from a bare package and its equality with the source routes. Scope: FR-533-AC-1 to FR-533-AC-10. **R** is the package of FR-517.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | every slot of FR-390's table **R** holds, over the TC-1070 and TC-1315 supplies, on the bare, in-process and reader-backed routes | equal value, category, exit status, catalog code and cause, unavailable binder, location, losses and ordered charge log |
| 2 | 2 | each outcome row of FR-520's map; `retry.relation` at exact usage and one below | the successor outcomes; completes / `Incomplete` naming the counter |
| 3 | 3 | the calls of row 1 in a run that never compiled the source; the in-process `CheckedPackage` held beside the bare package replaced by another unit's with a different expression at the slot | the same results; the bare result is unchanged, the in-process one changes |
| 4 | 4 | search the bare route's reachable functions for `CheckedPackage`, `CheckedBody`, `source()`, source `Graph` and `protocol_selection::select`; the `NodeKind` match | none; no wildcard arm |
| 5 | 5 | bare observation admission over **R**'s model documents; a model selection the package lacks | the snapshot the source route admits with equal usage; `wrong-model-selection` with equal records |
| 6 | 6 | a package whose `model_root` selects another model than the supply's | the source route's record, before any charge |
| 7 | 7 | slots over a recursive record and a plain-Option record (project today), and records with a nested Option member and a nested Collection member | bare: evaluates to the source route's value and charges; until the wire carries nested Option and Collection member types, `MissingTypeRepresentation` at the binder for the nested cases, no charge, the unmet parity recorded; source route: evaluates |
| 8 | 8 | an `Unavailable` binder that is read | `Unavailable` with the package's location; `None` where the wire has none while the in-process route keeps its source location, recorded as unmet parity |
| 9 | 9 | three slot calls on one meter against three on fresh meters; against the source route | sum for `work_units` and `result_units`, maximum for the other eight; equal to the source route |
| 10 | 10 | `new`, `with_supply`, view creation, selection and supply admission | no counter charged, no `Meter` taken; a refusal leaves the meter identical |

Tag the test `#[trace("FR-533-AC-1", ..., "FR-533-AC-10", "TC-1333")]`; rows 1 and 2 also carry FR-390-AC-11 and FR-520-AC-9.

## Expected Results

Each row's Expected column.

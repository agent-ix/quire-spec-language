---
id: TC-1313
title: "Write the adverse mutation corpus from checked packages and name the destination of every legacy case"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-518
    type: verifies
---
# TC-1313: Write the adverse mutation corpus from checked packages and name the destination of every legacy case

## Description

Verify the checked-package mutation corpus. Scope: FR-518-AC-1 to FR-518-AC-10.
The case table is FR-518's; rows are referenced by number.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `write(R, dir)` with `Standard`; parse `mutations/manifest.json` | format `quire.checked-handoff.mutations/1`; cases in table order with unique identities and stages `reader` or `selection`; 14b, 15b, 28m present; every file non-empty |
| 2 | 2 | submit each `reader` case to `read_checked_package` with the evidence of the directory | refused with the table's code, cause and pointer; every reader case of the table (rows 1 to 7, 14b, 15b, 16, 22 to 27, 28m, 30, 31) |
| 3 | 3 | submit each `selection` case to the reader, then to `Selection::check`; submit unmutated **R** to both | admitted then refused with the row's `SelectionAxis` (rows 8 to 13, 17 to 21, 29); **R** passes both |
| 4 | 4 | submit the cases of rows 12 and 29 to the reader, then `Selection::check` | admitted, then `definition.identity` and `definition.authority` |
| 5 | 5 | a stage double giving another code or pointer; replace a case file by the base bytes | `Error::CaseMismatch` naming the identity, nothing written |
| 6 | 6 | two writes | identical manifest and case files; cases in table order |
| 7 | 7 | search each case's `expected` for `digest`, `seal`, `v2.`; write with `None` | none names any of them; no `mutations/` directory |
| 8 | 8 | list `mutations/` and the manifest's cases; look for a case-count or identity-list export | each file named by one case and each case by one file; no such export |
| 9 | 9 | submit 28m to a reader implementing IR-1273 | `invalid_package`/`source-integrity` at the `CheckedSourceRef` pointer |
| 10 | 10 | submit 28m to a reader double that admits it | `CaseMismatch` naming `definition-original-bytes-moved`, nothing written |

Tag the test `#[trace("FR-518-AC-1", ..., "FR-518-AC-10", "TC-1313")]`; rows 1 to 3 also carry FR-042-AC-7 and FR-042-AC-10.

## Expected Results

Each row's Expected column.

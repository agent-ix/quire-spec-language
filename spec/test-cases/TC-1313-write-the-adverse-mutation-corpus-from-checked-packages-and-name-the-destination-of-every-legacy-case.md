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

Verify the checked-package mutation corpus. Scope: FR-518-AC-1 to FR-518-AC-8.
The case table is FR-518's; rows are referenced by number.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `write(R, dir)` with `Standard`; parse `mutations/manifest.json` | format `quire.checked-handoff.mutations/1`; 30 unique identities equal to the set rows 1 to 13, 14b, 15b, 16 to 27, 29, 30, 31; every file non-empty |
| 2 | 2 | submit each `reader` case to `read_checked_package` with the evidence of the directory | refused with the table's code, cause and pointer; 18 reader cases (rows 1 to 7, 14b, 15b, 16, 22 to 27, 30, 31) |
| 3 | 3 | submit each `selection` case to the reader, then to `Selection::check`; submit unmutated **R** to both | admitted then refused with the row's `SelectionAxis` (12 cases: rows 8 to 13, 17 to 21, 29); **R** passes both |
| 4 | 4 | list the identities in the pinned `writer.rs` at rev `9395be4` (28) and in QSL main (27) | every identity has a row; rows 14, 15 and 28 are marked not movable; the test fails if one is missing |
| 5 | 5 | change one expected value in the table; replace a case file by the base bytes | `Error::CaseMismatch` naming the identity, nothing written |
| 6 | 6 | two writes | identical manifest and case files; cases in table order |
| 7 | 7 | search the manifest for `digest`, `seal`, `v2.`; write with `None` | no case expects any of them; no `mutations/` directory |
| 8 | 8 | import `CORPUS_CASES` and `CORPUS_AXES` from a downstream crate | 30 and the axis set of the table |

Tag the test `#[trace("FR-518-AC-1", ..., "FR-518-AC-8", "TC-1313")]`; rows 1 to 3 also carry FR-042-AC-7 and FR-042-AC-10.

## Expected Results

Each row's Expected column.

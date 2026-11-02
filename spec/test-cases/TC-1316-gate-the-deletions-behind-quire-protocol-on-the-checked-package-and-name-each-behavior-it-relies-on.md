---
id: TC-1316
title: "Gate the deletions behind quire-protocol on the checked package and name each behavior it relies on"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-521
    type: verifies
---
# TC-1316: Gate the deletions behind quire-protocol on the checked package and name each behavior it relies on

## Description

Verify the deletion gate of QSL-840, QSL-838 and QSL-858. Scope: FR-521-AC-1 to
FR-521-AC-5.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | `quire validate` over FR-515 to FR-521; list `use quire_spec_language::...` items in quire-protocol's `src` and `tests` | every row's ids resolve; every imported item is named by a row |
| 2 | 2 | `quire matrix` after the successors merge; delete the file carrying the only tag of FR-042-AC-10 in a fixture tree | none of the listed criteria untagged; the matrix test fails naming FR-042-AC-10 |
| 3 | 3 | check out quire-protocol's default branch after QSL-1549; run `cargo check --locked --all-targets` and its tests with the QSL revision of the deleting change's parent | exit 0; no `handoff-writer` feature in `Cargo.toml`; tests green |
| 4 | 4 | read FR-499's three rows and FR-500's AC-10, AC-15 rows | the stated wording |
| 5 | 5 | read rows 11 and 13; search the deleting change for `work.rs` and the integrity lanes | defaults stated; neither removed while open |

Tag the matrix test `#[trace("FR-521-AC-2", "TC-1316")]`.

## Expected Results

Each row's Expected column.

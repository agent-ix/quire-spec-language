---
id: SR-2160003
title: "Spec review of the FR-517 and FR-518 amendments carried by quire-spec-language PR 2160 (QSL-1546)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-517-write-a-checked-package-handoff-directory-from-source-in-process.md; spec/functional/FR-518-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md; spec/test-cases/TC-1313-write-the-adverse-mutation-corpus-from-checked-packages-and-name-the-destination-of-every-legacy-case.md (context)"
review_set: subset
evaluated_revision: "103461726b52387c4c4922c1b707b1fd3e400997"
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-517
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-518
    type: references
---

# Spec review of the FR-517 and FR-518 amendments, PR 2160

## Summary

Ticket: QSL-1546. The PR edits FR-518 rows 8, 12, 13, 14b, 15b, the counts (31 cases: 22 reader, 9 selection), the
"Moved cases" paragraph and AC-4, and adds a paragraph to FR-517 (R is three packages; `Workflow` becomes `Attempt`
and `Recovery` inside the event-position unit). `quire validate` on both files exits 0 (module duplicate-archetype
warnings only).

Evidence for the amendments: reproduced. The writer self-check runs every case through the real reader; the
reader outcomes in the table (`unknown_member` at `/lock/definition_selections/0/digest(_domain)`, `unknown_profile`/
`unsupported-selection` at the clause law definition, `invalid_semantic_graph` at the node `declaration`) are the
values `every_case_gets_the_refusal_of_its_row` asserts against the real reader, and the test passes. FR-518 itself
says a code or pointer that differs is amended "in the same change", so rows 14b and 15b (`malformed_wire` to
`unknown_member`) are within that clause and need no more review than this.

What goes beyond that clause: rows 8, 12 and 13 change stage (selection to reader), change counts and AC-4, and so
change where legacy rows 8 and 14 end up. Merged review SR-2026110215394 (QSL-1501, PR 2144), FND-019 folded legacy
14 and 15 into rows 12 and 29 "axes definition.identity and definition.authority", and the planner ruling was
"nothing dropped, successor for each moved case". After this PR no case fires `clause.name`, `definition.identity` or
`definition.revision`; the property "a package the reader admits whose profile or name differs from the sidecar is
refused" is tested nowhere (SR-2160001 FND-001, mutation proof).

Per the process (spec edits land as their own reviewed spec PR before code), the stage-moving amendments should be
split out. Recommendation: SPLIT. Move FR-518 rows 8, 12, 13 with their counts, AC-4, the "Verification status" and
"Moved cases" text, and a matching TC-1313 update into a spec PR that carries a planner ruling on the successor for
`clause.name`, `definition.identity` and `definition.revision`; keep the 14b and 15b code corrections and the FR-517
note with the code PR. The code PR then rebases on the spec PR.

## Verdict

NOT READY as written: three medium findings, two low. The reader outcomes are correct; the consequence of the
amendments (dropped coverage, stale TC-1313) is not resolved.

## Findings

| ID | Severity | Summary | Refs | Escape Cause |
| --- | --- | --- | --- | --- |
| FND-001 | medium | TC-1313 is not updated: row 2 still lists the reader rows as 1 to 7, 14b, 15b, 16, 22 to 27, 28m, 30, 31; row 3 still lists the selection rows as 8 to 13, 17 to 21, 29; row 4 still expects rows 12 and 29 "admitted, then `definition.identity` and `definition.authority`". All three contradict the amended FR-518 (reader rows 1 to 8, 12, 13; selection rows 9 to 11, 17 to 21, 29; row 12 refused as `unknown_profile`). | spec/test-cases/TC-1313 Test Procedure rows 2 to 4; FR-518-AC-4 | wrong-requirement |
| FND-002 | medium | Rows 8, 12 and 13 move to the reader stage, so legacy row 8 (declared name differs) and legacy 14 (identity differs, folded onto row 12 by SR-2026110215394 FND-019) lose the selection-stage destination the merged review set, and the axes `clause.name`, `definition.identity`, `definition.revision` stay in the Outputs list with no case and, per the code, no reachable path. The sentence "a separate case would repeat them, so none is added and no coverage is dropped" is no longer true for identity and revision (the reader refuses a different profile as unknown, not as a departure from the sidecar), and no successor is named for row 8. This is a scope change against the planner ruling "nothing dropped" and needs a ruling, not a table edit. | FR-518 rows 8, 12, 13; "Moved cases for rows 14, 15 and 28" | wrong-requirement |
| FND-003 | medium | Row 8's amended mutation (add a `declaration` member naming the clause `Renamed` to a `temporal_clause` node) tests that the reader refuses an unexpected member, not that a declared name differs from the sidecar; and "Verification status" still says rows 8 to 13 "restate codes and axes the current reader and `Selection::check` give", which the amendment itself shows is false for rows 8, 12 and 13. | FR-518 row 8; Verification status paragraph | wrong-requirement |
| FND-004 | low | "Rows 8, 12 and 13 were selection rows before the writer self-check ran them against the reader." is a build-log sentence; the spec should state the system behavior and rationale only. | FR-518 corpus table closing paragraph | wrong-requirement |
| FND-005 | low | The FR-517 note says only the `Workflow` object becomes `Attempt` and `Recovery`; the port also replaces the `Full` and `Partial` receipt-sum conditions with receipt counts (SR-2160001 FND-003), so the note does not describe how R now differs, and the FR-517 Outputs table still lists `inventory.json`, which this PR does not write. | FR-517 note after the recipe table; Outputs table | missing-requirement |


## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | deferred | The PR no longer edits FR-518 rows or TC-1313; TC-1313 rows 2 to 4 and FR-518 rows 8, 13, counts and AC-4 must change together in the spec PR the coordinator is authoring (code FND-011). |
| FND-002 | deferred | The stage-moving amendments left this PR. Evidence for the spec PR from the real reader: `definition.identity` fires when clauses, lock selection and binding rows move to another registered profile (row 12 stays a selection case); `definition.revision` cannot (reader registers `/v1` only, row 13 is a reader case, `unknown_profile`); `clause.name` cannot (node carries no declaration, row 8 is a reader case). The spec PR needs the planner ruling on `definition.revision` and `clause.name` successors. |
| FND-003 | deferred | Row 8 mutation text and the Verification status paragraph are in the merged text and change in the spec PR. |
| FND-004 | fixed | The build-log sentence is no longer in this PR. |
| FND-005 | fixed | The FR-517 note now lists each departure with its ticket (QSL-1555 to QSL-1558, QSL-1522); inventory.json is covered by the ignored full-list test (QSL-1512). |
| FND-001 | fixed | Spec PR 2161 merged: TC-1313 rows 2 to 4 and 11 match the amended FR-518 (rows read from the manifest, row 4 over rows 12 and 29, row 11 gated on IR-1278). |
| FND-002 | fixed | FR-518 as merged keeps row 12 a selection case, makes rows 8 and 13 reader cases with successors FR-518-AC-11 (IR-1278) and AC-12, and lists the 16 axes; the revision and name properties have a named test or gated test. |
| FND-003 | fixed | Row 8 text and the verification paragraph are amended in the merged FR-518. |

## Review method

quoin spec-review base pass over the two spec diffs, reading FR-517 and FR-518 in full and TC-1313; compared with the
merged review decisions on QSL-1501 (SR-2026110215394 rounds 1 to 3); reproduced the reader outcomes by running the
tests. Sub-analyses not run: the diff changes table cells and an AC text without new relationships or new requirement
statements, so EARS, dependency and object analyses have no new scope.

## Verification

`quire validate --scope .` on FR-517 and FR-518: exit 0. Counts recomputed by hand from the table: 22 reader, 9
selection, 31 total, equal to the tests' `(21 + 28m, 9)`.

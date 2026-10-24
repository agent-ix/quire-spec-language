---
id: SR-7866
title: "QSL-736 PR 796 gap analysis: TC-235 member paths as typed values"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1e6ac7d34879ab98616ae75956ed7909c7ff05e6; QSL-736 AC1-AC3; QSpec TC-235 rows M03, M04, M05, M06, M07, M08, M09, M11, M12, M18, M19, M21; qsl-semantics/tests/it/tc235_business_meanings.rs; qsl-semantics/tests/it/tc235_state_machine.rs; qsl-semantics/tests/it/tc235_record_event.rs; qsl-semantics/src/model/refusal.rs"
review_set: subset
---

## Summary

Plan completion: not assessed

Ticket: QSL-736. This is a manual check of the ticket's acceptance criteria
against the QSpec TC-235 rows (QSpec checkout `qspec-main-1498`) and the
tests and source at the reviewed sha. No computed matrix, build or test run
was used.

- AC1 (IntakeMalformedDeclaration, and any sibling cause the rows name a
  member path for, carries a typed path): met for `IntakeMalformedDeclaration`
  and `UnsupportedDeclarationForm`. Not met for `conflicting-binding` or
  `derivation-conflict`, although the rows name paths for both (FND-001).
  `missing-name` already carries a typed `JsonPointer` with the same
  information, which is accepted here.
- AC2 (M04, M08 and M09 assert the path as a typed field, and no
  detail-substring assertion is added): met. The PR removes four
  detail-substring path assertions and adds none.
- AC3 (lib and it tests N>0, no new failure): not verified by this review.
  See the gate note in the verdict.

## Verdict

PASS WITH FINDINGS. M03 runs 1-2, M04 runs 1/2/4/5, M05 runs 4/6, M06 runs
1-3, M07 run 5, M08 runs 1-4/6/7, M09 runs 1-4/6, M11 runs 1-6, M12 and M21
runs 3-4 now assert the typed `member` with `==`, so a wrong path fails the
test. The full gate still has to cover:

- the qsl-semantics lib and `it` suites;
- `qsl-bench` and `qsl-replay`, which match on these variants;
- all-features clippy;
- `make ci` on the head as it stands after it is updated to the current base.
  The base now includes #793 and #794; textual merge is clean.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | AC1 sibling-cause gap. The rows name member paths for `conflicting-binding` (M06 run 5 `TypeDefinition.states[3]`, run 6 `TypeDefinition.transitions[3]`, M07 run 3 `TypeDefinition.steps[3]`) and for `derivation-conflict` (M18 run 2, M19 runs 2-3). `ConflictingBinding { key }` and `DerivationConflict` carry no path. M06 runs 5-6 assert no path at all; M07 run 3, M18 run 2 and M19 runs 2-3 still assert detail substrings. | qsl-semantics/src/model/refusal.rs:543-546; qsl-semantics/tests/it/tc235_state_machine.rs:231-256; qsl-semantics/tests/it/tc235_business_meanings.rs:649-669; qsl-semantics/tests/it/tc235_business_meanings.rs:1686-1693 |
| FND-002 | medium | Every converted row uses one-element arrays, so no test can catch the hard-coded or counted indices (`operations[0]` after a key sort, `relationships[0]`, the counted `clauses[n]`; see SR-7865 FND-001/002/005). No test has a refused member at position 1 or later. | qsl-semantics/tests/it/tc235_business_meanings.rs:1290-1365; qsl-semantics/tests/it/tc235_record_event.rs:693-703 |
| FND-003 | low | Pre-existing; the touched helpers expose it. The node the tests assert differs from the row in M21 run 3 (row `Fulfilment/store`, test `relationship/Fulfilment-store-OrderStore`), M21 run 4 (row `Money/owner`, test `Money`) and M07 run 5 (row `Fulfilment`, test `Fulfilment/reserve_stock`). Either the rows or the code are wrong; the typed path is right in each. | qsl-semantics/tests/it/tc235_record_event.rs:687-703; qsl-semantics/tests/it/tc235_business_meanings.rs:671-684 |

## New findings (disposition pass 1)

Reviewed at agent-ix/quire-spec-language@77a645298.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | The counted type-owned `clauses[n]` position has no test at n>0. M08 run 7 and M09 run 6 each add one clause, so a wrong count would pass. Operations and relationships now have two-element tests. | qsl-semantics/tests/it/tc235_business_meanings.rs:1411-1419; qsl-semantics/tests/it/tc235_business_meanings.rs:1525-1533 |
| FND-005 | low | QSpec-side. TC-235 M21 runs 3-4 (and M10 runs 7-8) name relationship nodes `<owner>/<role>` (`ix://test/orders/Fulfilment/store`, `ix://test/orders/Money/owner`). FR-154 and FR-154-AC-9 give a relationship the identity `ix://<package>/relationship/<Name>-<verb>-<TargetName>`, never owner-nested, and the reader refuses any other form. The rows mean the relationship member itself. The code now names the FR-154 relationship key in both runs, which is consistent with FR-154 and between the two runs. The rows need correcting in QSpec. | qsl-semantics/tests/it/tc235_record_event.rs:681-703 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 77a645298 |
| FND-002 | fixed | 4d84820a5 (clauses remain, see FND-004) |
| FND-003 | fixed | 77a645298 (M21 row spelling is FND-005) |
| FND-004 | fixed | 11bb95903 |
| FND-005 | deferred | QSpec-side row defect, out of scope for this QSL PR. The test now names the divergence at M21 runs 3-4. M10 runs 7-8 have no such comment. No QSpec ticket exists yet. |

---
id: SR-940
title: "QSL-353 spec review of PR 553, FR-093-AC-19 and TC-416 step 11"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6d5e539395f42434e641fb9854eee4b94140931a; spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md, spec/test-cases/TC-416-emission-writes-the-nodes-check-lowered.md, spec/tests.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: reviews
---
## Summary

Ticket: QSL-353. Base review of the spec diff: the new FR-093-AC-19 row, the FR-093
TC-416 backing sentence, TC-416's scope, step 11, its expected result and status line,
and the tests.md TC-416 row.

- The TC-416 scope, status line and tests.md row are consistent with each other and
  with the test.
- Step 11's expected result is checkable, and it matches the test's assertions.
- AC-19 cites Linear ids (STD-129, QSL-247, IR-450). That matches FR-093's existing
  practice (IR-280, IR-242).
- No conflict with main: #550 and #551 do not touch FR-093 or TC-416.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-093-AC-19 is scoped to the families the emission writes "for the emit test fixtures". So the AC's coverage is whatever the fixtures happen to hold, and a corpus missing families still satisfies it. The six uncovered families (SR-939 FND-001) do. The intent in QSL-353 is every family QSL can emit. State it as every (`node_tag`, `semantic_form`) the lowering can write, or every IR `CheckedNodeKind` classified as admitted, a named gap, or not written by QSL. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:719 |
| FND-002 | low | AC-19 says "Two cases are refused", but the compound unit case is omitted by the emitter, and IR never sees it. Only the STD-129 case is refused. "until QSpec or QSL rules" is also a schedule clause in a requirement. State what the emission does today for each case. | spec/functional/FR-093-lower-checked-value-expressions-to-fr-322-terms.md:719 |

## Verdict

The TC and the tracking changes are sound. AC-19 is too weak to carry the ticket's
intent (medium), and one wording inaccuracy (low). Fix both in the same round as SR-939
FND-001.

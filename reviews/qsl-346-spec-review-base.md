---
id: SR-925
title: "QSL-346 spec review of PR 546 status edits (TC-166, FR-062, FR-065, ADR-012 §14.1, tests.md, spec.md)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6260464f3408e75aea2126a681f638b933690474; spec/test-cases/TC-166-replay-executor-typed-name-selection.md (Status); spec/functional/FR-062-implement-checked-family-contract.md (AC-10 row, count sentence); spec/functional/FR-065-migrate-function-application-to-checked-family.md (AC-6 row, count sentence); spec/decisions/ADR-012-semantic-family-extension-contracts.md (§14.1 row); spec/tests.md (TC-166 row); spec/spec.md (FR-062, FR-065 rows)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: reviews
---
## Summary

Ticket: QSL-346. PR: quire-spec-language#546 at 6260464f.

The FR-065 claim "All eight of this requirement's Acceptance Criteria are backed" is
true: the AC-1 through AC-8 rows each read "backed". The FR-062-AC-10 and FR-065-AC-6
rows name the real tests and doctests. The tests.md TC-166 row and the ADR-012 §14.1
"Done" row match what runs. No stale "Planned; QSL-346" or "TC-166 has zero tests"
text is left in spec/.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-062's count sentence, edited by this PR, now reads: "Twelve of this requirement's thirteen Acceptance Criteria are backed ... one (AC-11) is partly backed, for the clause named in its own row above". The AC-11 row reads "backed (`TC-381`)" and names no partial clause. All thirteen per-AC rows read "backed". spec.md's FR-062 row, also edited, says the opposite: AC-11 "is backed" and AC-4 "is partly backed". The FR-062 AC-4 row says plain "backed". Both rows also still lead with "not yet implemented". The inconsistency predates this PR, but the PR rewrote both lines and carried it forward. Fix: decide which AC, if any, is partly backed, and make the count sentence, that AC's row and the spec.md row agree. | spec/functional/FR-062-implement-checked-family-contract.md:439,594-610; spec/spec.md:487 |

## Verdict

The status edits for AC-10, AC-6 and TC-166 are accurate. One low finding: FR-062's
partly-backed AC is named inconsistently in three places. Fix it in this PR.

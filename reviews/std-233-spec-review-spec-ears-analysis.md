---
id: SR-13991
title: "EARS review of STD-233 FR-205 key-member statement (PR #1189, QSpec PR #329)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@e2239ccee1cd807c6f824fb928d90717f6798981; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md; agent-ix/quire-specification@d2b6b37bea68292b7b74ee5208b4916b3598166f; spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md, spec/functional/protocol/FR-426-apply-protocol-steps.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
---
# SR-13991: EARS review of STD-233 (PR #1189, QSpec PR #329)

## Summary

Ticket: STD-233. The one changed requirement statement is FR-205's
`ProtocolKey` bullet: "The key SHALL also hold, as FR-425 states: for each
compensation thread, the registration it compensates; for each protocol
instance in which a step of a template's commit node has occurred, that node;
and for each compensation thread that has taken a `cattempt`, the record of
its previous attempt, until its `cend`." The other edits are not EARS
statements: the ADR-027 PS-8 and ST-2 rows, the FR-425 key table, the FR-426
step table and the AC rows. `make check-spec-validation` reports no EARS
warning on FR-205.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. The FR-205 sentence is a ubiquitous statement with one subject, "The
key", and one SHALL whose object is a list of three members. The "as FR-425
states" clause is a cross-reference, not a second obligation. The
substantive problems with the third member are in SR-13990 (FND-002 and
FND-003).

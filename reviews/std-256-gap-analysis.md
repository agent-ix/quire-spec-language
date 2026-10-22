---
id: SR-14852
title: "gap analysis of STD-256 new and changed ACs against TC rows (PR #1200, QSpec PR #353)"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@6e35052ff7072d7486001ff9721d5e5a0bad0fe9; spec/functional/FR-205-define-the-protocol-subject-and-its-state-key.md, spec/functional/FR-207-run-compensation-templates-as-protocol-threads.md, spec/functional/FR-216-give-every-protocol-step-a-static-footprint.md, spec/test-cases/TC-650-protocol-subject-and-state-key.md, spec/test-cases/TC-652-compensation-templates.md, spec/test-cases/TC-661-protocol-step-footprints.md; agent-ix/quire-specification@21ff693e1113a6d69e501fab5300552a8ead515b; spec/functional/protocol/FR-425-define-protocol-subjects-and-state-keys.md, spec/functional/protocol/FR-427-check-protocol-compensation.md, spec/test-cases/TC-375-protocol-state-key-and-coalescing.md, spec/test-cases/TC-380-compensation-worked-example.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: reviews
---
# SR-14852: gap analysis of STD-256 (PR #1200, QSpec PR #353)

## Summary

Ticket: STD-256. Plan completion: not assessed. This is a manual mapping of
each new or changed AC to its TC rows. It covers spec only. The code for these
ACs is in the code lane's remaining work, and the code refuses
`UnboundAttemptRecord` today, so no test exists yet. Mapping:

- FR-205-AC-8 (retry part restored) maps to TC-650 step 8, with its procedure
  and expected result extended. The Scope is AC-1 to AC-8.
- FR-207-AC-6 (new) maps to TC-652 step 6 and its expected result. The Scope
  is now AC-1 to AC-6.
- FR-216-AC-7 (new) maps to TC-661 step 7 and its expected result. The Scope
  is now AC-1 to AC-7.
- QSpec FR-425-AC-12 (retry part restored) maps to TC-375 PK-16.
- QSpec FR-427-AC-7 (new) maps to TC-380 CP-13.

The changed Behavior text in FR-205, FR-207 and FR-216 is covered by these
ACs: the key entry, enumeration and identity, the commit write and the
reference read. The FR-426 `record` row and the FR-427 retention prose are
covered by FR-427-AC-7 ("identities differ only in `record`") and by
FR-425-AC-12.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Clean. Every new or changed AC has a TC row in its own repo, and each TC row's
expected result states what its AC asserts.

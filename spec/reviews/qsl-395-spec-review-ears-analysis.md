---
id: SR-1041
title: "QSL-395 EARS review of PR 586 requirement statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@4dffb8b83bd8c32bdba78f39b536fc9ed716ffe8; requirement statements edited in git diff origin/main...HEAD: FR-061, FR-099, FR-101, FR-106, FR-110, FR-111, FR-115, FR-116, FR-122, NFR-001, NFR-011"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-061
    type: reviews
---
## Summary

Ticket: QSL-395. PR: quire-spec-language#586 at 4dffb8b8. Checked the
normative statements this PR adds or rewrites for EARS form. Most edits
delete a clause from an existing statement and leave its EARS form as it
was. The edited statements in FR-099, FR-101, FR-106, FR-110, FR-111,
FR-115, FR-116, FR-122, NFR-001 and NFR-011 keep a trigger, a subject and a
response. The rewritten FR-061 Behavior has one statement with no subject,
and `quire validate` reports it too (`ears:missing-subject`,
`ears:unclassifiable`).

## Verdict

One low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "It SHALL report a component whose distinct `source` values number more than one." names no subject, so EARS cannot classify it. It should name the check as the subject. | spec/functional/FR-061-check-duplicate-ecosystem-revisions.md:42 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | b052250b (FR-061 deleted; the statement no longer exists) |

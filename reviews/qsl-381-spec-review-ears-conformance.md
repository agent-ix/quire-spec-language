---
id: SR-1073
title: "QSL-381 EARS and testability review of PR 577: FR-255 to FR-264 criteria"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@9d240032be4eac1df486d3c224cbd48273053dc5; spec/functional/FR-255..FR-264 statements and ACs; TC-720..TC-739; amended FR-018-AC-6, FR-062-AC-7, FR-082-AC-3, FR-083-AC-4, FR-092-AC-7, FR-096-AC-3/AC-11, FR-102-AC-5"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-261
    type: reviews
---
## Summary

Ticket: QSL-381. Check 2 of the brief: each 100,000-deep criterion is
testable. Every 100,000-deep AC names a concrete input, sets its own limits
("raised to fit"), runs on a 512 KiB-stack thread (which cannot hold one
frame per level at 100,000 deep, so a pass shows non-recursion) and states
an observable outcome; every AC maps to a TC in TC-720..TC-739 that tests
behaviour. Statements use SHALL with When/If triggers. Exceptions below.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-261-AC-1's oracle "refuses with the defect of that member's value" names no code or cause, so a test cannot assert the refusal and any refusal other than not-an-object passes. Name the code/cause the library identity read returns for that member. | spec/functional/FR-261-read-other-untrusted-json-at-any-depth.md:56 |
| FND-002 | medium | FR-260-AC-1's oracle "refused with the schema defect at that member" names no code, cause or location form. Name the semantic-IR refusal and its locus. | spec/functional/FR-260-admit-semantic-ir-documents-at-any-depth.md:52 |
| FND-003 | low | FR-264-AC-3 says "refused ... as a malformed wire" and Behavior 3 says `refused`, but QSpec FR-322-AC-40 names the code `malformed_wire`. State the code so the test asserts it. | spec/functional/FR-264-emit-and-read-v2-packages-at-schema-fixed-depth.md:45,56 |
| FND-004 | low | FR-256-AC-2 expects "brackets nested to the depth the default token limit admits" to parse, but the default node limit (50,000) is below the token limit (100,000) and may bind first. Say "the depth the default S1 limits admit". | spec/functional/FR-256-parse-source-at-any-nesting-depth.md:52 |

## Verdict

Changes requested (two medium). The 100,000-deep criteria are testable;
two deep-document ACs lack a precise refusal oracle.

## Dispositions

Round 1, reviewed at 76e47d3a96aacde91f7964e872d50dcd87dfbc15.

| FND | Outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 16a9154f: FR-261-AC-1 names `PreimageDefect::MemberType("edition")`. |
| FND-002 | fixed | 16a9154f: FR-260-AC-1 names FR-056's reader-refusal rule and the retained diagnostic. |
| FND-003 | fixed | 16a9154f: FR-264-AC-3 names `malformed_wire`. |
| FND-004 | fixed | 16a9154f: FR-256-AC-2 builds each chain to the longest length the default S1 limits admit. |

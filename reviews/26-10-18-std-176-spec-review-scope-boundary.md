---
id: SR-12894
title: "Scope-boundary review of PR #986: what FR-165 owns versus QSpec's graph-predicate"
type: SpecReview
analysis: scope-boundary
scope: "agent-ix/quire-spec-language@2cb28aa4b; FR-165 Parsing (S1) and Dependencies; ADR-022 SG-4, SG-5, QS-1 and the paragraph after the SG table; agent-ix/quire-specification@origin/main 4f65ed2 shared-grammar state-graph section"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-165
    type: reviews
  - target: ix://agent-ix/quire-spec-language/ADR-022
    type: reviews
---

## Summary

Ticket: STD-176. PR quire-spec-language#986. This review checks the
ownership boundary. QSpec owns the surface grammar and its refusals
(ADR-022 QS-1, SG-5). QSL owns S1 recognition, S3 refusal and the record.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-165 says it "does not restate" `graph-predicate`, and then lists what S1 refuses as a closed SHALL list ("what `graph-predicate` does not recognise: ..."). QSpec instead lists examples under "Everything else is a syntax error". The closed list leaves some unrecognised placements without a stated outcome, such as a temporal operator as a comparison or arithmetic operand (`c.x = eventually c.y`) or in a `case` arm. If QSpec adds a case, the two lists drift apart. Reading it as "everything `graph-predicate` does not recognise, including: ..." would keep QSpec as the only owner. | spec/functional/FR-165-check-state-graph-claims-at-s3.md:91-93,101-109; spec/decisions/ADR-022-possible-properties-and-state-graph-queries.md:98 |

## Verdict

The boundary is sound. The PR removes the earlier claim that QSL widens the
grammar "ahead of QSpec" and the STD-171 hand-off. It now names QSpec's
production, FR-390 and SG-R09 to SG-R18 as the reference. S3 keeps only the
admission refusals that FR-390 assigns to the checker. ADR-022 SG-4 and QS-1
agree. The single low concerns how the restated list is worded, not who
owns the rule.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | 0a710fa5e reads "S1 SHALL refuse with `invalid_syntax` everything `graph-predicate` does not recognise, including:", and adds the comparison-operand case `c.x = eventually c.y` (AC-6 vector `{ c.versionNumber = eventually c.versionNumber = 2 }`). |

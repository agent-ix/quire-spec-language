---
id: SR-13593
title: "ears-conformance review of QSL-545 refinement history spec follow-ups (PR #1112)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@cc890d9998cd4ca09d16522620b1ef03907ba528; spec/functional/FR-120-simulate-a-checked-package-s-state-family.md, spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md, spec/functional/FR-138-check-and-compute-history-fields.md, spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-120
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-149
    type: reviews
---
# SR-13593: ears-conformance review of QSL-545 (PR #1112)

## Summary

Ticket: QSL-545. This review covers the requirement statements the diff
adds or edits: the FR-120 incomplete-clause bullet, the FR-135 support-set
and payload bullets, the new FR-138 S3 bullet, and the FR-149 History
bullet. The engine check (`make check-spec-validation`) gives no
`[ears:*]` warning on any touched file. Semantic review found three low
findings. In each, a statement names the wrong actor or leaves the actor
out. None changes what gets built.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new FR-138 bullet makes the type the subject: "A declared type that is an enum type ... SHALL refuse". The checker (S3) refuses; the type does not. Write "S3 SHALL refuse a declared type that is ...". | spec/functional/FR-138-check-and-compute-history-fields.md:65-67 |
| FND-002 | low | The FR-135 support-set bullet holds three obligations ("SHALL hold", "SHALL omit", and "it SHALL refuse"). In the third, the `history` row is the actor that refuses. Split the refusal into its own statement with S3 as the subject. | spec/functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md:199-207 |
| FND-003 | low | FR-149's History bullet opens with an agentless passive: "A position SHALL be identified by its concrete state key ...". The sentences after it name the checker and the producer. Name the actor here too, for example "the checker and the producer SHALL identify a position by ...". | spec/functional/FR-149-check-a-refinement-s-simulation-certificate.md:178 |

## Verdict

Mergeable as far as EARS goes. All three findings are wording fixes and
can go into the fix round for SR-13594. The FR-120 bullet is declarative
explanation inside an existing list of rules and needs no `SHALL`.

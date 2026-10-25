---
id: SR-17192
title: "QSL-926 PR #1524 EARS conformance review of the on each rules"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md, spec/functional/FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: reviews
---

## Summary

Ticket: QSL-926. PR: quire-spec-language#1524 (head a8b073ce6). `quire validate` emits no `ears:*` warning for a changed file. Read by hand, the S3 bullets of FR-337 and the first three FR-125 bullets are ubiquitous SHALL statements with a named subject (S3, the emitter, the letter seam).

Examined units: every bullet of FR-337 "`on each` activation at S3", "`on each` activation at S4" and "Clauses S4 does not emit", and every bullet of FR-125 "The `on each` activation guard".

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Normative bullets stated as fact with no SHALL. FR-125: "The engine reads the activation guard only for a clause whose activation has one" and "FR-128's replay and FR-338's and FR-339's checkers read the guard through the same seam". FR-337 S4: "The guard and the trigger node enter the clause node's content identity ..." and "The emitted clause's identity triple and `over` node are those of Outputs". Each states behaviour an implementation must have. Restate each as "<subject> SHALL ...". | FR-125:278-283, FR-337:208-215 |

## Verdict

Conforms, with one low wording finding.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The FR-125 bullets now read "The engine (FR-126) SHALL read ...", "the seam SHALL return an empty guard footprint" and "... checkers SHALL read the guard ...". The FR-337 S4 bullets read "The emitter SHALL key the clause node ..." and "The I2 reader SHALL return ...". |

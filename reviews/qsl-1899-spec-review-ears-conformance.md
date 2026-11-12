---
id: SR-2690002
title: "ears-conformance review of FR-576 operation-body carrier (PR 2690)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-576-emit-the-operation-body-into-checked-packages-for-backend-consumers.md"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-576
    type: reviews
---

## Summary

Ticket: QSL-1899. Reviewed the six FR-576 Behavior SHALL statements for EARS
form. Four are acceptable (the `has_body` false rule, the keying rule, the
sourcing rule, and the FR-302 refusal rule). `quire validate` flags the first
bullet, and one statement makes a passive artifact its subject.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The first Behavior bullet ("For every admitted operation occurrence whose `has_body` is true, the emitter SHALL write ...") is flagged by `quire validate` as `ears:unclassifiable` and `ears:missing-subject` at line 61. Restate it as a state-driven rule, for example "While an admitted operation has `has_body` true, the emitter SHALL ...". | FR-576 Behavior bullet 1 (line 60-63) |
| FND-002 | low | "The checked package SHALL let quire-driver read the record ..." makes a passive artifact the subject. The real obligation is either the emitter's (write a complete, self-contained record) or quire-driver's (its typed read), and FR-576's own Description gives the read to quire-driver. | FR-576 Behavior bullet 5 |

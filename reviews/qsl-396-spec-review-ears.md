---
id: SR-1012
title: "QSL-396 EARS review of FR-205 to FR-218"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@4873939885cbe47d6da1bf6d47074d695dd0586f; spec/functional/FR-205..FR-218"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
---

## Summary

Ticket: QSL-396. This review checks each FR's Description and Behavior
statements, FR-205 to FR-218, against EARS requirement grammar.

- Event-driven statements ("When the request names a protocol that is not…, the subject builder SHALL refuse…") name a trigger, a system and a response.
- State-driven statements ("While another thread of the instance is `running`, `finish` SHALL NOT be enabled") name the state.
- Ubiquitous definitional statements ("Each ordinal SHALL be the smallest natural number…") follow the repo's established FR style, as in FR-120 and FR-126.
- Each FR has a "Use case" section, and each traces to US-024.
- No statement names an unsupported alternative.
- No statement carries a delivery order.
- No requirement prose or AC holds a ticket id.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

The statements conform to EARS. The semantic and consistency findings are in
SR-1010 and SR-1011.

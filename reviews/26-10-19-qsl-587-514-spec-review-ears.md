---
id: SR-13062
title: "EARS conformance review of PR #1020: FR-307 new export rule and FR-307-AC-6"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@66ef5a3e3; FR-307 Behavior (Per-item results, new bullet), FR-307-AC-6, FR-307 Dependencies; FR-251-AC-4 to AC-8 (text unchanged, Verification cells only)"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-307
    type: reviews
---

## Summary

Tickets: QSL-587, QSL-514. PR quire-spec-language#1020. The only new
requirement statement is FR-307's partition bullet; FR-251's AC text is
unchanged in the diff (only Verification cells). The Dependencies addition is
context, not a requirement.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The new Per-item results bullet states its rule without SHALL ("The bound items are exactly ...", "the export still returns one refusal per item") while every neighbouring bullet uses SHALL, so it reads as commentary rather than an obligation. Rewrite as "When every requested item is refused, the export SHALL succeed and return one refusal per item" and move the driver-selection clause to rationale. | spec/functional/FR-307-export-the-bindings-each-item-references.md:79-82 |

## Verdict

One low wording finding; otherwise conformant.

---
id: SR-7881
title: "QSL-737 PR 800 EARS conformance: FR-359"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language@67de856995d7f102f050fad3275c167e4f66b26d; spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md"
review_set: subset
---

## Summary

Ticket: QSL-737. This analysis checks FR-359's requirement statements against
EARS form, measured against the repository's own FR layout (Description,
Inputs, Outputs, Behavior, Acceptance Criteria, Dependencies; compare FR-357
and FR-358, which carry explicit `SHALL` statements).

## Verdict

CONDITIONAL. Items 1 to 4 and 8 of the target design are normative `SHALL`
statements, and the ACs are concrete. Two defects remain:

- The FR has no requirement statement of its own.
- Items 5 to 7 are indicative only.

## Examined Scope

```yaml
scope:
  - {id: FR-359, path: spec/functional/FR-359-carry-a-child-record-value-at-an-ancestor-typed-slot.md, role: examined}
  - {id: FR-358, path: spec/functional/FR-358-settle-a-composite-bounded-shadow-item.md, role: context_only}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | FR-359 has no top-level requirement statement. Its Description is narrative ("This requirement states what the rest of QSL does with that value"). Items 5 ("all admit a child"), 6 ("admit a child ... by the same conformance rule") and 7 ("Replay decodes every record by ...") are indicative, with no `SHALL`. So the Option, collection, argument and replay obligations are binding only through their ACs. An event-driven statement would fix this, for example: "When a record value whose most-specific type conforms to a slot's record type reaches that slot at check, evaluation or replay, QSL SHALL ...". | FR-359 Description; FR-359 items 5-7 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d449b0273 |

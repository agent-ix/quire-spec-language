---
id: SR-2026111305495003
title: "ears-conformance review of QSL-1901 postcondition range facts (FR-577)"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-577-discharge-a-postcondition-s-range-obligations-from-its-operation-s-preconditions.md; spec/functional/FR-104-check-state-clauses.md; spec/functional/FR-121-locate-a-function-call-site-through-the-replay-facade.md; PR quire-spec-language#2696"
review_set: subset
---

## Summary

Ticket: QSL-1901. EARS grammar check of every new or amended requirement
statement in FR-577, FR-104 (the arithmetic-node fact sentence and the new
definedness-fact bullet) and FR-121 (the `region` bullet). Most statements are
ubiquitous or event-driven (`When the checker checks ... it SHALL`) and have a
named responsible system and one response each. The FR-121 `region` bullet and
the FR-104 amendments conform. Three FR-577 statements mix explanatory prose
into requirement bullets or have no system subject.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "No other relational reasoning SHALL be admitted" is passive and names no responsible system (the checker). | FR-577:124 |
| FND-002 | low | "The range proof of a postcondition is a proof only for invocations whose preconditions hold" is a declarative rationale sentence with no SHALL, placed in the Behavior bullet that holds the evaluator's SHALL NOT clamp requirement. It should be moved to Description or rewritten as a requirement. | FR-577:94-95 |
| FND-003 | low | The transfer-key bullet mixes two SHALL statements with three explanatory sentences ("Precondition reads are `pre`...", "So such a fact matches...", "A parameter is an immutable input..."). These read as further normative rules but carry no SHALL, so it is unclear which sentences are binding. | FR-577:74-80 |

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | FR-577:163 now reads "The checker SHALL admit no other relational reasoning." |
| FND-002 | fixed | The rationale moved to the Description (FR-577:52-62). The evaluation bullet (FR-577:126-130) is now an event-driven "When an evaluation of `Q` reaches ... the evaluator SHALL refuse". |
| FND-003 | fixed | The transfer-key bullet is now separate SHALL statements (FR-577:102-105), and the explanatory sentences are in a plain paragraph after the list (FR-577:121-123). |

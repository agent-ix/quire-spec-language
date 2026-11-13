---
id: SR-2026111306075604
title: "EARS review of PR2697 FR-570, FR-574 and FR-575 statements"
type: SpecReview
analysis: ears-conformance
scope: "agent-ix/quire-spec-language; spec/functional/FR-570-*.md, spec/functional/FR-574-*.md, spec/functional/FR-575-*.md; PR2697 c7587607454cf151473007cd14cd87fdd7b08286"
review_set: subset
---

## Summary

Ticket: QSL-1895 (ADR-036 slice 3). EARS check of the requirement statements the PR adds or edits. The run-limit, horizon and global-event bullets are testable and use measurable values. The FR-574 trace-position row is exact. Two wordings are unclear.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | "as external choice when a peer sends a receive in every branch's first events" is garbled (a peer sends a message; the role performs the receive). Write "when every branch's first events hold a receive by the role from one peer, and the message types differ across branches". | spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:76-77 |
| FND-002 | low | The FR-570 "At a choice" bullet carries a parenthetical justification ("a checked package's `visible` holds expression nodes, so a role lies in `visible` exactly by taking part in such a message") inside the shall-statement. That is rationale, not behaviour, and "lies in `visible`" no longer names anything. Move it to a note and cite QSpec FR-466 "Decision facts". | spec/functional/FR-570-project-a-global-protocol-onto-one-role.md:73-75 |

## Scope and evidence

Each edited statement was read for one subject, one condition and one observable response. The run-limit bullet names both limits, both defaults, the label and the axis. The horizon bullet defines depth ("the longest chain of events ending at it"). FR-575's ordering sentence ("roles, queues, compensations, then controls in layout order") is testable.

- FR-570 Behavior "At a choice" (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md). (examined)
- FR-570 Behavior "A choice event, a send, a receive" (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md). (examined)
- FR-570 Behavior "Run limits" (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md): default to 4096 and 262144; failed, resource-incomplete. (examined)
- FR-570 Behavior "An event deeper than max_depth" (spec/functional/FR-570-project-a-global-protocol-onto-one-role.md). (examined)
- FR-574 `trace_position` row (spec/functional/FR-574-carry-the-separating-witness-record-on-a-decisive-protocol-counterexample.md). (examined)
- FR-575 `unsupported-control` row and the first-construct sentence (spec/functional/FR-575-answer-unsupported-for-constructs-outside-the-bounded-static-profile.md). (examined)

## Dispositions

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | 66bed12d8: the bullet reads "as external choice when, in every branch, the first events include a receive from one peer and the message types of those receives differ across branches". |
| FND-002 | fixed | 66bed12d8: the "At a choice" bullet is split into three shall-statements, and the rationale is a plain sentence ("a checked package's `visible` holds expression nodes, not roles"). |

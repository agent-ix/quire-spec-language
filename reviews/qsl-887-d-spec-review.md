---
id: SR-12312
title: 'Spec review of PR #1198: FR-147 Remaining-work paragraph for non-attempt node records (QSL-887)'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@41fbf4af8; git diff origin/main...41fbf4af8 (PR #1198, branch qsl-887d-impl): spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md; checked against QSpec FR-432 at agent-ix/quire-specification@97bfd65e and QSL #1191 (f37128d1a); ticket QSL-887'
review_set: subset
date: '2026-10-22'
relationships:
- target: ix://agent-ix/quire-spec-language/FR-147
  type: reviews
---

## Summary

Ticket: QSL-887. PR: quire-spec-language#1198, head 41fbf4af8. Method: spec-review with the spec-integrity sub-analysis (one edited requirement paragraph, no new statement or AC). Tooling: quoin 0.28.3, spec-artifacts-process@b82e42e.

The PR adds six lines to FR-147 "Step rows and observations", inside the bullet "The expressions of a node right side SHALL be typed against the node's binder record, whose fields per node kind are QSpec FR-432's":

> Remaining work: QSL-887 for every node kind but `attempt`. QSpec FR-432 does not yet state the binder record's fields for an `effect`, `event`, `commit`, `send`, `receive` or `finish` node, so this build gives an `attempt` node's record (ADR-027 ST-1: the receiver, the operation's parameters, then its result when it has one) and types the written expressions of the other kinds without a count or type check.

History: QSL #1191 (STD-249, f37128d1a) removed the earlier "Remaining work: STD-249 (QSpec does not yet state the binder record's fields per node kind)" from this bullet, because QSpec #330 (STD-249, c44bf2d7) added FR-432 "The binder record of an abstract node" with a row for every node kind. This PR puts the same claim back.

The `attempt` description in the paragraph is right: it matches ADR-027 ST-1 ("the binder's record (receiver, arguments, result)") and FR-432's attempt row.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-147 node typing bullet with the added Remaining-work text | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:112-119 | examined |
| FR-147 count and type refusal bullets | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:120-125 | context_only |
| QSpec FR-432 binder record table | quire-specification spec/functional/protocol/FR-432-map-every-protocol-step-in-refinements.md:85-97 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The added text says "QSpec FR-432 does not yet state the binder record's fields for an `effect`, `event`, `commit`, `send`, `receive` or `finish` node". It does: FR-432's table gives `effect`, `event`, `commit`, `finish` as "when the node's binder type `T` is a record type, one field per field of `T`, in declaration order; otherwise exactly one field" and `send`, `receive` as "as `event`, over the channel's `carries` type" (QSpec #330, merged before this PR's commit). The paragraph re-opens a deferral #1191 closed, on a false premise, and leaves the SHALL beside it unmet for six kinds. Delete it and implement those kinds (SR-12311 FND-001). | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:114-119 |
| FND-002 | low | The Remaining-work text sits inside a SHALL bullet, so the bullet mixes a requirement with a build-status note. FR-147 elsewhere puts Remaining work in its own paragraph after the bullets (line 96). If any Remaining-work note survives the fix, move it out of the bullet. | spec/functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md:114 |

## Verdict

Not mergeable as written. FND-001 is high: the spec edit states a fact about QSpec that QSpec main contradicts, and it is the only justification for the implementation gap. FND-002 is low.

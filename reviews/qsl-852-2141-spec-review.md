---
id: SR-2141001
title: spec-review of QSL-852 composed-root reader spec (PR 2141)
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language; spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md; spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md; QSL-852"
review_set: subset
---

## Summary

Ticket: QSL-852; PR 2141 at 121bf7be. Reviewed the diff only: FR-390 'Composed roots from the reader' with AC-28/29, FR-383 'Protocol term locations' with AC-32, and the scope lines of TC-1063/1070/1073. Sub-analyses applied inline: EARS, integrity, criterion-strength (Jev unavailable; manual judgement).

## Verdict

**FAIL** - not mergeable until FND-001, FND-002 and FND-003 are fixed. quire validate: no errors. No id collisions with main or open PRs.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | untestable-ac: AC-29: halting location per visit, in `earlier.tally < 3` / | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-002 | high | soundness: Removing the row of one visit, or making two visits resolve | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-003 | high | ambiguous: the k-th visit of that application, in a left-to-right pre-o | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-004 | medium | soundness: the rows of that application whose origin is the slot's site | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-005 | medium | other: It refuses no other slot of the package that the view could | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-006 | medium | exceeds: a kind that QSL emits and TC-1070's fixtures E and L use (`a | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-007 | medium | untestable-ac: a variant whose row is removed or duplicated is refused by t | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |
| FND-008 | low | other: A reader view whose source package has had its `CheckedBody` | spec/functional/FR-390-evaluate-a-protocol-node-expression-from-a-checked-package.md |
| FND-009 | low | compound: The emitter SHALL write, in the `checked_term_data.protocol_ | spec/functional/FR-383-emit-a-checked-protocol-as-v2-protocol-nodes.md |

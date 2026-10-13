---
id: SR-7858
title: "QSL-723 PR 789 spec review: FR-094 state machine row"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@0d84e0cea7b32600caa6e2f2d95b18ea13d2dd0d; spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md"
review_set: subset
---

Redacted: QSpec content removed (public repo); the unredacted record is in the private tracker.

## Summary

Ticket: QSL-723. A base spec review of the single FR-094 edit: a new `StateMachineRecord` row in the "Model declaration nodes" table. The row matches the code. It conflicts with the upstream QSpec contract it keys for (QSpec FR-322, FR-340), and no criterion backs it.

## Verdict

FAIL. FND-001 is high: the row specifies a node that QSpec checked-package-v2 refuses.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The row `StateMachineRecord` -> `model`/`state_machine` makes a model-owned node that carries a `ModelOwner` (FR-094 "Model-owned nodes"). This conflicts with QSpec FR-322, QSpec FR-340 and the checked-package-v2 owner rule (details in the private record). The row should not be published until QSpec changes. | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:106 |
| FND-002 | medium | No FR-094 acceptance criterion names a state machine. The new row is backed only by a test tagged `FR-094-AC-1`, which states `Reference<M::Order>`, `Invoice` and `billedTo`. | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:675 |
| FND-003 | medium | The "no node" list after the table is still not exhaustive. Record value type, event type, process, persistence interface, namespace and clause records also have no node in `record_form` but are not listed. QSpec FR-322 and FR-340 treat `record_value_type` differently, so FR-094 and QSpec disagree on that kind. | spec/functional/FR-094-key-model-owned-reference-population-and-quantity-nodes.md:109 |

## Coverage

Examined: the FR-094 table row (line 106), the "no node" paragraph (line 109), FR-094-AC-1 and FR-094-CON-1.

Context only: QSpec FR-322 (incl. AC-25), QSpec FR-340 (incl. AC-2/AC-3/AC-12), and checked-package-v2.

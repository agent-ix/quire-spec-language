---
id: SR-2348002
title: 'Gap analysis of PR #2348: qsl-analyze node-to-family reader (QSL-1702), gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@b90258f05; PR #2348 diff only: FR-507-AC-7, FR-507-AC-8, FR-507-AC-11 against qsl-analyze/src/claim_family.rs, qsl-analyze/src/analyze/items.rs and qsl-analyze/tests/it/claim_family.rs; ticket QSL-1702'
review_set: subset
date: '2026-11-05'
---

## Summary

Ticket: QSL-1702. This is a planless gap analysis scoped to the ACs that PR #2348 tags. Plan completion: not assessed. Each tagged AC was compared with what its test actually asserts. Code defects are recorded in SR-2348001. This file records only gaps between the ACs and the tests.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-507-AC-7 | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md | examined |
| FR-507-AC-8 | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md | examined |
| FR-507-AC-11 | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md | examined |
| FR-507-AC-9 | spec/functional/FR-507-apply-each-capability-kind-to-its-declaration-family-only.md | context_only |
| QSpec FR-290 node-to-family table | quire-specification FR-290 | context_only |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | FR-507-AC-11's Boolean value clause ("an `expression` node, once at occurrence role `claim` and once at role `declaration`") is tagged but tested on the function node. The AC's form is not exercised, and the reader would refuse it. Code side: SR-2348001 FND-001. | qsl-analyze/tests/it/claim_family.rs:478-496 |
| FND-002 | high | FR-507-AC-7's "the relation's claim occurrence (shared by its model and code claims)" is tagged, but the test asserts that the two claims are distinct occurrences (`assert_ne!`), which contradicts the AC. | qsl-analyze/tests/it/claim_family.rs:239 |
| FND-003 | medium | FR-507-AC-7's "two records at their request indices, in caller order" and "four items give four records in caller order" are not exercised. Each item is settled alone in its own `analyze_items` call. | qsl-analyze/tests/it/claim_family.rs:243-258, 369-385 |
| FND-004 | low | FR-507-AC-8's "run on a `temporal-trace` engine" and AC-7's model claim "settled by an engine" are covered only by `Engine::probe`, a stand-in that advertises every kind. The family read is verified, but no real temporal-trace engine run is. This is disclosed in the PR body, not in the tests. | qsl-analyze/tests/it/claim_family.rs:388-468 |

## Verdict

CONDITIONAL. The AC-8 derived items and the AC-11 rows other than the Boolean clause are covered with value assertions and fail on regression. Those AC-11 rows are the scalar operation, frame obligation, state-graph claim, `hyper` clause, timed and hybrid temporal claims, and the protocol-abstract refinement. AC-7's refinement half is covered too: four distinct occurrences of one node, all `relation-claim`, with `schedulability` refused for each.

The AC-7 and AC-11 tags over-claim on two units (FND-001, FND-002) and leave AC-7's caller-order clause unexercised (FND-003). Must change in this PR: remove the `assert_ne!` at claim_family.rs:239 (FND-002). Take the Boolean-clause half out of the AC-11 tag, or mark it `Remaining work: QSL-1714` (FND-001). Settle the AC-7 items in one `analyze_items` call so the test checks caller order (FND-003). Asserting the expression-node carrier and `model == code` can wait for QSL-1714/QSL-1715, because the producer does not yet emit that carrier. FND-004 can stay a disclosed stand-in.

The FR-507-AC-11 hybrid row is exercised through a temporal claim over a hybrid subject, as the AC words it. The protocol-abstract refinement row is covered: `cas_twice` has the protocol `Spec::Twice` as its abstract side. FR-507-AC-9 is not tagged by this PR. One `None` read, the function node at role `anchor`, adds evidence for it.

## Dispositions

- FND-001: fixed. The Boolean clause is out of the AC-11 tag, marked `Remaining work: QSL-1714`.
- FND-002: fixed. The `assert_ne!` is removed; `model == code` waits for QSL-1715.
- FND-003: fixed. AC-7 items settle in one `analyze_items` call with order and indices asserted.
- FND-004: accepted-no-change. The `Engine::probe` stand-in stays disclosed.

---
id: SR-20261104130234
title: 'Spec review of PR #2295: FR-337, FR-194, FR-515, FR-516, FR-532, TC-1267 aggregate emission text, spec-review'
type: SpecReview
analysis: integrity
scope: 'agent-ix/quire-spec-language@1681cae5f; PR #2295 spec diff: spec/functional/FR-337-*.md, FR-194-*.md, FR-515-*.md, FR-516-*.md, FR-532-*.md, spec/test-cases/TC-1267-*.md; QSpec cited by id (FR-370, FR-414); ticket QSL-1643'
review_set: subset
date: '2026-11-04'
---

## Summary

Ticket: QSL-1643. Spec-review with integrity and object sub-checks over the changed requirement text. It checks the text against itself and against QSpec FR-370 "Window aggregate atom" (cited by ID, read in place).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-337-AC-15 and "Aggregate formula emission" | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:519-580 | examined |
| FR-194 family bullet | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:90-99 | examined |
| FR-515 formula_graph AggregateUse | spec/functional/FR-515-read-the-checked-predicate-and-temporal-subject-from-a-checked-package.md:111 | examined |
| FR-516 aggregate prose and dependencies | spec/functional/FR-516-carry-the-temporal-evidence-request-and-result-on-the-checked-package-route.md:332-463 | examined |
| FR-532 aggregate prose and FR-532-AC-11 | spec/functional/FR-532-evaluate-temporal-evidence-from-the-checked-package-s-own-nodes.md:136-191 | examined |
| TC-1267 step 8 | spec/test-cases/TC-1267-a-temporal-clause-with-a-capture-checks-emits-and-is-observed-at-activation.md:107-118 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The new FR-194 bullet says S3 "SHALL refuse another value or threshold `ill_typed`/`type-mismatch`". The bullet directly above it requires `unit-mismatch` for a threshold of another dimension. A quantity threshold in a non-convertible unit falls under both bullets, and two readers would pick different causes; the code picks type-mismatch. Fix: say "another family" for type-mismatch and keep unit-mismatch for a quantity of another dimension (QSpec FR-370 step 5). | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:94-99 |
| FND-002 | low | FR-532-AC-11 still says `formula_graph()` returns the "expression node". FR-515's `AggregateUse` no longer has an `expression` field; it now has `value`, `predicate`, `filter`, `window_kind`, `lower` and `upper`. Fix: reword AC-11 to "its kind, value node, window_kind, lower, upper, min_count, comparison and threshold". | spec/functional/FR-532-evaluate-temporal-evidence-from-the-checked-package-s-own-nodes.md:191 |
| FND-003 | low | FR-337 item 7 types `duration` bounds by "the lower bound's written time unit". QSpec FR-370 types them by the clause's clock unit and converts both bounds to it. Today the emitter refuses every duration window, so nothing ships wrong. Once QSL-1478 lands, `past[0 s, 300000 ms]` under a clock in ms would still refuse instead of converting to ms. Fix: state the clock unit as the target, or record the restriction against QSL-1478. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:547-550 |

## Verdict

Conditional on FND-001. FR-515's `AggregateUse` now matches FR-370's binding set and presence rules (optional `q`, `predicate`, `value` and `filter`), and it agrees with FR-194's checked term, where `value` is absent for `count` and `fraction` and the predicate belongs to the `fraction` kind. The FR-337 binding list matches FR-370's table order and presence column. The duration-window refusal is stated consistently in FR-337's prose, in AC-15 and in TC-1267, with the QSL-1478 cause given. The FR-516 and FR-532 status prose is accurate: the emitter writes the node, and the locked Contract IR reader and the in-process plan still do not admit it.

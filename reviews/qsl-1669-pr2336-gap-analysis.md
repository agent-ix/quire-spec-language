---
id: SR-2336002
title: 'Gap analysis of PR #2336: FR-326-AC-6, FR-337-AC-24, FR-233-AC-2 coverage, gap-analysis'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@b619cd3abf128955efad055e687af23e9246c482; git diff origin/main...HEAD (PR #2336); ticket QSL-1669'
review_set: subset
---

## Summary

Ticket: QSL-1669. PR: quire-spec-language#2336. Method: gap-analysis over the three criteria the PR implements, planless. FR-326-AC-6 is covered value-for-value by the new TC-836 test. FR-337-AC-24 is implemented but untagged and only half asserted. FR-233-AC-2 second sentence is covered by the restated TC-1078 test, which is not tagged TC-688. The reader half of FR-337-AC-24 and of FR-326-AC-6 lives in the contract-model crate and is out of this PR (IR lane).

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-326-AC-6 | spec/functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md | examined |
| FR-337-AC-24 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md | examined |
| FR-233-AC-2 | spec/functional/FR-233-bind-a-model-claim-to-model-time-or-model-steps.md | examined |
| TC-836 | spec/test-cases/TC-836-s3-admits-temporal-operators-by-profile.md | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-337-AC-24 (TC-524): no test carries the tag. Its emit half (`eventually[0,5]` emits `temporal_interval {lower: "0", upper: "5"}`) and the no-node-written half are not asserted at emit level; the unit test of `temporal_interval` checks only the returned fault string. | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:541; qsl-semantics/src/check/lowering/temporal.rs:839 |
| FND-002 | low | FR-233-AC-2 second sentence (`eventually[0 ms, 3 ms]` under `model-steps` refuses `ill_typed`/`type-mismatch`) is exercised by the TC-1078 test, which is tagged TC-1078 only; no test carries TC-688 or FR-233-AC-2. Add the tag. | spec/functional/FR-233-bind-a-model-claim-to-model-time-or-model-steps.md:94; qsl-semantics/src/check/temporal_clause/tests.rs:911 |
| FND-003 | low | The timed-interval refusal in hyper clauses and probabilistic claims (same shared arm) has no test of its own. | qsl-semantics/src/check/hyper_clause.rs:1357; qsl-semantics/src/check/probabilistic/claim.rs:1212 |

## Verdict

FR-326-AC-6 is complete: tick `[0,5]` checks with `Steps(0,5)` and horizon 5; the five timed forms (`[0 ms, 5 ms]`, `[0 ms, 5 ms)`, `[0.5 ms, 5 ms]`, `(1 ms, 5 ms]`, `[3 ms, *]`) refuse `ill_typed`/`type-mismatch` at the interval span; `[3,*]` refuses `unsupported_construct`/`expression-form`; the aggregate window `count over past[0 ms, 300 ms] min_count 1 >= 1` checks. No ticketed or specified behavior is narrowed (FR-195/196/201/204 digital-clock claims and the FR-393/FR-447 dense Timed profile still route through `IntervalReading::Timed`).

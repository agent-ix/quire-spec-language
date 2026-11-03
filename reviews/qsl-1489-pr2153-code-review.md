---
id: SR-2026110216384
title: 'PR #2153 code-review: AC sweep FR-146 total pure functions AC-10..14 (QSL-1489)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@peter/qsl-1489-ac-sweep; git diff origin/main...pr2153 (PR #2153): tests and trace tags only; tickets QSL-1489'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1489. PR: quire-spec-language#2153 (head peter/qsl-1489-ac-sweep). Method: code-review over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| QSpec-FR-146-AC-10 (QSL-1489) | spec/functional/expressions/FR-146-check-total-pure-functions.md; tests: qsl-semantics/src/check/sum_case/tests.rs each_exhaustiveness_obligation_refuses_with_its_union_arm_set_and_locus; a_case_reports_only_its_first_refusal_in_the_fixed_order | examined |
| QSpec-FR-146-AC-11 (QSL-1489) | spec/functional/expressions/FR-146-check-total-pure-functions.md; tests: qsl-semantics sum_case/tests.rs; qsl-eval union_case.rs a_case_over_a_union_checks_with_positional_binders_per_arm; a_binder_is_visible_only_in_its_own_arm_and_a_case_nests; case_selects_the_active_members_arm_and_binds_its_payload; a_case_in_an_arm_body_evaluates_its_inner_arm | examined |
| QSpec-FR-146-AC-12 (QSL-1489) | spec/functional/expressions/FR-146-check-total-pure-functions.md; tests: qsl-semantics/src/check/sum_case/tests.rs without_an_expectation_the_arm_bodies_decide_the_result_type; a_case_result_type_follows_the_expectation_then_the_arm_bodies | examined |
| QSpec-FR-146-AC-13 (QSL-1489) | spec/functional/expressions/FR-146-check-total-pure-functions.md; tests: qsl-eval/src/value/expression/evaluate/union_case.rs a_stopped_scrutinee_or_argument_is_the_outcome_with_nothing_after_it; construction_and_case_charge_exactly_their_accounting_points | examined |
| QSpec-FR-146-AC-14 (QSL-1489) | spec/functional/expressions/FR-146-check-total-pure-functions.md; tests: qsl-semantics/src/check/sum_case/tests.rs a_case_reports_only_its_first_refusal_in_the_fixed_order | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | In a_case_result_type_follows_the_expectation_then_the_arm_bodies the Int-ranges arm group is `let c = case ... in c` inside a function declared `: Integer`, and the test only calls typed.expect("checks"). The inferred type of the case is never read, so a regression that infers the hull Int[0,99] (which also satisfies a declared Integer) passes. Only the first group (root type == Int[0,100]) asserts a type. Assert the let-bound case node's value_type == Integer, as the Text group does for its type. | qsl-semantics/src/check/sum_case/tests.rs:985-995 |

## Verdict

1 finding(s). Every added or changed test was read against the QSpec acceptance-criterion text. Gates on the trial merge: cargo test --all-features -p qsl-semantics -p qsl-eval all pass; gates rc=0. Ticket closability per ticket:

- QSL-1489: NO: AC-9, 15, 16 remain untagged (not implemented; AC-16 IR-blocked). AC-10..14 are tagged and pass.

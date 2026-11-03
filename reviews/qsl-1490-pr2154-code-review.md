---
id: SR-2026110216386
title: 'PR #2154 code-review: AC sweep QSpec FR-041 ordered queries (QSL-1490)'
type: SpecReview
analysis: code-review
scope: 'agent-ix/quire-spec-language@peter/qsl-1490-ac-sweep; git diff origin/main...pr2154 (PR #2154): tests and trace tags only; tickets QSL-1490'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1490. PR: quire-spec-language#2154 (head peter/qsl-1490-ac-sweep). Method: code-review over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| QSpec-FR-041-AC-1 (QSL-1490) | spec/functional/foundation/FR-041-evaluate-ordered-queries.md; tests: qsl-eval/tests/it/collection_queries.rs fr041_the_amounts_example_keeps_duplicate_occurrences_and_its_exact_aggregates | examined |
| QSpec-FR-041-AC-2 (QSL-1490) | spec/functional/foundation/FR-041-evaluate-ordered-queries.md; tests: qsl-eval/tests/it/collection_queries.rs fr041_an_empty_domain_yields_the_declared_query_identities | examined |
| QSpec-FR-041-AC-3 (QSL-1490) | spec/functional/foundation/FR-041-evaluate-ordered-queries.md; tests: qsl-eval/tests/it/collection_queries.rs fr041_a_wrong_domain_refuses_and_six_items_are_not_admitted_to_the_total | examined |
| QSpec-FR-041-AC-4 (QSL-1490) | spec/functional/foundation/FR-041-evaluate-ordered-queries.md; tests: qsl-eval/tests/it/collection_queries.rs fr041_decisive_forms_stop_and_the_rest_reach_every_occurrence | examined |
| QSpec-FR-041-AC-5 (QSL-1490) | spec/functional/foundation/FR-041-evaluate-ordered-queries.md; tests: qsl-eval/tests/it/collection_queries.rs q08, q14, q15, q16 | examined |
| QSpec-FR-041-AC-6 (QSL-1490) | spec/functional/foundation/FR-041-evaluate-ordered-queries.md; tests: qsl-eval/tests/it/collection_queries.rs q13_sum_proves_every_prefix_inside_its_domain_for_every_order; tc_500_a_running_total_outside_the_domain_is_undefined_at_the_sum_node | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | fr041_a_wrong_domain_refuses_and_six_items_are_not_admitted_to_the_total covers the N=6 total (Unproved Range), a Boolean total (type-mismatch) and six items refusing CardinalityOutOfBound. No unit mismatch case exists, so the "unit ... refuses" clause is untested. | qsl-eval/tests/it/collection_queries.rs:1595-1632 |
| FND-002 | medium | The four tagged tests (q08, q14, q15, q16) cover the resource-exhaustion clause (Incomplete, no completed value). None feeds an incomplete domain or an unknown required membership to a query and asserts it is not read as empty or successful, which is the first clause. Judgement call: those semantics may live in the FR-084 population tests. | qsl-eval/tests/it/collection_queries.rs:774, 1099, 1130, 1153 |
| FND-003 | low | fr041_decisive_forms_stop_and_the_rest_reach_every_occurrence compares work totals (equal when later occurrences are skipped, strictly larger otherwise). That checks stopping and reaching but not that each reached occurrence and its source binding are retained. | qsl-eval/tests/it/collection_queries.rs:1640-1672 |
| FND-004 | low | The new tests compare `format!("{:?}", completed(..))` strings against `format!("{:?}", int(..))` rather than the Values. They are value-asserting (a wrong sum fails), but a Debug-format change would break them and equal Debug text hides type differences. Compare Values. | qsl-eval/tests/it/collection_queries.rs:1530-1590 |

## Verdict

4 finding(s). Every added or changed test was read against the QSpec acceptance-criterion text. Gates on the trial merge: cargo test --all-features -p qsl-eval all pass; gates rc=0. Ticket closability per ticket:

- QSL-1490: YES against its exit criterion (all six ACs tagged and passing on the trial merge, QSpec FR-041 has no other AC); AC-3/4/5 each have an unverified sub-clause (findings), which a strict reading would fix first.

---
id: SR-2026110216387
title: 'PR #2154 gap-analysis: AC sweep QSpec FR-041 ordered queries (QSL-1490)'
type: SpecReview
analysis: gap-analysis
scope: 'agent-ix/quire-spec-language@peter/qsl-1490-ac-sweep; git diff origin/main...pr2154 (PR #2154): tests and trace tags only; tickets QSL-1490'
review_set: subset
date: 2026-11-03
---

## Summary

Tickets: QSL-1490. PR: quire-spec-language#2154 (head peter/qsl-1490-ac-sweep). Method: gap-analysis over the trial merge of the PR onto origin/main (fd5bde253). Mergeability: clean (git merge-tree rc=0, trial merge onto fd5bde253 clean). Plan completion: not assessed.

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
| FND-001 | low | No findings (placeholder) | - |

## Verdict

Coverage of the QSpec criteria of the PR's tickets, computed by tag extraction over the trial tree (`QSpec-FR-n-AC-k` against the live ACs of the QSpec spec at /Users/peter/dev/quire-specification). 0 criterion(s) remain untagged. Ticket closability per ticket:

- QSL-1490: YES against its exit criterion (all six ACs tagged and passing on the trial merge, QSpec FR-041 has no other AC); AC-3/4/5 each have an unverified sub-clause (findings), which a strict reading would fix first.

---
id: SR-12039
title: "QSL-813 PR #919 gap analysis: FR-365 multi-valued reaches destination"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@607a2285856cf5330fee3a2b5c91df459d39f9b3; FR-365-AC-1, FR-365-AC-3, FR-365-AC-5; qsl-semantics/src/check/state_model.rs, qsl-semantics/tests/it/relationship_reachability.rs, qsl-package/src/emit/tests/reachability.rs"
review_set: subset
---

## Summary

Ticket: QSL-813. PR quire-spec-language#919. Planless, diff-scoped gap analysis. Plan completion: not assessed. The changed criteria FR-365-AC-1, FR-365-AC-3 and FR-365-AC-5 were traced to tagged tests by reading; no matrix was run (read-only brief).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No findings (placeholder) | - |

## Verdict

PASS. FR-365-AC-1 multi-valued admission is backed by `c17_a_finite_multi_valued_destination_checks_boolean` (Set and Bag shaped, invariant and precondition). FR-365-AC-3 foreign multi-valued refusal is backed by `c10_a_foreign_destination_is_ineligible` (`manyOwners`), and operand-before-multiplicity by `c18_the_operand_check_is_reported_before_direction_and_multiplicity`. FR-365-AC-5 for a multi-valued end is backed by `a_multi_valued_destination_lowers_to_the_same_node_shape`, which also asserts the IR v2 reader admits the package. The evaluator in qsl-eval already collects every neighbour from `targets`/`sources`, so no runtime code path depends on a single-valued destination. No stubs or tautological assertions in the added tests.

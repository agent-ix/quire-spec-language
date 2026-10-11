---
id: SR-4963
title: "Manual acceptance-to-test mapping for QSL PR #721"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@288503ee7cbc7c6a1bc98ffe5dff94bdbfd3848e; base f6adf1fbccca4b2a8db00ba142300537228bebd7; changed files qsl-eval/tests/it/model_reference_queries.rs, qsl-semantics/src/model/population.rs, qsl-semantics/tests/it/model_population.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-089
    type: reviews
---
# Manual acceptance-to-test mapping for QSL PR #721

## Summary

Ticket: QSL-674. Exact reviewed head `288503ee7cbc7c6a1bc98ffe5dff94bdbfd3848e`; base `f6adf1fbccca4b2a8db00ba142300537228bebd7`. `quoin matrix --repo /tmp/qsl721-review --json` and the static fallback `quire matrix --scope /tmp/qsl721-review --format json` both timed out without matrix output. Accordingly, this is the required manual acceptance-to-test mapping for the increment, not a repository-wide matrix, reverse-gap, or stub audit.

## Manual mapping

- FR-089-AC-7 member change and order invariance: `population_identity_binds_maximum_and_canonical_members` compares a production identity against an independently constructed expected identity and asserts changed-member and permutation behavior.
- FR-089-AC-7 maximum distinction: the same semantics test checks 3 versus 7; `population_bindings_with_different_maxima_have_distinct_ids` records and resolves both identities in `ObjectEnvironment`.
- FR-089-AC-7 Post/Pre distinction: `population_post_identity_binds_its_pre_population_id` holds post members equal, changes the pre binding, and checks the results against the independent helper.
- FR-089-AC-7 fixed preimage: `population_identity_golden_vector_contains_all_seven_fields` hashes literal seven-field text independently of the producer helper.
- FR-084-AC-1: `invalid_population_member_is_refused_before_identity_is_minted` asserts the concrete ForeignType refusal.
- FR-084-AC-4: the new invalid-member test's AC-4 tag is inaccurate; however, the existing `l05_conflicting_identity_refuses_after_fourth_member_charge` and `l05_duplicate_collapses_and_recovers_l01` tests in the reviewed test file carry AC-4 and cover its two stated behaviors.
- NFR-012: the new producer's member-byte serialization is input-proportional and is not charged by the admission meter; this code finding is detailed in SR-4962 FND-002.

## Verdict

CONDITIONAL — no uncovered test behavior was identified in the manually traced acceptance increment. The computed full-repository matrix and reverse-gap/stub audit are unavailable due to both matrix commands timing out. This does not establish a repository-wide PASS.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | No uncovered acceptance behavior found in the manually mapped increment (placeholder) | - |

## Coverage

- Reconciliation: manual acceptance-to-test mapping only; Quoin and Quire matrix commands each timed out after 25 seconds with no matrix output.
- Plan completion: not assessed.
- Reverse-gap and stub inventory: not assessed beyond the three changed code files.
- Oracle review: included in companion code/Rust review SR-4962; tests were not run because queued gates are active.
- No builds or tests were run by this reviewer.

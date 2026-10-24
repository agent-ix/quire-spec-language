---
id: SR-4961
title: "QSL-669 gap analysis and manual acceptance-to-test mapping for PR #720"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@4e16d437ec13a70de86aee6e8f5ea21076a7e3a4; base bdf259f008c6577f4235b06ab1d87c32db83099a; changed model identity wire shapes, normalization provenance and charge-before-work behavior; QSpec FR-150-AC-1, FR-150-AC-4, FR-150-AC-6, FR-150-AC-7, FR-150-AC-8; NFR-012"
review_set: subset
---
## Summary

Ticket: QSL-669. PR #720 at `4e16d437ec13a70de86aee6e8f5ea21076a7e3a4`. This is gap-analysis disposition round 2, against base `bdf259f008c6577f4235b06ab1d87c32db83099a`.

The prior FND-001 remains fixed by `8eb14aec90fabb189ccfdf485d87099db326e68a`: `effective_declaration_identities_match_their_golden_vectors` retains `QSpec-TC-195` and now includes `QSpec-FR-150-AC-6`. This round rechecked that binding and the subsequent change to charge inherited facts before work. No new scoped trace gap, unowned behavior, stub, coverage inflation, or test-oracle defect was identified.

| Acceptance unit | Relevant test | Round-2 mapping |
| --- | --- | --- |
| QSpec FR-150-AC-1: inherited/redefined member identity links to every contributing original declaration and rule | `n01_normalizes_f1_to_stable_deterministic_identities`; `n02_normalizes_f2_diamond_inheritance_to_stable_deterministic_identities` | Assertions retain qualification followed by each inheritance-path fact. |
| QSpec FR-150-AC-4: ordered derivation replay reproduces effective-view identity independent of IR node order | `n07_record_order_does_not_affect_identity_or_view`; `effective_view_identity_matches_its_golden_vector` | Record-order behavior and exact effective-view vector remain asserted. |
| QSpec FR-150-AC-6: diamond paths collapse to one member retaining every path and each effective identity equals its published vector | `n02_normalizes_f2_diamond_inheritance_to_stable_deterministic_identities`; `effective_declaration_identities_match_their_golden_vectors`; `object_universe_identity_matches_its_golden_vector` | The test-to-AC trace added in 8eb remains present; prior FND-001 remains fixed. |
| QSpec FR-150-AC-7: equal nodes across package versions retain declaration identities while effective views differ | `n01v2_a_version_only_change_reuses_declarations_but_changes_view_and_universe` | Equal declaration and distinct view/universe identities remain asserted. |
| QSpec FR-150-AC-8 and NFR-012: exact bounds complete; denied charges stop before priced work | `n01_exact_limits_complete_and_the_charge_totals_match_ground_truth`; `n01_one_less_work_unit_is_incomplete_at_the_view_hash`; `a_refused_normalization_does_work_bounded_by_the_limit_not_the_package` | Exact charge totals and the narrow/wide denied-fact regression check remain explicit; the new charge ordering places `work_step()` after successful fact charges. |

The whole-repository matrix and reverse-gap audit remain unavailable. `quoin matrix --repo . --json` was attempted, but did not produce output during this review and was interrupted; no counts or repository-wide PASS are inferred. This round records the scoped manual mapping only. Optional intent↔test↔code semantic review was not requested and was skipped. Plan completion: not assessed.

## Coverage

- Reconciliation: quoin matrix (quoin 0.28.3, quire 0.36.2) attempted but incomplete; matrix counts unavailable.
- Plan completion: not assessed
- Criteria: tagged / untagged / ignored / method-without-symbol counts unavailable because matrix computation did not complete.
- Untraced behaviors / stubs: whole-repository counts unavailable; none identified in the scoped changed behavior.
- Semantic review: skipped

## Verdict

CONDITIONAL for the scoped gap-analysis method because the whole-repository matrix/reverse-gap audit could not complete. The prior FND-001 remains fixed; no new scoped gap was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The changed effective-declaration golden-vector test asserts the exact published preimages and digests, but `effective_declaration_identities_match_their_golden_vectors` is tagged only `QSpec-TC-195` at `identity_golden_vectors.rs:149`. The only inspected test tagged to QSpec FR-150-AC-6 that covers the diamond (`n02_normalizes_f2_diamond_to_stable_deterministic_identities`) checks SHA-256 shape, uniqueness, path count and rerun stability, not equality to the published effective-declaration vectors. Thus the PR changes the two effective-declaration vectors without a test-to-AC binding that carries the AC's vector-equality obligation. Add the FR-150-AC-6 trace to the effective-declaration vector test (and retain the TC trace), or add an equivalent correctly traced vector assertion. | qsl-semantics/tests/it/identity_golden_vectors.rs:149-169; qsl-semantics/tests/it/model_normalization.rs:742-797; QSpec FR-150-AC-6 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8eb14aec90fabb189ccfdf485d87099db326e68a |

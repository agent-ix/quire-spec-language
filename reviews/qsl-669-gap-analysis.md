---
id: SR-4961
title: "QSL-669 gap analysis and manual acceptance-to-test mapping for PR #720"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@8eb14aec90fabb189ccfdf485d87099db326e68a; changed model identity wire shapes and derivation provenance; QSpec FR-150-AC-1, FR-150-AC-4, FR-150-AC-6, FR-150-AC-7, FR-150-AC-8; QSpec model normalization derivation and published vectors"
review_set: subset
---
## Summary

Ticket: QSL-669. PR: quire-spec-language#720 at `8eb14aec90fabb189ccfdf485d87099db326e68a`.

This is disposition round 1 for SR-4961. The prior FND-001 was a medium test-to-criterion trace gap: the changed effective-declaration golden-vector test lacked the FR-150-AC-6 trace. The current test at `qsl-semantics/tests/it/identity_golden_vectors.rs:149-150` now has `#[trace("QSpec-TC-195", "QSpec-FR-150-AC-6")]`, retaining the test-case trace and adding the missing acceptance-criterion trace. The finding is fixed by `8eb14aec90fabb189ccfdf485d87099db326e68a`.

Manual acceptance-to-test mapping for changed behavior:

| Acceptance unit | Relevant test | Mapping |
| --- | --- | --- |
| QSpec FR-150-AC-1: inherited/redefined member identity links to every contributing original declaration and rule | `n01_normalizes_f1_to_stable_deterministic_identities`, `n02_normalizes_f2_diamond_inheritance_to_stable_deterministic_identities`, `effective_declaration_identities_match_their_golden_vectors` | Tests assert qualification plus inherited derivation facts and preserve each diamond path; exact effective-declaration vectors are now also tagged to AC-6. |
| QSpec FR-150-AC-4: ordered derivation replay reproduces effective-view identity independent of IR node order | `n07_record_order_does_not_affect_identity_or_view`; `effective_view_identity_matches_its_golden_vector` | Record-order identity and exact effective-view vector are asserted. |
| QSpec FR-150-AC-6: diamond paths collapse to one member retaining every path and each effective identity equals its published vector | `n02_normalizes_f2_diamond_inheritance_to_stable_deterministic_identities`; `effective_declaration_identities_match_their_golden_vectors`; `object_universe_identity_matches_its_golden_vector` | The diamond test asserts one effective member and all derivation paths; the effective-declaration vector test now has both TC-195 and FR-150-AC-6 tags and checks exact published bytes/digests. FND-001 is fixed. |
| QSpec FR-150-AC-7: equal nodes across package versions retain declaration identities while effective views differ | `n01v2_a_version_only_change_reuses_declarations_but_changes_view_and_universe` | The test asserts equal declaration identities and distinct view/universe identities across package versions. |
| QSpec FR-150-AC-8: exact normalization limits complete and one-less is incomplete at the named charge | `n01_exact_limits_complete_and_the_charge_totals_match_ground_truth`; `n01_one_less_work_unit_is_incomplete_at_the_view_hash`; `n01_charges_the_exact_ground_truth_sequence_in_order` | Exact counts/charge sequence and one-less refusal assertions were adjusted for the added qualification fact. |

The code now emits the qualification fact before inherited facts, with ordinals following vector order. The direct-member and inherited-member accounting tests were updated to include that fact. No new unowned behavior, stub, or trace gap was identified in this scoped manual review.

The required `quire matrix --scope . --format json` invocation emitted module warnings but did not complete or produce matrix output during this review; it was stopped. The repository-wide computed matrix and reverse-gap audit are unavailable. This artifact records only the scoped manual mapping above, not a repository-wide PASS. Optional intent↔test↔code semantic review was not requested and was skipped. Plan completion: not assessed.

## Verdict

CONDITIONAL. The prior FND-001 trace gap is fixed. Repository-wide matrix and reverse-gap results remain unavailable. No additional scoped gap was identified.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The changed effective-declaration golden-vector test asserts the exact published preimages and digests, but `effective_declaration_identities_match_their_golden_vectors` is tagged only `QSpec-TC-195` at `identity_golden_vectors.rs:149`. The only inspected test tagged to QSpec FR-150-AC-6 that covers the diamond (`n02_normalizes_f2_diamond_to_stable_deterministic_identities`) checks SHA-256 shape, uniqueness, path count and rerun stability, not equality to the published effective-declaration vectors. Thus the PR changes the two effective-declaration vectors without a test-to-AC binding that carries the AC's vector-equality obligation. Add the FR-150-AC-6 trace to the effective-declaration vector test (and retain the TC trace), or add an equivalent correctly traced vector assertion. | qsl-semantics/tests/it/identity_golden_vectors.rs:149-169; qsl-semantics/tests/it/model_normalization.rs:742-797; QSpec FR-150-AC-6 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 8eb14aec90fabb189ccfdf485d87099db326e68a |

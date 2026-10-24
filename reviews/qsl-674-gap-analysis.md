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

## Disposition pass 3 — exact-head manual mapping

This pass reviewed PR #721 head `331d2ccab5e90586112c5af09ed3b5380536c186` against fresh `origin/main` base `bdf259f008c6577f4235b06ab1d87c32db83099a`. The changed source/test scope remains the three G3 files listed above. `quoin matrix --repo /tmp/qsl721-review --json` and `quire matrix --scope /tmp/qsl721-review --format json` both timed out at 30 seconds without matrix output; the latter emitted duplicate-archetype/inverse-edge and inline-schema warnings before timing out. The manual mapping below is therefore subset-only, not a computed matrix or repository-wide reverse-gap/stub audit.

- FR-089-AC-7 member and maximum distinctions remain behaviorally checked by `population_identity_binds_maximum_and_canonical_members` and `population_bindings_with_different_maxima_have_distinct_ids`.
- FR-089-AC-7 Post/Pre identity remains checked by `population_post_identity_binds_its_pre_population_id`; the literal seven-field vector is checked by `population_identity_golden_vector_contains_all_seven_fields`.
- `population_identity_encoding_is_charged_before_hashing` checks an NFR-012 work-limit outcome, not FR-089-AC-7 identity behavior. Its trace binding is recorded as code-review FND-005 in SR-4962. This does not leave FR-089-AC-7 uncovered because the identity tests above remain tagged.
- FR-084-AC-1 and FR-084-AC-4 coverage is unchanged; the old false AC-4 tag in FND-003 remains corrected.
- The prior SR-4963 finding row is only the required clean placeholder, not a substantive finding. No acceptance-coverage finding is added in this pass. Oracle review of the current member/max/pre assertions and literal vector is recorded in SR-4962; tests were not run by the reviewer.

The current source review identifies an NFR-012 accounting defect in worst-case member sorting (SR-4962 FND-004); this is a code/resource-bound finding rather than an uncovered acceptance-test behavior. The computed matrix and broad reverse-gap/stub inventory remain unavailable.


## Disposition pass 4 — exact-head manual mapping

Reviewed PR #721 head `8e7a6de3dd57812da067beab9e5f020a45ffe726` against fresh `origin/main` base `bdf259f008c6577f4235b06ab1d87c32db83099a`. `quoin matrix --repo /tmp/qsl721-review --json` again timed out after 30 seconds with no matrix output; therefore this remains a manual subset mapping, not a repository-wide matrix or reverse-gap/stub audit. The historical round-3 observations above are retained as history: on this head, identity work is charged before encoding, sorting work is priced, and the charge-bound test is now tagged `NFR-012` rather than `FR-089-AC-7`.

- FR-089-AC-7 member/max distinctions remain checked by `population_identity_binds_maximum_and_canonical_members` and `population_bindings_with_different_maxima_have_distinct_ids`.
- FR-089-AC-7 Post/Pre identity distinction remains checked by `population_post_identity_binds_its_pre_population_id`; `population_identity_golden_vector_contains_all_seven_fields` retains the literal independent vector.
- NFR-012 has new trace-bound tests `population_identity_sort_work_bound_covers_repeated_prefixes` and `population_identity_encoding_is_charged_before_hashing`; source charges the derived work estimate before identity encoding/sorting. The second test does not detect a regression that moves the encoder before the denied charge (SR-4962 FND-007), so test-oracle strength remains an open code-review finding even though the current production ordering is correct.
- The shared admission tests now expect four admitted charges (three member charges plus identity-work charge) and totals 16,318 and 5,963. These assertions address the stale oracle from the prior head; they were not run by this reviewer.
- FR-084-AC-1/AC-4 mappings and coverage are unchanged; the historical FND-003 false tag remains fixed.

No new uncovered acceptance behavior was identified in this manual subset mapping. The complete computed matrix and repository-wide reverse-gap/stub inventory remain unavailable due to the matrix timeout. SR-4963 still has only its clean placeholder; no gap-analysis disposition comment is needed.


## Disposition pass 8 — FND-008 fix delta

Reviewed exact PR #721 head `a7619ac671e3bc2e54054a50b7e9d39124304642` against `origin/main` `dd64d017bfc7635a67493736755a65c77488b4dd`, limited to the delta from `0fe2da0c1c7bb045b9264befc5fdb49f90092b0f`. The code review's FND-008 concerned preserving canonical encoder refusal details; no uncovered acceptance behavior was found in this delta.

- `population_identity_digest_limit_preserves_encoder_limit_details` is trace-tagged `TC-434` / `NFR-012`; it forces a real canonical digest limit and checks `ResourceExhausted`, `CanonicalBytes`, bound zero, and a positive required-byte count.
- `population_identity_allocation_refusal_preserves_requested_size` is trace-tagged `TC-434` / `NFR-012`; it verifies the allocation cause retains 4096 bytes.
- `population_identity_unexpected_encoder_refusal_is_internal_failure` is trace-tagged `TC-434` / `NFR-012`; it exercises the non-exhaustive fallback with a serializer refusal and checks the internal-failure cause and resource-exhausted code.
- These tests assert observed refusal fields, so removing or misrouting the corresponding mapping causes an assertion failure. No tests were executed by this reviewer.

Quoin matrix was invoked on the repository root and interrupted after more than 30 seconds without output. This is a manual mapping of the requested fix delta only, not a repository-wide matrix, reverse-gap, or stub audit. Plan completion: not assessed.


Oracle qualification: the fallback test does fail if the internal-failure cause is not produced, but its assertion that this cause reports `ResourceExhausted` is questioned by code-review FND-009 because encoder invariant failures are internal faults, not resource refusals.


## Disposition pass 9 — FND-009 fix delta

Reviewed exact pushed PR #721 head `0509b6e98aab492d34a2530166b0a50631f365e7` against `origin/main` `dd64d017bfc7635a67493736755a65c77488b4dd`, scoped to the delta from round-8 head `a7619ac671e3bc2e54054a50b7e9d39124304642). The prior round-8 finding/comment `4fcdc11d-37b4-4944-9d6f-fd79402b0962` is dispositioned fixed.

- The non-exhaustive `quire_canonical::Error` match retains a wildcard. Its unexpected-error branch produces `PopulationIdentityFailure::InternalFault`, then `AdmissionOutcome::InternalFault`; it is no longer represented as a refusal or `ResourceExhausted`.
- `population_identity_unexpected_encoder_refusal_is_internal_failure` remains trace-tagged `TC-434` / `NFR-012`. Its oracle checks typed outcome, runtime-invariant code/cause, internal-failure category, non-resource-exhausted classification, and original upstream diagnostic. The limit and allocation tests continue to assert their separate refusal outcomes/details.
- `admit_invocation` propagates either pre/post non-admitted outcome unchanged. No new uncovered acceptance behavior or delta-specific test gap identified.
- No tests/builds were run. `quoin matrix --repo . --json` emitted no output and timed out at 25 seconds; this is a focused manual test-oracle mapping, not a repository-wide computed matrix or reverse-gap audit. Plan completion: not assessed.

Round-9 delta disposition: FND-009 fixed; no new findings.

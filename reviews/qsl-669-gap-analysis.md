---
id: SR-4961
title: "QSL-669 gap analysis and manual acceptance-to-test mapping for PR #720"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@cfe61582fd9a51f727be92b3659bc6dd744f9b51; base ab93113ae72c0750282c7db0daaf11eb4e70be7a; changed model identity wire shapes and golden vectors; QSpec FR-150-AC-4, FR-150-AC-6, FR-150-AC-7; `agent-ix/quire-specification/proposals/quire-v1/definitions/model-complete.md`; `agent-ix/quire-specification/proposals/checked-package-v2/model-effective-declaration.schema.json`"
review_set: subset
---
## Summary

Ticket: QSL-669. PR: quire-spec-language#720 at cfe61582fd9a51f727be92b3659bc6dd744f9b51.

Quoin's `matrix --repo /tmp/qsl720-review --json` and Quire's `matrix --scope /tmp/qsl720-review --format json` both remained silent and did not complete during this review; both invocations were stopped. The automated repository matrix and full reverse-gap audit are therefore unavailable. The following is the separately recorded manual acceptance-to-test mapping for the changed behavior, not a claim that the repository-wide matrix passed.

| Acceptance unit | Relevant test | Mapping |
| --- | --- | --- |
| QSpec FR-150-AC-4: replaying ordered derivation reproduces effective-view identity independent of IR node order | `n07_record_order_does_not_affect_identity_or_view` in `qsl-semantics/tests/it/model_normalization.rs`; `effective_view_identity_matches_its_golden_vector` in `identity_golden_vectors.rs` | Record-order behavior is asserted and the changed effective-view bytes are pinned. The golden-vector test has only the TC trace; its bytes are also among the cross-reference context for FND-001. |
| QSpec FR-150-AC-6: diamond derivations retain all paths and effective identities match published vectors | `n02_normalizes_f2_diamond_inheritance_to_stable_deterministic_identities` in `model_normalization.rs`; `effective_declaration_identities_match_their_golden_vectors` and `object_universe_identity_matches_its_golden_vector` in `identity_golden_vectors.rs` | The diamond test checks identity shape, uniqueness and retained paths, but does not compare identities with published vectors. The effective-declaration vector test compares exact bytes/digests but carries no FR-150-AC-6 trace. The object-universe vector test has that trace. This leaves the changed effective-declaration vectors without a correctly traced acceptance test (FND-001). |
| QSpec FR-150-AC-7: equal nodes across package versions preserve declaration identities while effective views differ | `n01v2_a_version_only_change_reuses_declarations_but_changes_view_and_universe` in `model_normalization.rs` | Equal declaration identities and distinct view/universe identities are asserted for the published version-1/version-2 fixtures. |

The changed typed code has no apparent reverse-spec gap: the field removals implement the closed shapes in the model definition and `model-effective-declaration.schema.json`. No source/test stub or coverage inflation was found in the changed production code.

Plan completion: not assessed

## Verdict

CONDITIONAL. Manual mapping found one medium traceability/evidence gap (FND-001). The automated matrix and repository-wide reverse-gap check could not be completed because both matrix commands hung without output.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The changed effective-declaration golden-vector test asserts the exact published preimages and digests, but `effective_declaration_identities_match_their_golden_vectors` is tagged only `QSpec-TC-195` at `identity_golden_vectors.rs:149`. The only inspected test tagged to QSpec FR-150-AC-6 that covers the diamond (`n02_normalizes_f2_diamond_to_stable_deterministic_identities`) checks SHA-256 shape, uniqueness, path count and rerun stability, not equality to the published effective-declaration vectors. Thus the PR changes the two effective-declaration vectors without a test-to-AC binding that carries the AC's vector-equality obligation. Add the FR-150-AC-6 trace to the effective-declaration vector test (and retain the TC trace), or add an equivalent correctly traced vector assertion. | qsl-semantics/tests/it/identity_golden_vectors.rs:44-50,149-169; qsl-semantics/tests/it/model_normalization.rs:714-759; agent-ix/quire-specification/spec/functional/type-model/FR-150-preserve-model-normalization-provenance.md:Acceptance Criteria |

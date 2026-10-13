---
id: SR-8721
title: "Gap analysis of quire-spec-language PR #808 (QSL-744) against its acceptance criteria"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@88cdadd279a4247945340f3bd8ae3b982c85e1cf; qsl-semantics/src/model/normalize.rs, qsl-semantics/src/model/admitted.rs, qsl-semantics/src/check/field_refinement.rs, qsl-semantics/tests/it/tc235_origin_retained.rs, qsl-semantics/tests/it/model_conformance.rs, spec/functional/FR-056-admit-domain-package-model-declarations.md"
review_set: subset
---
# Gap analysis of quire-spec-language PR #808

## Summary

Ticket: QSL-744. Method: quoin gap-analysis, run planless over the PR diff. Each ticket AC and FR-056-AC-33 was mapped to the tests that assert it. Plan completion: not assessed. Reviewer-only: no build or test run.

Examined:
- QSL-744 AC1: the remaining normalize.rs sites fill artifact and span from `DomainPackage::original()` (examined)
- QSL-744 AC2: tests first, one typed vector per site, asserting artifact and span as values (examined)
- QSL-744 AC3: FR-056 states the origin is retained, consistent with FR-154 (examined)
- QSL-744 AC4: TC-235 M10 run 10, a population member of the wrong meaning refuses `malformed-declaration` at `PopulationDefinition.members[i]`; a member naming nothing stays missing (examined)
- QSL-744 AC5: the node-less constructors are documented, and the scalar-domain refusal names the scalar type at `TypeDefinition.constraints` with its origin (examined)
- FR-056-AC-33 (examined)

AC to test map:
- AC1 / AC-33 record value identityFields: `a_record_value_type_with_identity_fields_keeps_its_origin`
- event identityFields: `an_event_type_with_identity_fields_keeps_its_origin`
- event occurrence: `an_event_occurrence_that_is_not_a_timestamp_keeps_its_origin`
- event operation: `an_operation_declared_by_an_event_type_keeps_its_own_origin`
- machine emits: `a_state_machine_transition_emitting_a_non_event_keeps_its_origin`
- machine required members: `a_state_machine_missing_required_members_keeps_its_origin`
- process identity and supertype: `a_process_without_identity_or_process_supertype_keeps_its_origin`
- Field.typeRef variant: `a_field_typed_by_a_variant_type_keeps_its_origin`
- relationship source: `a_relationship_source_of_another_meaning_keeps_its_origin`
- relationship target: `a_relationship_target_of_another_meaning_keeps_its_origin`
- generated origin: `a_generated_origin_leaves_artifact_and_span_none`
- AC4: `a_population_member_of_another_meaning_refuses_malformed_with_origin`
- AC5 scalar: `r08i_a_malformed_scalar_domain_refuses_rather_than_panicking`
- `admitted.rs::identity_refusal`: no test

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | AC5 asks for the scalar-domain refusal to carry "its origin". The only test builds its package without the reader and asserts `artifact: None, span: None`, so the retained-origin case for this refusal is never asserted. The node assertion (`model.Count`) pins the key passed to the shared `malformed_at`, so the risk is small. Fix: add a reader-built variant with a sourced scalar and assert the artifact and line. | qsl-semantics/tests/it/model_conformance.rs:1321-1335; qsl-semantics/src/check/field_refinement.rs:117-128 |
| FND-002 | low | `admitted.rs::identity_refusal` was moved onto `malformed_at`, but no vector asserts its origin. AC2 asks for one vector per site. With a reader-built package every key is under the selection's identity, so the first caller (admitted.rs:211) is only reachable from a hand-built package that has no origins. The second (neither identity form) may be reachable from the reader. Fix: add a vector for the reachable branch, or state in the test module why none can carry an origin. | qsl-semantics/src/model/admitted.rs:169-183, 211-220, 280-290 |
| FND-003 | low | The AC4 test is traced `QSpec-TC-235`, but it does not reproduce M10 run 10. The row (as amended by QSpec #245, merged) declares population `PA` with members `[OrderManagement]` and expects 22 charges at `PopulationDefinition.members[0]`. The test uses `PO` with `[Order, <wrong>]`, asserts `members[1]`, and does not assert the charge count. The cause and path rule are covered, but the row's literal outcome (index 0, 22 charges) is not. Fix: add the row's own run (`PA`, `[OrderManagement]`, 22 charges, `members[0]`), or drop the `QSpec-TC-235` trace from this vector. | qsl-semantics/tests/it/tc235_origin_retained.rs:366-404 |

## Verdict

AC1: met. Every normalize-stage site now uses `malformed_at`, and a grep of production code finds no `IntakeMalformedDeclaration` with a literal `None` origin.

AC2: met, apart from FND-002. The red log shows 10 of 13 functions failing before the production change. The three that passed were the generated-origin negative, a process vector since merged into a combined one whose sites had `None` before, and an occurrence vector whose current body can only be produced by normalize.rs:1224, which had `None` before. Assertions compare the code, cause, node, member path, artifact and line/column, never detail text. The event-supertype refusals (normalize.rs:1242-1251) share the event `malformed` closure that the identity and occurrence vectors cover.

AC3: met. The FR-056 paragraph and FR-056-AC-33 state the retention and cite FR-154.

AC4: met in behaviour (FND-003 is about the trace only). The missing-name case still asserts `MissingDeclaration`.

AC5: met for the documentation (the reasons at normalize.rs:1083-1086 and 1987-1989, plus the FR-056 sentence). The node is met. FND-001 covers the origin.

No stubs, no coverage inflation, and no code without an owning requirement.

**Mergeable. The three findings are low and can be fixed in the FND-001 code-review round.**

---
id: SR-20261105001152
title: "Gap analysis — QSL-1714/QSL-1715 acceptance slice"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; FR-507-AC-7, FR-507-AC-11; qsl-analyze/src/claim_family.rs, qsl-analyze/tests/it/claim_family.rs, qsl-analyze/tests/it/operation_contract.rs, qsl-package/src/emit/tests/one_occurrence.rs, qsl-package/src/emit/tests/step_relation.rs, qsl-replay/src/contract_check.rs, qsl-semantics/src/check/claims/map.rs, qsl-semantics/src/check/lowering.rs, qsl-semantics/src/check/mod.rs"
review_set: subset
---

## Summary

This manual acceptance-to-test check covers the changed QSL-1714/QSL-1715 behavior. The step-relation shared-occurrence case is exercised end to end; the Boolean clause behavior is asserted by a dedicated test, but the test is untagged and the tagged FR-507-AC-11 test delegates that case to it, leaving that subcase without a computed trace binding.

## Verdict

**CONDITIONAL** — FR-507-AC-11's Boolean-clause subcase has no trace-bound test; repository-wide matrix coverage is unavailable from this run.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | The Boolean-clause subcase of FR-507-AC-11 is asserted only by an untagged test; the tagged aggregate test explicitly delegates that subcase to it, so the acceptance evidence cannot bind this subcase through the declared trace model. | FR-507-AC-11, qsl-analyze/tests/it/claim_family.rs::a_boolean_value_clause_is_read_on_its_expression_node |

## Coverage

- Reconciliation: `quoin matrix --repo ... --json` remained CPU-bound for more than nine minutes without output; the `quire matrix --scope ... --format json` fallback remained CPU-bound for more than five minutes without output. Both were interrupted. The full repository matrix and evidence status are unavailable.
- Plan completion: not assessed.
- Criteria examined manually: FR-507-AC-7 and FR-507-AC-11. Repository-wide tagged / untagged / ignored-only counts are unavailable.
- FR-507-AC-7: pass. The claim-family integration test is trace-tagged and checks the shared relation occurrence, family reads, and independently settled items; the operation-contract integration test selects the shared occurrence as a step relation; the package integration test verifies emitted single claim/declaration roles and Contract IR read-back.
- FR-507-AC-11: partial trace binding. The tagged aggregate test covers the scalar application and other listed forms; its source comment delegates the Boolean clause to `a_boolean_value_clause_is_read_on_its_expression_node`, which asserts both claim/declaration roles, the Boolean type, family selection, engine execution, and wrong-kind refusal, but lacks `#[trace("FR-507-AC-11")]`.
- Reverse-gap inventory was limited to the changed behavior: four behavior areas examined, none found without an owning requirement; no source or test stubs found in the changed files. A repository-wide reverse-gap inventory was not completed.
- Semantic review: skipped; this optional analysis was not requested.
- The repository root `spec/spec.md` declares no `id`, so no `relationships` edge to a spec-root id was fabricated.

## Dispositions

| FND | outcome | reason |
| --- | --- | --- |
| FND-001 | fixed | The Boolean-clause test now carries the TC-1302 and FR-507-AC-11 trace tags. |

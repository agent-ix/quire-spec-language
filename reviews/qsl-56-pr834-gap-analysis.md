---
id: SR-9089
title: "Gap analysis of quire-spec-language PR #834: conformance stage inside check"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@bd0e216d9804ec6a8da75bd4f2bcd3458fc4027f; PR #834: FR-082-AC-4, AC-8, AC-16, AC-17 and QSpec TC-196 R02-R09, C01-C07, FR-110-AC-3 against qsl-semantics/tests/it/{model_conformance_boundary,model_conformance,type_environment_model}.rs, the deleted tests/it/field_presence.rs and xtask/src/definition_scan.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #834

## Summary

Ticket: QSL-56. Plan completion: not assessed. The oracle is the private QSpec rows TC-196, FR-150, FR-151 and FR-110, cited by id only.

Bindings checked:

- C05(a), C05(b) and C05(c), C04, C01 (including reversed node order), C02, C06 and C07, T8's totals 10/8/8/10 with the exact one-unit-short Incomplete, T9's R08 totals 3/12/4/3/12/12, the diamond lineage test (<=3 admits, <=6 gives one failure against B1/y, <=8 gives two), and FR-110-AC-3 through `staged_work_units()` against `types.spent()`. Each matches its QSpec row.
- FR-082-AC-16 is bound to `reference_covariance_uses_the_complete_model_without_a_writer_obligation`, which covers the subtype admit and the reverse and unrelated refusals. The no-proof-form clause is covered by `c03_r08c_object_typed_narrowing_reaches_no_proof_form`.
- The scan test drops `check_field_refinement_obligation`, which this PR deletes. That is correct, and no rule lost its scan.

## Verdict

FAIL. TC-196 R05's field rows are not tested, and testing them would fail (FND-001, see SR-9088 FND-001). FR-082-AC-17 has no bound test, and AC-8's no-obligation clause lost its only test.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | `c03_r05_multiplicity_narrowing_reaches_its_fr151_cause` replaces TC-196 R05, which is a field row, with a parameter row. Parameters are contravariant, so a narrower parameter is R03's refusal, not R05's. Nothing exercises R05's admitted field vectors through check. Those vectors are refused at `complete()` (SR-9088 FND-001), so the failing test shape is `check(&document(A/n:A {0,5}; B/n:A {1,3} redefines A/n), NOOP) == Ok(())` and the same for A/n {0,unbounded} with B/n {0,5}. The FCD validator accepts both, because it only checks that a redefinition's bounds lie within the target's. | qsl-semantics/tests/it/model_conformance_boundary.rs:1157 |
| FND-002 | medium | FR-082-AC-17 has no test that traces it. The tests that exercise it are C02, C06 and C07, T8 and the diamond test. They carry other tags, and C06 and C07 are tagged FR-082-AC-8 (field presence), which they do not exercise. | qsl-semantics/tests/it/model_conformance_boundary.rs:1308, qsl-semantics/tests/it/model_conformance_boundary.rs:1359, qsl-semantics/tests/it/model_conformance_boundary.rs:1636 |
| FND-003 | medium | AC-8's second sentence has no remaining test since `field_presence.rs` was deleted: both fields required [0,1], or both optional [1,1], written with no fact, admit and engage no obligation. The remaining AC-8 tags cover only the refuse and discharge half. | qsl-semantics/tests/it/field_presence.rs (deleted) |

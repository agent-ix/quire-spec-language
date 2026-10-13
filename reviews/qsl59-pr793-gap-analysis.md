---
id: SR-7862
title: "Gap analysis of quire-spec-language PR #793: FR-056-AC-5, FR-260-AC-3, FR-103-AC-3 and FR-103-AC-4 expectations"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@1e97537704f80527e67247e5f969d7e53fe90e01; PR #793: FR-260-AC-3 (TC-731), FR-056-AC-5 (TC-145 step 5), FR-056 native UUID rule (FR-056:213-222), FR-103-AC-3 and FR-103-AC-4 (TC-458 steps 3-4) against qsl-semantics/tests/it/model_intake.rs and qsl-semantics/tests/it/model_operations.rs"
review_set: subset
---
# Gap analysis of quire-spec-language PR #793

## Summary

Ticket: QSL-59. Plan completion: not assessed.

Checked each changed expectation against its cited spec:

- FR-260-AC-3 / TC-731: `a_composite_cycle_of_any_length_is_refused_by_the_readers_rule` names the closing relationship's identity as the node. Correct with the new arm.
- FR-056-AC-5 / TC-145 step 5: `missing_relationship_target_retains_its_referring_origin_and_pointer` asserts node `relationship/A-related-B`, the relationship's origin and the pointer. Correct under FR-154's relationship slot identity.
- FR-056:213-222 (native UUID retained): `reading_fcd_199s_golden_shape_reads_all_12_types` asserts `Pump/Sys/Tank.id` as `Native(Uuid)`. Correct.
- FR-103-AC-3 / TC-458 step 3: the test asserts the wrong locus. See SR-7861 FND-002.
- FR-103-AC-4 / TC-458 step 4: the test covers one clause of four (FND-001 below).

## Verdict

Changes requested: two medium findings and one low finding.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | `a_record_value_type_beside_an_admitted_operation_assembles` is the only test traced to FR-103-AC-4 / TC-458 step 4. It asserts only that a standalone `Note` record type assembles. It adds no `address: Address` field to `ConfigVersion` and no `modifies ConfigVersion/address` entry. It checks no `deref(a).address.street` read and no record_value_type emission, and does not assert that no `Reference<Address>` is emitted. The matrix therefore reports AC-4 covered while three of its four clauses are untested. Add the step 4 field, frame entry, checked read and emission assertions. | qsl-semantics/tests/it/model_operations.rs:2370-2399; spec/test-cases/TC-458-spine-admits-model-operations-and-frames.md:54-57 |
| FND-002 | medium | No test pins the locus of a reader diagnostic raised at a field, operation or operation-parameter pointer. Only relationship pointers (TC-145 step 5, TC-731) are covered, so the misattribution in SR-7861 FND-001 went undetected. Add tests asserting that a diagnostic at `/types/N/fields/M/...` and at `/types/N/operations/M/params/K` names that member's identity and origin. | qsl-semantics/src/model/intake.rs:1857-1922 |
| FND-003 | low | TC-731's test identifies the composite-cycle diagnostic by `refusal.detail.contains("COMPOSITE_CYCLE")`, matching message text. TC-145 says "Assertions compare typed refusal codes and loci, never message text". Match the typed diagnostic code if the refusal carries it, or record the reader code in the cause. | qsl-semantics/tests/it/model_intake.rs:1194-1203 |

---
id: SR-9031
title: "QSL-726 gap analysis of PR #816"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@3e004e2039af35d8fa340899aca3ed4852748fda; base impl/business-model-meanings (now includes #811, #813); QSL-726 AC-3 (QSL-INPUT-AC-3); FR-302-AC-1; FR-302-AC-5; FR-302-AC-8; TC-793; TC-795; TC-1011; qsl-replay/tests/dispatch_link_compile.rs; qsl-semantics/tests/it/dispatch_linking.rs; qsl-semantics/tests/it/operation_bodies.rs; spec/tests.md"
review_set: subset
---
## Summary

Ticket: QSL-726, AC-3. This is a planless gap analysis scoped to the PR diff. Each AC and TC was mapped to its tests, and test-oracle strength was judged by reading. Plan completion: not assessed.

- **QSL-INPUT-AC-3.** The production path calls `checked_dispatch_operation` once per family root, from `finish_unit`, using the retained bodies, the unit `pre` clauses and the admitted view. All seven replay tests reach it through `spine::check` or `compile_package`. Backed.
- **FR-302-AC-1 / TC-793.** `unit_compile_carries_one_linked_table_per_called_operation` asserts exactly two tables, compared as literals for `A` and `B`. It does not bind each call to the right table (FND-002).
- **FR-302-AC-5 / TC-795.** `unit_compile_refuses_a_family_past_the_callers_family_steps` covers both steps against the stale merge-base. It asserts the cause that #811 replaced (FND-001).
- **FR-302-AC-8 / TC-1011.** `unit_compile_checks_a_bodys_arity_and_type_against_its_operation` covers the `tooMany`, `wrongType` and `ok` steps. It asserts `ill_typed`/`type-mismatch`, a resolved region, the `Origin::Body` function, and no package. Backed.
- **Red evidence.** `qsl726e-red-replay.log` shows 3/3 failing with `MissingName`. `qsl726e-red-semantics.log` shows 2 of 13 failing. The green run is `qsl726e-green-replay6.log`, 7/7.

Whether each tests.md ✅ flip is earned:

- **TC-1011:** earned.
- **TC-793:** earned for steps 1 and 2. Expected result 3 ("both `size` calls name the `size` table") is only partly checked (FND-002).
- **TC-795:** earned only on the stale base. On the merged ref, the test asserts a cause that FR-302-AC-5 and TC-795 no longer state.

## Verdict

**FAIL.** Before the TC-795 flip can land, the test must assert `resource_exhausted`/`insufficient-next-charge` with the charge record. TC-795's interim paragraph must also go (SR-9032 FND-003). Coverage on the TC-793 and TC-1011 tags is inflated by tests that verify other behaviour.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | After the ref merges, the TC-795 test asserts a cause that the spec no longer states. FR-302-AC-5 and TC-795 now require `resource_exhausted`/`insufficient-next-charge`, naming the setting `model.family_steps` and the limit 2. The test asserts `catalog.cause() == "family-steps"` and `(bound, reached) == (2, 3)` through `DispatchLinkLimit`. The bridge's own unit test on the base already asserts the full record: setting `model.family_steps`, limit 2, actual 3, `limit_kind` `family_steps`, consumed 2, `next_charge` 1, `charge_point` `model.family-edge`, stage `model-normalization`, and `accounting_contract` `ModelNormalizationLimitsV1`. Rewrite the unit-level assertions to those values, read through the `CompileRefusal` fields that the refusal reports. Resolve the tests.md conflict on the TC-1011 row by keeping the base's FR-361 rows (TC-1012 to TC-1017). | qsl-replay/tests/dispatch_link_compile.rs:316-350; spec/tests.md:302 |
| FND-002 | medium | The TC-793 oracle for expected result 3 is partial. The test shows that the `size` calls share one table index, the `area` calls share another, and the two differ. It never checks that the `size` calls' index is the table whose entries are the `size` candidates. A table-index shift or swap in `merge_linked` would pass. Look up each call's table in `dispatch_tables()`, and compare its `(linked(a), linked(b))` pair with the literal expected for that member. | qsl-replay/tests/dispatch_link_compile.rs:257-280 |
| FND-003 | low | The test `a_candidates_precondition_is_the_units_pre_clauses_not_the_domain_requires` is tagged TC-793 / FR-302-AC-1. It verifies where a candidate's precondition comes from: FR-302's join rule and FR-103's rule that domain `requires` are not used. It does not verify one table per called operation. Retag it to FR-302-AC-7 (or FR-302's join rule). Its own doc comment already says FR-302-AC-7. | qsl-replay/tests/dispatch_link_compile.rs:410-417 |
| FND-004 | low | The test `a_unit_with_linked_dispatch_emits_a_package` is tagged TC-793 / FR-302-AC-1. It verifies that E4 emission is deterministic for a linked unit, not that the unit carries one table per called operation. Retag it to the FR-096/FR-060 placement criterion that the fix serves (see SR-9032 FND-002). | qsl-replay/tests/dispatch_link_compile.rs:540-547 |
| FND-005 | low | The test `a_body_reads_its_receiver_and_its_renamed_parameter` is tagged TC-1011 / FR-302-AC-8. It verifies that `self` and a renamed parameter bind inside a body, not that arity or type is checked. Retag it to FR-302's join rule, or leave it untagged. | qsl-replay/tests/dispatch_link_compile.rs:525-529 |
| FND-006 | low | The test `intake_reads_an_operations_redefines_property` is tagged TC-793 / FR-302-AC-1. It verifies that intake reads `redefines` (FR-103), not the linked tables. Retag it to an FR-103 criterion. | qsl-semantics/tests/it/dispatch_linking.rs:116-120 |
| FND-007 | medium | Nothing tests the new region resolution. `synthesized_spans` is read by all three resolvers and by the three `declaration_region` methods, and lowering adds a new `generated` occurrence on every clause function. The emission test asserts only that the bytes are non-empty and deterministic. No test checks that a refusal or occurrence in a synthesized precondition function resolves to its unit `pre` clause, or that a body function's occurrence resolves to its `operation-body`. Deleting the `synthesized` arm in `resolve` would leave every test green, as long as emission still placed some region. Add one region assertion for each kind. | qsl-semantics/src/check/region.rs:98-120; qsl-semantics/src/check/lowering.rs:2728-2747; qsl-replay/tests/dispatch_link_compile.rs:540-583 |

## Dispositions

Re-checked at `d0c5332fbbb9586ce0ba8f887d7b276ccd7e5e3f`. The tests.md flips were judged against each TC's steps:

- **TC-793:** steps 1-4 are covered, including the abstract `Mid` receiver.
- **TC-795:** the record is asserted through `CompileRefusal::fields()`.
- **TC-1011:** unchanged and earned.
- **TC-1018:** all four steps, with regions and the `generated` occurrence.
- **TC-1019:** all three steps.

All five ✅ flips are earned.

The coder ran no separate red runs for the new tests, so they were judged by mutation reasoning:

- Restoring name-only refusal fails the first compile of the unrelated-family test.
- Dropping the pending report turns the second compile into `MissingName`, and `refused_family` panics.
- Restoring owner-plus-entries exposure fails the abstract-`Mid` compile.
- Deleting the `synthesized` resolver arm, or the `with_spans` call, makes `region_text` panic.

The body-dispatch test documents a gate that already existed (TC-196 D07). Its claim was verified in the code: `dispatch_admitted` (typing.rs:1007) runs before the operation lookup and refuses every context except invariant, pre and post (state_model.rs:303-311).

The fix-round gate logs (`qsl726e-fixgate-*.log`) are green:

- clippy exit 0
- 11/11 `dispatch_link_compile`
- 0 FAILED in the test log
- root: 986 passed, 5 ignored
- xtask: 182 passed
- spec validation exit 0

No new findings.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d0c5332fb |
| FND-002 | fixed | d0c5332fb |
| FND-003 | fixed | d0c5332fb |
| FND-004 | fixed | d0c5332fb |
| FND-005 | fixed | d0c5332fb |
| FND-006 | fixed | d0c5332fb |
| FND-007 | fixed | d0c5332fb |

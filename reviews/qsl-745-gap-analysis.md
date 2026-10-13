---
id: SR-8731
title: "QSL-745 gap analysis of PR #811"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language@34a3c0a6e8ae0dbfd9cc8a68cd22c78c842a8df8; base impl/business-model-meanings; QSL-745 (a), (b); FR-277-AC-1; FR-255-AC-1; FR-260-AC-4; FR-100-AC-9; qsl-replay/src/spine/lifecycle/tests.rs; qsl-replay/src/spine/call/tests.rs; qsl-semantics/src/model/refusal.rs; qsl-package/src/emit/tests.rs; qsl-semantics/src/model/intake/unit.rs; qsl-replay/src/spine/native_result.rs"
review_set: subset
---
## Summary

Ticket: QSL-745, parts (a) and (b); (c) is deliberately out of this PR. This is a planless gap analysis scoped to the PR diff. Each ticket part and AC is mapped to its tests, and test-oracle strength was judged by reading. Plan completion: not assessed.

- (a) and FR-277-AC-1 (select): `select_names_the_model_limit_field_it_reached` drives six `model.*` fields to one below their counter. It asserts `resource_exhausted`, the FR-255 cause (`ancestor-steps` or `insufficient-next-charge`), `setting`, `bound` = counter - 1 and `actual` > bound, and it asserts success at the counter. Backed.
- (a) and FR-260-AC-4 (intake through select): `select_names_the_intake_limit_for_an_oversize_document` asserts `resource_exhausted`/`intake-limit-exceeded` with exact `setting`, `bound` (B) and `actual` (B + 1). Backed.
- Compile path: the emit test `normalization_refusals_and_limits_stop_intake_at_the_declaration` asserts the code and `insufficient-next-charge`. Backed for code and cause.
- (b): `a_reached_model_ceiling_carries_setting_bound_and_actual` checks exact maps for all three causes. `tc_452_step_4` checks the record shape and exact fields for `ancestor-steps` and `family-steps`. Backed.
- FR-255-AC-1 (caller-budget rows): carried by the tests above.

All changed expectations derive from FR-255 Behavior 1's table, FR-260-AC-4 and the amended FR-277-AC-1. None merely restates observed output. Each would fail on the old code (see SR-8730).

## Verdict

CONDITIONAL. Parts (a) and (b) are backed with strong oracles for code, cause, `setting`, `bound` and `actual`. The retained denied-charge fields and the native-run-result rendering of intake fields have no test. One select field gap is pre-existing.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | No test asserts the retained denied-charge fields (`stage`, `accounting_contract`, `consumed`, `next_charge`, `charge_point`) that `UnitIntakeCause::limit_fields` adds on the compile and select path. FR-255 Behavior 1 now says this path gives "the same ... fields" as observation admission. `assert_budget_field` reads only `setting`, `bound` and `actual`, and the emit test reads only code and cause. Deleting those five entries, or the new `record.fields = source.fields()` routing for Intake in native_result, leaves every test green. Add an exact-map assertion for one `insufficient-next-charge` select refusal and one native-run-result compile refusal. | qsl-semantics/src/model/intake/unit.rs:156-170; qsl-replay/src/spine/native_result.rs:1984; qsl-replay/src/spine/lifecycle/tests.rs:1144-1186 |
| FND-002 | low | Pre-existing gap: FR-277-AC-1 covers every field of `select`'s limits, but the select test drives only six of the eight `model.*` fields. `model.dispatch_candidates` and `model.family_steps` are never reached through `select`, although the test doc says "each field ... that a charge reaches". The `family-steps` cause is checked only through S6a conversion. | qsl-replay/src/spine/lifecycle/tests.rs:1252-1308 |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-003 | medium | The PR rewrites FR-302-AC-5 and TC-795 step 2 to say a compile past `family_steps` refuses at S3 with `resource_exhausted`/`insufficient-next-charge`, naming `model.family_steps` and the limit `n - 1`. No test is tagged TC-795 or FR-302-AC-5. The nearest test, `a_family_at_the_configured_bound_links_and_one_step_more_refuses`, runs `link_chain` directly. It asserts the native label `family-steps` and the setting through `limit_exceeded()`, never the catalog code, cause or charge record on the S3 compile path. The edited criterion is unverified. Add a compile-path test tagged `TC-795`/`FR-302-AC-5`. | spec/functional/FR-302-link-dispatch-during-unit-compile.md:114; spec/test-cases/TC-795-dispatch-family-walk-refuses-at-the-caller-limit.md:24-26; qsl-semantics/tests/it/model_dispatch.rs:798-860 |

## New findings (disposition pass 2)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | low | `ancestor_closure_admits_the_bound_and_refuses_one_past_it`, now tagged `TC-795`/`FR-302-AC-5`, checks the S3 dispatch bridge's `ancestor_closure` over a test-built chain. TC-795's procedure compiles a unit under `SpineLimits` and expects "no checked package". No unit compile reaches `checked_dispatch_operation` today: ADR-016 SC-3 says "once G-7 wires it". So the tag claims a procedure the test does not run, and FR-302-AC-5's compile-and-no-package half stays unverified. Fix: say in TC-795 that the bridge test backs the refusal record until G-7 wires dispatch into compile, or move the compile half to G-7's ticket. | spec/test-cases/TC-795-dispatch-family-walk-refuses-at-the-caller-limit.md:15-26; qsl-semantics/src/check/checked_dispatch.rs:1336-1361; spec/decisions/ADR-016-state-model-finite-execution-mapping.md:183 |

## Dispositions

Round 1, reviewed at a7be05cb81cb87eab1b83eee676595771d16b494.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | d060eb068: `select_retains_the_denied_charge_of_a_normalization_ceiling` asserts the exact key set and values of a select denied charge. `run_clause_writes_the_caller_budget_fields_of_a_compile_ceiling` asserts the native-run-result fields of a compile denied charge and of the intake size ceiling. Reverting `record.fields = source.fields()` to the Limit-only arm leaves Intake `fields` null, so `fields["setting"] == "model.work_units"` fails. Deleting the retained entries from `BudgetDenial::catalog_fields` fails the exact key assertion |
| FND-002 | rejected | `select` never charges `model.dispatch_candidates` or `model.family_steps`: dispatch families are linked at `check` (FR-302), so `select` reaches neither counter on any input, and FR-277-AC-1's "one below the counter the input reaches" is empty for them. The test now says so (lifecycle/tests.rs:1302-1305). The `family_steps` code path is tracked separately as new FND-003 |

Round 2, reviewed at bd42e758db3cd9f61c71f56bbcaff02d8551f8d2.

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-003 | fixed | bd42e758d: `assert_family_steps_refusal_with` asserts `resource_exhausted`/`insufficient-next-charge` and the full charge record (`setting` `model.family_steps`, `limit`, `actual` limit+1, `limit_kind` `family_steps`, `consumed` limit, `next_charge` 1, `charge_point` `model.family-edge`, `stage`, `accounting_contract`) for the S3 family-walk refusal. `ancestor_closure_admits_the_bound_and_refuses_one_past_it` is tagged `TC-795`/`FR-302-AC-5` and checks both admission at n and refusal at n+1 edges. The unit-compile wording that remains is new FND-004 |

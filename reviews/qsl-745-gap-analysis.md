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

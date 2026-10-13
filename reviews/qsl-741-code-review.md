---
id: SR-8706
title: "Code review of quire-spec-language PR #805 (QSL-741): observation values limit refuses resource_exhausted"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@3f013f617751966548c21596b1a3c53652a3400d; qsl-semantics/src/model/observation.rs, qsl-semantics/src/model/observation/document.rs, qsl-semantics/src/model/observation/tree.rs, qsl-semantics/tests/it/child_composite.rs, qsl-semantics/tests/it/state_clauses.rs"
review_set: subset
---
# Code review of quire-spec-language PR #805

## Summary

Ticket: QSL-741. Method: code-review with its rust-review lane. PR #805 (base `impl/business-model-meanings`) replaces `ReadMeter::value` with `ReadMeter::read_value`. A value past `observation.values` now refuses `resource_exhausted`/`insufficient-next-charge` with the typed FR-207 path of the next value. Reviewer-only: no build or test run.

Examined:
- `ReadMeter::read_value` and `values_exhausted`, observation.rs:1083-1144 (examined)
- `read_raw_value` and its `path_to_next`, tree.rs:116-212 (examined)
- the three `read_value` call sites, document.rs:630, 688, 718 (examined)
- `the_exact_values_bound_admits_and_one_below_refuses_resource_exhausted`, `a_deep_chain_is_bounded_by_the_value_limit_and_names_its_path`, `assert_values_exhausted`, `forms_before`, `chain_path` in child_composite.rs (examined)
- `values_per_document_limit_refuses_naming_observation_values` and `a_value_nested_100_000_deep_is_counted_as_observation_values` in state_clauses.rs (examined)
- `limit_refusal` and `LimitExceeded::catalog_fields` (qsl-foundation/src/diagnostic/stage.rs:170-177) (context_only)
- the existing accounting incomplete record, qsl-semantics/src/model/accounting.rs:362-388 (context_only)
- consumers of the old cause: qsl-replay/src/spine.rs:283, qsl-semantics/src/check/refusal.rs:404, qsl-replay/src/spine/native_result.rs:966-1000, src/command/clause.rs:151 (context_only)

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | The new record has no `kind` field. Before this PR the values refusal went through `limit_refusal`, which copied `LimitExceeded::catalog_fields()` (`kind`, `bound`, `actual`, `setting`). `values_exhausted` writes `setting`, `bound` and `actual` by hand and leaves out `kind`. FR-255-AC-1 requires, for every row of the setting table including `observation.values`, a record with the fields `kind`, `bound`, `actual` and `setting`. FR-255 Behavior 1, which this PR edits to name `observation.values`, says the record "carries the same four values". Example: `ObservationLimits::default().with_values_per_document(0)` gives a record with no `kind`. The test tagged FR-255-AC-1 (`values_per_document_limit_refuses_naming_observation_values`) asserts `setting`, `bound`, `prior` and `actual` but not `kind`, so it passes. Fix: start the record from `LimitExceeded::new(Setting::ObservationValues, limit, prior + 1).catalog_fields()`, so the FR-255 field set comes from one place, then add the retained fields. Assert `kind` in that test. | qsl-semantics/src/model/observation.rs:1117-1137; qsl-semantics/tests/it/state_clauses.rs:3068-3083 |
| FND-002 | medium | `accounting_contract` is hard-coded to `quire.value.accounting/v1`, but `observation.values` is not one of that contract's counters. QSpec `value-accounting.md` defines only `ScalarLimitsV1`, `ModelNormalizationLimitsV1` and `PopulationAdmissionLimitsV1`, and none of them has an observation value count. FR-122 says "no `quire.value.accounting/v1` counter names an admission limit". FR-255 says a `quire.value.accounting/v1` budget "has no row in this table" and is not set through `--limit`, but `observation.values` has a row and is set through `--limit`. So every values refusal names a contract it does not belong to. The literal also repeats `qsl-semantics/src/value/definition.rs:188`. Fix: name the contract that really owns the counter (for example the stage's limits type, `ObservationLimits`), once the spec names it (SR-8708 FND-001). | qsl-semantics/src/model/observation.rs:1126,1144; spec/functional/FR-122-replay-a-state-clause-counterexample.md:146; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:113-117 |
| FND-003 | medium | No spec defines the field names `accounting_contract`, `prior` and `work_item`. The repo already has a record for an exhausted accounting charge with QSpec's names: `incomplete { limit_kind, limit, consumed, next_charge, charge_point }` (value-accounting "Incomplete record", qsl-semantics/src/model/accounting.rs:362-376). It covers the same catalog items: effective limit = `limit`, prior usage = `consumed`, work item = `charge_point`. The PR uses different names for the same facts, and they are visible on the wire. Fix: reuse the existing names where they mean the same thing (`consumed` for `prior`, and a point name for `work_item`), or have FR-106 name the fields (SR-8708 FND-004) so the tests assert names the spec defines. | qsl-semantics/src/model/observation.rs:1124-1131 |
| FND-004 | low | The invocation call sites have no test: parameters, which carry `("field", <parameter name>)`, and `result`, which carries `("field", "result")`. Every new test refuses inside a snapshot population field. The deep-chain test checks `field` but not `population` or `object`. If an invocation site passed the wrong locus, or none, every test would still pass. Fix: add one refusal test on an invocation parameter or result that asserts `field` and the path, and assert `population` and `object` in the deep-chain test. | qsl-semantics/src/model/observation/document.rs:688,718; qsl-semantics/tests/it/child_composite.rs:984-1004 |
| FND-005 | low | `the_exact_values_bound_admits_and_one_below_refuses_resource_exhausted` is tagged `FR-359-AC-9` and `TC-1002`. Those cover a child record admitted at an ancestor-typed slot, which this test never exercises. The tags inflate FR-359-AC-9 coverage. Fix: tag only `FR-106-AC-16`. | qsl-semantics/tests/it/child_composite.rs:960 |

## Verdict

The mechanism is sound:
- `read_value` refuses before it increments, so the exact bound admits and `prior` equals the bound. The `+= 1` cannot overflow, because `values < limit <= u64::MAX`.
- `path_to_next` is built only on the refusal path, from the open-form stack. It names the in-progress item of each open sequence or record, the pending field and the option-value step, so the path is relative to the checked field, as FR-207 requires.
- No new panic surface in production code. `#[qsl_attrs::string_edge]` is kept on `read_raw_value`, and `values_exhausted` dispatches on no string.
- Admission refusal category is unchanged: `stage_limit_exceeded` and `resource_exhausted` both map to incomplete (`Code::is_incomplete`).
- No other consumer keys on the old cause. The replay and check `Setting` matches are exhaustive stage maps. `validate_compile_limit` applies only to `stage_limit_exceeded`. No test or doc other than the ones the PR edits asserts `node-count-exceeded` for `observation.values`.

Test oracles: on the old code every new or changed assertion fails at its `code` check (old `stage_limit_exceeded`), and the path assertion also fails (old path empty). The deep-chain path is computed independently from the wire text, so it is not tautological.

`observation.objects` and `observation.input_bytes` stay `stage_limit_exceeded`. That matches the ticket ("leave those alone"), FR-106 check 1.2 for input bytes, and FR-255's table.

No overlap with PR #804 or the QSL-726 stack. **Not mergeable until FND-001 is fixed.** FND-002 and FND-003 depend on the spec ruling in SR-8708 FND-001 and FND-004.

## New findings (disposition pass 1)

Reviewed at `08dcb08376af112b261ef7196049eeef6cd5d1e2`. The PR was redirected: `observation.values` keeps `stage_limit_exceeded`/`node-count-exceeded` through `limit_refusal`. `git diff origin/impl/business-model-meanings HEAD -- qsl-semantics/src` is empty (merge base equals the base tip `ffaefa335`).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | The two new invocation tests carry `#[trace("TC-465", "FR-106-AC-16")]`. TC-465's table has no values-per-document row, and FR-106-AC-16 names no TC. The tag binds TC-465 to a case it does not list, which inflates TC-465 coverage. Fix: tag them `#[trace("FR-106-AC-16")]` only, as the exact-bound test is. | qsl-semantics/tests/it/state_clauses.rs:3137,3160; spec/test-cases/TC-465-admission-refuses-each-input-defect.md |
| FND-007 | low | The PR body's limit-cause audit misstates the `replay.input_bytes` row. It says the current cause is `resource_exhausted`/`input-bytes-exceeded` and that "FR-255 B1 names this form". The code refuses `stage_limit_exceeded`/`input-bytes-exceeded`: `BoundExceeded::catalog_code` is `self.limit().catalog_code()` (qsl-replay/src/bounds.rs:107-110), and `ReplayRequestRefusal::BoundExceeded` maps to `Code::StageLimitExceeded` (qsl-replay/src/request.rs:532). FR-255 Behavior 1 names format, intake and library as the exceptions, not replay. So the row agrees with the rule. The audit's closing argument ("FR-255 B1 itself keeps `resource_exhausted` for library, intake, format and replay rows") goes to the planner's pending ruling with one wrong member. Fix: correct the row to "stage_limit_exceeded \| same \| agrees", and drop "replay" from the closing sentence. | PR #805 body; qsl-replay/src/bounds.rs:66-110; qsl-replay/src/request.rs:532; spec/functional/FR-255-name-the-setting-that-raises-a-reached-limit.md:121-131 |

## Dispositions

| FND | outcome | sha/reason |
| --- | --- | --- |
| FND-001 | fixed | 08dcb08376af112b261ef7196049eeef6cd5d1e2: `values_exhausted` is gone. `ReadMeter::value` refuses through `limit_refusal`, which copies `LimitExceeded::catalog_fields()` (`kind`, `bound`, `actual`, `setting`). The FR-255-AC-1 test now asserts `kind` = `node-count-exceeded`. |
| FND-002 | rejected | mooted by redirect: the record no longer carries `accounting_contract`. `qsl-semantics/src` is byte-identical to the base, which has no such field. |
| FND-003 | rejected | mooted by redirect: `prior`, `work_item` and `accounting_contract` are no longer written. The record is the FR-255 `stage_limit_exceeded` record, whose field names FR-255 Behavior 1 defines. |
| FND-004 | fixed | 08dcb08376af112b261ef7196049eeef6cd5d1e2: `an_invocation_parameter_past_the_values_bound_refuses_stage_limit_exceeded` and `an_invocation_result_past_the_values_bound_refuses_stage_limit_exceeded` refuse at bound 20, actual 21. Each snapshot holds about 5 value forms and the invocation is metered per document, so only the oversized parameter or result can cross 20. Without the charge at that site, the outcome would be a different code. The `field`/path half of the fix no longer applies: QSpec's `stage_limit_exceeded` record has no path field. |
| FND-005 | fixed | 08dcb08376af112b261ef7196049eeef6cd5d1e2: the exact-bound test is now tagged `#[trace("FR-106-AC-16")]` only. |
| FND-006 | fixed | 543558986b1dfcbe7b18d3ad0cc6b744298edfa3: both invocation tests are now `#[trace("FR-106-AC-16")]` only. |
| FND-007 | fixed | 543558986b1dfcbe7b18d3ad0cc6b744298edfa3 (PR body edit with that push): the `replay.input_bytes` row now reads `stage_limit_exceeded/input-bytes-exceeded (bounds.rs, request.rs) \| same \| agrees`, and the closing argument is gone. Its new `explore.*` row is wrong; that is recorded once, as SR-8708 FND-008. |
| FND-008 | still-open | The deferral to QSL-745 is right for the `model.*` half: FR-255 Behavior 1's follow-up (a), the PR audit row "model.* in replay `select` ... NOT fixed here, QSL-745" and QSL-745 (a) all state it. The intake half is stated nowhere. `limit_of`'s `UnitIntakeCause::Refused` arm also turns intake's `IntakeLimitExceeded` into `StageFailure::Limit`, so `stage_limit_exceeded` (qsl-replay/src/spine/lifecycle.rs:206-216), and `select_names_the_intake_limit_for_an_oversize_document` pins that (lifecycle/tests.rs:657-676, tagged FR-260-AC-4). FR-260-AC-4 and FR-255's table say `resource_exhausted`/`intake-limit-exceeded`. Yet the PR audit row says intake "agrees", and FR-255's follow-up list and QSL-745 (a) name only model-normalization ceilings. Fix, text only: add `intake.input_bytes` through `select` to FR-255 follow-up (a), the PR audit row and QSL-745 (a). Then this is `deferred: QSL-745`. |
| FND-009 | fixed | 8cdf7abd4900f9ea06b8a2a04df31bc35e12db02: the denied-charge record now retains `stage` `model-normalization`, `accounting_contract` `ModelNormalizationLimitsV1`, `consumed`, `next_charge` and `charge_point`. The `work_units: 1` case asserts all five. |
| FND-008 | deferred | QSL-745 (a), at a870d4c01658c1fba92b6fcc764dc52636c1bc86. Replay `select` still turns model-normalization ceilings and `intake.input_bytes` into `stage_limit_exceeded` (qsl-replay/src/spine/lifecycle.rs `limit_of`; pinned by `select_names_the_model_limit_field_it_reached` under FR-277-AC-1 and `select_names_the_intake_limit_for_an_oversize_document` under FR-260-AC-4). It is now stated in FR-255 Behavior 1 follow-up (a), in the PR body's audit rows (marked "NOT fixed here, QSL-745 (a)") and in QSL-745 (a) with its intake addendum. Out of scope by the planner-approved scope cut. |

Round 2: Reviewed at `543558986b1dfcbe7b18d3ad0cc6b744298edfa3`. `git diff origin/impl/business-model-meanings HEAD` touches no production code: the only Rust file changed is `qsl-semantics/tests/it/state_clauses.rs`, and the merge base is still the base tip `ffaefa335`.

## New findings (disposition pass 3)

Reviewed at `5ff49edaf955fbb44b7508f2527b35d7ae4a0264`. The base branch moved and was merged in (`d2a16a86c`); the PR's own diff against `origin/impl/business-model-meanings` (`3dde241d3`) changes production code in two places only: `qsl-semantics/src/model/observation.rs` (new `caller_budget_refusal`, used by both `model_views` arms) and `qsl-semantics/src/model/observation/document.rs:1467` (the conformance walk).

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-008 | high | The type rule is not applied in the replay front end. In `select`, a `ModelNormalizationLimits` ceiling becomes a stage limit. `limit_of` turns both `UnitIntakeCause::Limit(incomplete)` and an `ancestor-steps`/`family-steps` refusal into a `LimitExceeded`, and the same arm also catches intake's `IntakeLimitExceeded` (lifecycle.rs:206-220). `failure_of` then returns `StageFailure::Limit` (lifecycle.rs:164-166), which reports `stage_limit_exceeded` (spine.rs:200). FR-277-AC-1 requires this ("`LimitExceeded` with the field's limit kind" for every field of `select`'s limits). Its test `select_names_the_model_limit_field_it_reached` fails on anything but `StageFailure::Limit` (lifecycle/tests.rs:1053-1058, 1174-1226). FR-255's table now says `model.*` refuses `resource_exhausted` "wherever the ceiling is read". So the PR's audit row "model.* and admission.* at compile \| resource_exhausted \| agrees" is wrong for `select`, and so is the PR body's "re-emits the compile path's `resource_exhausted`". Fix: in `limit_of`, route `model.*` and intake ceilings to the `resource_exhausted` refusal, and amend FR-277-AC-1 for `select`. Or the planner states a carve-out for the replay front end in FR-255 and ADR-014 §1. | qsl-replay/src/spine/lifecycle.rs:160-220; qsl-replay/src/spine.rs:200; qsl-replay/src/spine/lifecycle/tests.rs:1053-1058,1174-1226; spec/functional/FR-277-bound-every-lifecycle-operation-by-caller-limits.md:76 |
| FND-009 | medium | `caller_budget_refusal` drops items the catalog requires an `insufficient-next-charge` record to keep. QSpec's `resource_exhausted` row: "For `insufficient-next-charge`, retain the actual stage, accounting contract, effective limit, prior usage and the work item that could not proceed". The admission record carries only `setting`, `bound` and `actual`. `actual` is `consumed + next_charge` (accounting.rs:382-388). Prior usage (`Incomplete::consumed`) and the work item (`Incomplete::charge_point`) are both on hand at observation.rs:868-872 and are dropped. No stage or accounting contract is named either. The new `work_units: 1` case asserts only code, cause, `setting` and `bound`, so it passes. Fix: carry the incomplete record's items (FR-150's names: `consumed`, `next_charge`, `charge_point`) plus the stage, and assert them. | qsl-semantics/src/model/observation.rs:866-873,1048-1061; qsl-semantics/src/model/accounting.rs:362-389; qsl-semantics/tests/it/state_clauses.rs:4320-4335 |

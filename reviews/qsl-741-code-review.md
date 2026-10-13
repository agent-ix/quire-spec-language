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

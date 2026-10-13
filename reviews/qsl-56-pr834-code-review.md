---
id: SR-9088
title: "Code review of quire-spec-language PR #834: conformance stage inside check"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@bd0e216d9804ec6a8da75bd4f2bcd3458fc4027f; PR #834 diff origin/impl/business-model-meanings...feat/conformance-stage-in-check: qsl-semantics/src/check/{assemble,check,mod,model_conformance,field_refinement,refusal}.rs, qsl-semantics/src/model/{accounting,intake/unit}.rs, qsl-replay/src/spine/lifecycle.rs, Cargo.lock, with the rust-review lane folded in"
review_set: subset
---
# Code review of quire-spec-language PR #834

## Summary

Ticket: QSL-56. This review covers the code-review and rust-review lanes in one pass. It reports only defects that give a wrong result, a panic, a failed AC or a broken gate. The oracle is the private QSpec rows TC-196, FR-150, FR-151 and FR-110, cited by id only.

What was checked and found correct:

- Cargo.lock moves only `quire-semantic-value`, from a1fe08a to b7d5531. The workspace check, clippy (all features) and tests compiled every QSV consumer.
- One meter is carried end to end. Production `select_models` returns `SelectedModels { entries, meter }` from `admit_unit_with_cancel`. `assemble_with_cancel` splits it with `into_parts`, and `check` hands the same `Meter` to `model_conformance::check`. `From<Vec<SelectedModel>>` is compiled only under `test-support`, so production has no path that creates a fresh meter.
- The stage runs after the clause refusals are returned and before `complete()` and dispatch linking. A denial returns only the `Incomplete` and drops the failures collected so far.
- `relax_widening` cannot admit a real widening. `complete()` re-runs QSV's widening check over the declared (unrelaxed) types, and the relaxed environment is only kept when `complete()` succeeds.
- The gate logs are qsl56-g3 (workspace minus three crates: one xtask failure, later fixed), qsl56-g4 (xtask 182 passed, EXIT 0), qsl56-g5 (qsl-semantics, eval and package: EXIT 0, finished 14:38:14) and qsl56-cl1 (clippy, all features, EXIT 0). The final head bd0e216 was committed 20 s after g5. It touches only one test file and the xtask scan test, both of which g2, g4 and g5 compiled and ran. `cargo fmt --check` is clean at the head.

## Verdict

FAIL. There are two high findings. A valid field multiplicity narrowing is still refused (FND-001). The conformance-stage denial carries the wrong catalog cause (FND-002).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | A valid field multiplicity narrowing is refused at `complete()`. QSL types every multiplicity other than [1,1] as a bounded Collection, and QSV `narrows` accepts only equal collections. `relax_widening` retypes only the bounded typing environment. `stage_model_with_cancel` receives the declared `object_types` unchanged, so after the stage admits TC-196 R05's admitted rows (A/n {0,5} with B/n {1,3}, and A/n {0,unbounded} with B/n {0,5}), `complete()` refuses them as `RedefinitionWidens`, which reaches the caller as `TypeAdmission` ill_typed. `relax_widening` also matches the declaring type by name. It retypes every redefining field of that type to the widened member's type, whatever that field redefines, so a sibling field's clause typing can change and a clause refusal can come before the stage's own cause. | qsl-semantics/src/check/assemble.rs:1722, qsl-semantics/src/check/assemble.rs:1766, qsl-semantics/src/check/mod.rs:1283 |
| FND-002 | high | `CheckCause::ModelIncomplete` reports `resource_exhausted` with the cause `limit_kind.setting().kind().catalog_cause()`. For `model.work_units` that cause is `work-budget-exceeded`, and for `model.ancestor_steps` it is `edge-count-exceeded`. Both are `stage_limit_exceeded` causes, so the pairing is not in the catalog. FR-255 Behavior 1 gives `ModelNormalizationLimitsV1`, a caller budget, `resource_exhausted`/`insufficient-next-charge`, and dispatch linking already uses that cause on the same meter. The tests assert only the `Incomplete` payload, never the cause string. | qsl-semantics/src/check/refusal.rs:979 |

## Recorded

- medium, recorded: the full clippy run (cl1, 13:43) predates bd0e216, which changed only test and xtask files. Those files were compiled under g2, g4 and g5.
- low, recorded: `members_by_owner` and `declarations` in model/index.rs are now dead.
- low, recorded: the replay `limit_of` no longer projects `ModelIncomplete`, so a conformance-stage denial reaches the caller as `Refused(Check)`, not `Limit`. That is consistent with the caller-budget rows once FND-002 gives it the right cause.

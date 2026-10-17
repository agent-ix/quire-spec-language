---
id: SR-12044
title: "QSL-516 PR #924 code review: CompileRefusal all-causes accessor and refinement classification"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@be1f10ec80fc270c6dd0745787415fc0089ceb1e; qsl-replay/src/spine/refinement.rs, qsl-replay/src/spine.rs, qsl-replay/src/spine/clause/tests.rs, qsl-replay/src/spine/clause/tests/refinement.rs"
review_set: subset
---

## Summary

Ticket: QSL-516. PR quire-spec-language#924, diff `origin/main...HEAD` (4 files, +938). Code review with the rust-review lane. Read-only: no build or cargo run (author reported the focused tests and clippy `-D warnings` green; full `make ci` not yet run).

Checked: `CompileRefusal::causes` against every `CompileRefusal` variant, `group` and `classify_refusal` against the FR-341 rule order (rules 1 to 6), `classify_disposition` and `classify_outcome` against the FR-342 table, `Classification::limits`, exhaustiveness of the top-level matches, and the public re-exports in `spine.rs`. Spec conflicts the author already recorded for the planner (FR-341 rule 2 vs FR-342-AC-5 for Link/Emit, OperationClause in no rule, intake `resource_exhausted`, `CallValue::Reference`, missing Forms/Link/Emit limit data, admission Incomplete naming no limit) are not reported here.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | An `Import` refusal whose I2 read reached an `i2.*` ceiling (`View { Read { refusal: StageFailure::Limit } }`, code `stage_limit_exceeded`) is grouped `Setup` and classified `tool failure`. FR-341 rule 2 excludes an Import's limit cause from tool failure, and rule 4 makes any `stage_limit_exceeded` cause `incomplete`. `causes()` extracts the `ReachedLimit` for this case, but `limits()` then returns nothing because the class is not Incomplete. Only `ImportRefusal::Limit` is grouped `Limit`. | qsl-replay/src/spine/refinement.rs:301-307, qsl-replay/src/spine/refinement.rs:191-195 |
| FND-002 | medium | `check_limit` ends in `_ => None`, so a `stage_limit_exceeded` check cause other than `ResourceExhausted`/`Identity::InputBytes`/`DispatchLink` yields no limit. `CheckCause::TypeAdmission { code: StageLimitExceeded, .. }` (raised from a type-environment ceiling in the check stage via `type_admission`/`pending_refusal`) classifies `incomplete` with an empty `limits()`, which breaks FR-341 Outputs ("for each `stage_limit_exceeded` cause, the limit it names and that limit's value"). The wildcard, and the matching one over `AssemblyCause`, also lets a future limit-bearing cause drop its limit silently. `TypeAdmission` keeps only `code`/`cause`, so the fix needs it to carry its `LimitExceeded`. | qsl-replay/src/spine/refinement.rs:222-237, qsl-replay/src/spine/refinement.rs:164-171, qsl-semantics/src/check/mod.rs:661-667 |

## Verdict

FIX, because of FND-001 and FND-002.

Correct and clean: the top-level matches over `CompileRefusal` (`causes`, `group`) and `ClauseDisposition` have no wildcard arm. The rule order in `classify_refusal` matches FR-341 rules 2 to 6. Vector variants (`Assembly`, `Check`, `Profile`) yield every cause in order, an empty vector yields `runtime_invariant`, and `Dependency` recurses. `Code::from_code` drops an unknown call-refusal code rather than misreading it. The public surface (`classify_compile`, `classify_refusal`, `classify_clause_run`, `Classification`, `RefinementClass`, `RefusalCause`, `ReachedLimit`, `RunClassification`) is minimal and has no panics or unchecked conversions (`widen` saturates).

A note for the planner, not a code defect: a Forms `NoDispatchEntry`/`UnrepresentedConstruct` refusal is `unsupported_construct` with no catalog cause, so it reaches rule 6 "otherwise" and classifies `prohibited`. FR-341 rule 6 names only `declaration-form`/`expression-form`, and no rule covers a cause-less `unsupported_construct`.

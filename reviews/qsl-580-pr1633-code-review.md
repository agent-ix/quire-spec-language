---
id: SR-1633001
title: "Code review of quire-spec-language PR #1633: time units of different owners, Quantity positions (580f)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@3f80fb9ba73f9e6d5dc81d5201e13426e3cd8e09; PR #1633 diff against qsl-580e-clock-reading: qsl-semantics/src/check/{time_units,refusal,mod,timed_model}.rs, check/hybrid/cause.rs; qsl-replay/src/spine.rs; qsl-package/src/emit/tests/timed_source.rs; qsl-semantics/tests/it/{timed_declarations,unit_typed_members,main}.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1633

## Summary

Ticket: QSL-580, slice 580f, stacked on #1632. The Rust review lane is folded
into this review. Reviewed only this PR's diff against
`qsl-580e-clock-reading`.

The unit fix is correct. `TimeUnits::convert` first resolves the unit's scale
on its own owner's base `Time`, so a non-time unit still refuses as before. It
then compares the unit's nominal dimension node with the target
`TimeScale.dimension` and refuses `OtherTimeDimension`
(`ill_typed`/`type-mismatch`) on a mismatch. That matches FR-230 "every
quantity the model reads as time SHALL be a quantity of that node ... and
SHALL otherwise be refused ill_typed/type-mismatch at its span", together with
QSpec FR-142 "Dimensions of different owners". The cause is spec-backed. It is
located at the quantity for a clock constant and at the period for a tick
source, as AC-18 requires.

Moving the code and tag mapping onto `TimeUnitCause::code`/`tag` removes two
copies of the same table. This is a single-source improvement.

Gates: `cargo test -p qsl-semantics -p qsl-package --all-features` ran at this
head through `locked-build.sh`, with `QSPEC_DIR` at QSpec `origin/main`
ccad669c, and passed (exit 0).

## Verdict

**PASS with low findings.** No major bug was found.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | `convert` is shared with `task_set.rs` and `temporal_clause.rs` (timed intervals), so the new refusal now applies to task-set quantities and timed-interval bounds too. That is consistent with FR-142, but no test covers a foreign-`Time` unit on either path. Those callers also keep locating the refusal at the unit span rather than at the quantity. | qsl-semantics/src/check/time_units.rs:299-311; qsl-semantics/src/check/task_set.rs:351; qsl-semantics/src/check/temporal_clause.rs:917 |
| FND-002 | low | The FR-230-AC-15 test misses three things. It does not assert the `Set<Quantity<ms>>` member (`marks`). It checks the operation parameter and result only as `ValueType::Quantity(_)`, not as the one `ms` unit node. It does not check the span of `f(3 s)`'s `type-mismatch` (FR-230: "at the expression"). | qsl-semantics/tests/it/unit_typed_members.rs:80-214 |
| FND-003 | low | `three_time_dimensions_give_three_package_identities` compares identities of three units whose source texts differ (`R::ms`, `F::sec`, `s`). The `package_id` assertions would pass for any source change. The `time_dimension` assertions next to them are the ones that tie the result to the dimension (confidence low that this matters). | qsl-package/src/emit/tests/timed_source.rs:275-330 |

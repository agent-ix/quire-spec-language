---
id: SR-1681001
title: "Code review of quire-spec-language PR #1681: min/max bounds on clocks and units (580n part 1)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@18ba20b8d; PR #1681 diff against qsl-580m-collection-types: qsl-semantics/src/model/{intake,state,domain_package,admitted,index,normalize,systems}.rs; qsl-semantics/src/check/{assemble,timed_model,refusal,protocol_relationship}.rs, lowering/model.rs; qsl-eval/src/simulation/model{.rs,/contracts.rs,/timed.rs}; qsl-replay/src/spine.rs; src/protocol_artifact/domain.rs; tests clock_places, unit_typed_members, timed_subject"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: reviews
---
# Code review of quire-spec-language PR #1681

## Summary

Ticket: QSL-580, slice 580n part 1. The Rust review lane is folded into this
review. I reviewed only this PR's diff against `qsl-580m-collection-types`
at 18ba20b8d. The QSpec unit-bound text comes from FR-154 "Unit bounds" and
AC-28..31 on QSpec `origin/main`.

**AC-25 (clocks).** `read_value_bounds` reads `min`/`max` on a clock alias,
or on an object type with `appliesTo` `ix://quire/native/Clock`. Operands
must be canonical rationals, never negative on clocks. `clock_bounds` turns
each `max` into a `true`-guarded time invariant `x <= c` per owner, on every
clock field (or only the alias-typed ones). A `Family` slot reads every live
slot. `min` refuses `time-invariant-shape`. In qsl-eval, `over` reads only
live places.

**AC-26 / FR-154 (units).** `ValueBound` records come from unit aliases and
from object types with a unit `appliesTo`. `member_bounds` resolves each
`FieldMember`'s type through aliases and sequence items to `Unit(u)`. A
bound on `u` (`alias: None`) applies to every member of `u`, including
members typed through an alias. A bound on an alias applies only when that
alias is on the member's chain. A bound on `ms` never reaches a member of
`s`. Bounds come from the member's own package only. This is the scope
FR-154 and FR-154-AC-29 give. `violation` checks each collection element
and each present option payload, and an absent optional member passes.

**Can a violating state pass?**
- Initial admission refuses `invalid_runtime_input`/`invalid-value` with
  the member, constraint, keyword and bound.
- `contracts::invariants` checks every row of every expanded state and
  records `InvariantViolated` named by the constraint, without pruning.

There are two gaps: quantities nested in a record value, and operation
arguments and results (FND-001, FND-002). Neither can be reached yet,
because no state holds a quantity (FR-106's Quantity snapshot form is not
read, and EN-1 writes no field with an unbounded domain). They are latent,
and they will become live in the next slice.

Gates: `cargo test -p qsl-semantics -p qsl-eval --all-features` ran at
18ba20b8d through `locked-build.sh` (jobs 4, `QSPEC_DIR` at bd73a062) and
passed (exit 0). A local probe of a clock alias of an alias (not committed)
shows that intake refuses `alias → alias → Clock` ("not a clock or a unit
this reader aliases", from 580m), so alias chains are not reachable today
(FND-004).

## Verdict

**CONDITIONAL.** FND-001 is medium. It is a latent soundness gap in the
scope of the unit bound, and it should be closed before quantities reach
states. FND-002 to FND-004 are low.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | A unit bound misses quantities nested in a record value. `member_bounds` emits a `MemberBound` for every `FieldMember`, record fields included, but both checks look a bound up only in the object's own `fields`: initial admission and `contracts::invariants`, via `object.fields.get(&bound.field)`. `violation` also treats a `Value::Composite` as admitted. So a `ms` field of a record held by an object (`Call.t: Span { d: ms }`) is never checked, and a state with `t.d` above `max` passes admission and is not recorded. FR-154 bounds "every member of the selected package that is typed by `u`", and a record's field is such a member. Fix: have `violation` descend into composite slots, matching the record's field keys against the bounds, or resolve bounds per path the way clock places do. Add a test with a bounded quantity inside a record. | qsl-semantics/src/model/state.rs:119-140, 215-245, 257-310; qsl-eval/src/simulation/model/contracts.rs:120-142 |
| FND-002 | low | Operation arguments and results are never checked. FR-154 bounds "a field, an operation parameter or result", and says that a value supplied as "an operation argument, an environment choice or a replayed recorded value" that violates a bound refuses `invalid-value`. `member_bounds` reads only `FieldMember` records, and no argument, result or replay path consults the bounds. It cannot be reached today (no quantity domain to enumerate), so record it with the AC-28 end-to-end deferral, or add the check at application and replay. | qsl-semantics/src/model/state.rs:257-310 |
| FND-003 | low | The crossed-bound refusal looks only inside one type node. `read_value_bounds` compares a constraint with the earlier ones of the same node. A `min 5` on `ms` declared on object type `A` and a `max 3` on `ms` declared on `B` share an `appliesTo`, which FR-154 refuses at the later constraint, but both are admitted. The result is then the unsatisfiable-invariant case rather than a refusal. Compare across the package after all nodes are read. | qsl-semantics/src/model/intake.rs:3031-3100 |
| FND-004 | low | The clock and unit sides read alias chains differently. A unit member collects every alias on its chain (`aliases`), but a clock place keeps only the last alias before `Clock` (`PackagePlace.alias`), and `clock_bounds` keeps only bounds whose `target` is `Clock`. A `max` on an outer alias `Short` of `Timeout` would therefore be dropped for clocks, but would apply for units. Intake currently refuses an alias of an alias (probe above), so this cannot happen yet. Use the same rule as units (a bound on an alias applies to that alias and to every alias of it) before alias chains are admitted. | qsl-semantics/src/check/lowering/model.rs:1092-1105; qsl-semantics/src/check/timed_model.rs (clock_bounds) |

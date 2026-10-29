---
id: SR-1746001
title: "code review (with rust-review) of PR #1746 (QSL-580s): STD-526 template clock_place nodes"
type: SpecReview
analysis: code-review
date: 2026-10-28
scope: "agent-ix/quire-spec-language; PR #1746 git diff origin/qsl-580r-presence...385603d47 (12 files): qsl-semantics lowering/model.rs, timed_model.rs, check/mod.rs; qsl-package emit.rs, checked_v2/timed_model.rs, emit/tests/clock_places.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: reviews
---
# SR-1746001: code review of PR #1746 (QSL-580s)

## Summary

Ticket: QSL-580. Head reviewed: 385603d47, on qsl-580r-presence.

**Vector conformance.** I dumped the emitted `s: Sequence<Clock>[0, 3]` place. Substituting the model digest, the declaring identity and the bound (3 for `unbounded`) into QSpec 7d8d2090 `timed-slot-selector-vectors.json` SL-08 gives a body equal to ours under canonical comparison. The two bodies agree on:
- member order (`model`, `path`, `resolved_path`, `guards`);
- the literal value kinds and their type references (Text 0062ac9e, Integer 07f6dca9, both digests equal to the vectors' `type_nodes`);
- the selector reference to the Integer node;
- the `path_step` integers;
- the slot row `Sequence<ix://quire/native/Clock>[slot]` declared by `Clock`.

The node is self-typed, with occurrence `expression`/0. Its dependencies are Integer plus the model, as in SL-08. The `x` place depends on the model only, as in the vector `v` place. The row rule (field rows, slot rows, nested-sequence rows) also matches SL-01 and SL-05 in code.

**Intake.** `project_path_step` now refuses an index-literal selector (SR-01 shape), and the intake proof covers it. Removing `ClockSlotSelector` loses no reachable form, because QSL emits no `Map` place.

**AC-50.** `has_unwritten_members` (guards, resets, invariants, urgency, `Clock` parameters) refuses `TimedModelNotEmitted`, so a guarded model is never written with empty `guards`. `a_clock_model_with_a_guard_is_not_emitted` covers it.

**Read-back.**
- `a_clock_place_reads_back` passes.
- `a_template_slot_place_reads_back` with `--include-ignored` refuses `MalformedClockPlace` at `/semantic_graph/nodes/34/body/members/2/value/members/0` (the first `resolved_path` row), at lock c103a5e8.
- IR #653 (IR-1083, "validate symbolic timed slots") is merged after that lock. Neither `--precise 2024abf9` (#653) nor IR main compiles QSL as it stands: there is an `ExecutionPointName` `Hash` break. So the IR side cannot be re-checked here, and the ignore stands.

**Rust.** `push_scalar_type` removes the duplicated Text and Rational node blocks. `builtin_integer_type_key` mirrors the Text and Rational helpers.

Gates at 385603d47 (locked-build, jobs 4, `--all-features`, debug info off):
- qsl-package `clock_places`: 4 passed, 1 ignored.
- qsl-package `checked_v2` timed/clock: 65 passed. The only failures are the pre-existing C14/I2 QSpec fixture failures (`positive-model-tuple-union.json` `MalformedWire`).
- qsl-semantics it `clock_places`/`timed`: 97/97.
- clippy `-p qsl-semantics -p qsl-package --all-targets -D warnings`: exit 0.
- fmt: exit 0.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The `a_template_slot_place_reads_back` ignore reason names no ticket ("STD-526 IR delta, luna"); cite IR-1083, which is open and lists vectors SL-01..05 | FR-322-AC-148 |

## Verdict

Mergeable. FND-001 is low.

## Dispositions

Round 1, head addc801df (s-side fixes 2e62b358d, cfe75861c).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | The `a_template_slot_place_reads_back` ignore reason now cites IR-1083 (validate symbolic timed slots, SL-01..05) |

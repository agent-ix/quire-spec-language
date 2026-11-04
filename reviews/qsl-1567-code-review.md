---
id: SR-20261104013115
title: "Code review — QSL-1567 replay-side TraceBinding authenticates every FR-522 dimension (PR #2234)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@d30159139; PR #2234 diff: qsl-eval/src/temporal/evidence/binding.rs, qsl-eval/src/temporal/evidence/mod.rs, qsl-package/src/emit.rs, qsl-package/src/emit/monitor_plan.rs, qsl-package/src/lib.rs, qsl-replay/src/spine/clause.rs, qsl-replay/src/spine/clause/temporal.rs, qsl-replay/src/spine/clause/tests/temporal.rs, qsl-replay/src/spine/clause/tests/temporal/progressed.rs, qsl-replay/src/spine/lifecycle.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-522
    type: reviews
---

## Summary

Ticket: QSL-1567. Code review with the rust-review lane folded in, scoped to the PR diff. The emitter now keeps the `temporal_bindings` rows it writes, keyed by the checked graph's clause `NodeKey` and claim ordinal. The replay spine looks up the row with the same key (`node.identity()`, `node.claim().ordinal()`) and authenticates `Clock`, `ClockParameters` and the member dimensions through one shared `qsl_eval::temporal::evidence::clock_mismatch`, which `TraceBinding::authenticate` also uses. `run_temporal` has a single caller (`run_clause_with_temporal_selections`), and that caller always supplies the rows of the package it just emitted from the same `checked` graph. The keys agree on both sides. The emitter refuses a profile-bearing clause with no selection (`MissingTemporalClockSelection`), so the replay-side no-row branch is defensive only.

## Verdict

**MERGEABLE.** No correctness defects. The dimension order matches FR-522 "Authentication" (Profile, Clock, ClockParameters, then Epoch/SamplePeriod/ClockUnit, SequenceAuthority, TimestampUnit). The comparison runs before any visit, stamp check or meter charge, and the new tests assert zero visits and zero admission objects. There are two low findings: one name and one trace tag.

## Scope

- FR-522 "Authentication" section (dimension order, the no-row rule, and comparison before visit or charge): examined.
- FR-522-AC-3, AC-4, AC-5, AC-6, AC-8: examined against the new tests.
- `TemporalBindingRows` keying in `temporal_binding_rows` (temporal and hyper loops): examined.
- `binding_refusal` and `run_temporal` row lookup: examined.
- Focused tests: `cargo test -p qsl-replay --lib -- spine::clause::tests::temporal` (result in Verdict notes below).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | The name `clock_mismatch` reads as if it returns a mismatch, but it returns `Result<&TemporalBinding, BindingMismatch>` and gives the agreeing row on success. The call `clock_mismatch(...)?` in `authenticate` binds `row`, which reads backwards. Fix: rename it to something like `authenticate_clock` or `match_clock_row`. | qsl-eval/src/temporal/evidence/binding.rs:184; qsl-eval/src/temporal/evidence/binding.rs:158 |
| FND-002 | low | `timestamped_binding_is_required_and_authenticated` is newly tagged `FR-522-AC-3`, but AC-3 is specifically "Over R's fixed-sample clause, a request whose parameters are the EventPosition alternative is refused with ClockParameters". The test asserts `ClockParameters` over the timestamped-event clause instead. The fixed-sample case is already covered by `a_progressed_binding_is_authenticated_against_the_clock_row`. Fix: drop the AC-3 tag from the timestamped test (keep AC-6). | qsl-replay/src/spine/clause/tests/temporal.rs:2614 |

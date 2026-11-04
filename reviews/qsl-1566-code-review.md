---
id: SR-2239001
title: "code-review of QSL-1566 FR-267 temporal wire writer and reader (quire-spec-language PR 2239)"
type: SpecReview
analysis: code-review
scope: "agent-ix/quire-spec-language@f2de892fc132e076b6f43d8e39209b14a888ae66; qsl-replay/src/spine/native_result.rs, qsl-replay/src/spine/native_result/temporal.rs, qsl-replay/src/spine/clause.rs, qsl-replay/src/spine/clause/temporal.rs, qsl-eval/src/temporal/observed.rs, src/command/output.rs, qsl-replay/src/spine/clause/tests/temporal/wire.rs"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-267
    type: reviews
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: references
---
# SR-2239001: code-review of QSL-1566 FR-267 temporal wire (PR 2239)

## Summary

Ticket: QSL-1566. Code review with the rust-review lane folded in, scoped to
`git diff origin/main...HEAD` at f2de892fc. Reviewed: the temporal writer
(`verdict_wire`, `refusal_wire`, `contradiction_wire`, `counterexample_wire`),
the strict reader (`decode`, `validate_temporal`, `validate_counterexample`,
`trace_refusal`, `refuse_temporal_only_outcome`), `TemporalVerdict::basis`,
the `run_clause` counterexample builder, the `ObservedWindowRun.undefined`
plumbing and the CLI exit change.

Checked clean: the CLI exit change only alters temporal runs
(`ClauseDisposition::exit_code` uses `trace_exit_code`, which differs from
`exit_code` only for `Inconclusive`, and only a temporal `Pending` maps to
`Inconclusive`); the writer's basis gate (`TemporalBasis`) matches the reader's
row basis; the reader refuses `pending`/`indeterminate` outside a temporal row;
`decode`'s member paths strip the variant segment correctly; the lasso slot
arithmetic in `counterexample()` matches `ObservedLetters::environment`.

Focused tests: `cargo test -p qsl-replay --lib spine::clause::tests::temporal`
with `QSPEC_DIR` set: 79 passed, 0 failed (without `QSPEC_DIR`, the unrelated
`timestamped_counter_cover_uses_half_open_boundary_rules` panics on the missing
variable).

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | high | An `Undefined` verdict raised by an undefined `on each` guard or capture initializer cannot be written. `ObservedLetters` records `undefined` only in `atom()` (observed.rs:343-347); the engine also returns `Undefined` from `source.activation` (engine.rs:609-610) and `source.captures` (engine.rs:623-626), which never set it. `counterexample()` then returns `None` at `undefined?` (clause/temporal.rs:1082), and `verdict_wire`'s `checked` turns that into `NativeRunResultError::Counterexample`, so `qsl run` fails with a clause-output error instead of writing the `undefined` row. The same silent `None` follows any position mismatch (clause/temporal.rs:1083-1085). Fix: record the first undefined reading in `activation()` and `fix_captures()` too (FR-267 needs a `locus` form for a guard or capture, since neither is a formula atom), and add a run-through-writer test for an undefined guard. Reproduction: the existing `Unit::GuardUndefined` fixture (tests/temporal.rs:143, used by `an_observed_undefined_on_each_guard_settles_inconclusive`, whose FR-331-AC-9 states an observed packet cannot identify an undefined guard) yields an `Undefined` verdict with no atom locus, so its report has no writable form. | qsl-eval/src/temporal/observed.rs:343-347, qsl-eval/src/temporal/engine.rs:609-626, qsl-replay/src/spine/clause/temporal.rs:1079-1112, qsl-replay/src/spine/native_result/temporal.rs:383-389 |
| FND-002 | medium | The reader restores no typed disposition. FR-267 says writing and reading back SHALL restore an equal `ClauseDisposition`, `TemporalRefusal` variants included, but `TemporalRecord` exposes only `contradiction()` and `IndeterminateReasonRead::typed()`; `trace_refusal()` builds the `TemporalTraceRefusal` and discards it (native_result/temporal.rs:1455), and no `TemporalRecord -> TemporalVerdict` exists, so the round trip is asserted only through JSON members. Fix: add `TemporalRecord::disposition()` (or `verdict()` and `trace_refusal()`) returning the typed value, and assert equality with the written report in wire.rs. | qsl-replay/src/spine/native_result/temporal.rs:810-855, qsl-replay/src/spine/native_result/temporal.rs:1448-1465 |
| FND-003 | low | The `violated` interval takes the first interval-carrying operator and yields `null` when that operator's interval is not `Steps` (a time-bounded operator), even when a later operator carries a `Steps` interval. Either skip non-`Steps` intervals in `find_map`, or state in FR-267 what a time interval writes. | qsl-replay/src/spine/clause/temporal.rs:1066-1074 |
| FND-004 | low | rust-style: `WindowRun` is a four-element tuple alias and `(FormulaNodeId, TemporalPosition, UndefinedCause)` is repeated across qsl-eval and qsl-replay; a named struct (`FirstUndefined { atom, position, cause }`) reads better at every `(d, 0, None, None)` site. The `slot` closure always returns `Some` (needless `Option`), and `same_category` / the basis check compare `as_str()` labels instead of deriving `PartialEq` on `Category` and `Basis`. | qsl-replay/src/spine/clause/temporal.rs:1031-1036, qsl-replay/src/spine/clause/temporal.rs:1092-1098, qsl-replay/src/spine/native_result/temporal.rs:1266-1268, qsl-replay/src/spine/native_result/temporal.rs:1479 |

## Verdict

CONDITIONAL. FND-001 is a reachable write failure on a valid run result and
should be fixed in this PR, or the guard/capture case explicitly deferred to a
ticket with a test that pins the current refusal. FND-002 is a real gap
against FR-267's round-trip clause. FND-003 and FND-004 are low.

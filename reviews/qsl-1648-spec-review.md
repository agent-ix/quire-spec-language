---
id: SR-2305003
title: "Spec review of PR 2305: FR-337 clock-unit paragraph and AC-15, FR-194 window rule (QSL-1648)"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@6ac5a341f; spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md; spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md; ticket QSL-1648"
review_set: subset
date: '2026-11-04'
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: references
  - target: ix://agent-ix/quire-spec-language/FR-194
    type: references
---

## Summary

Ticket: QSL-1648. PR quire-spec-language#2305 head 6ac5a341f. Spec diff: FR-337 "Aggregate formula emission" clock-unit paragraph rewritten, FR-337-AC-15 extended with the `past[0 M::s, 300000 M::ms]` conversion; FR-194 window rule gains a unit-mismatch sentence for a `duration` bound that is not a duration of the clock. Checked against QSpec FR-370 "Window aggregate atom" (bounds in the clause's clock unit, converted exactly; any other unit refuses ill_typed/unit-mismatch) and vectors AG-07, AR-24, AR-25. `quire validate` on both files passes.

## Scope

| Unit | Path | Role |
| --- | --- | --- |
| FR-337 "Aggregate formula emission" clock-unit paragraph | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:569-587 | examined |
| FR-337-AC-15 | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:519 | examined |
| FR-194 window rule | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:70-79 | examined |
| FR-194-AC-7 | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:194 | examined |

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | FR-194's new SHALL (a `duration` bound in a non-time unit, or a Time unit that does not convert to the clock unit, refuses ill_typed/unit-mismatch at the bound's unit) has no acceptance criterion: FR-194-AC-7 lists no such case, yet the new test `a_duration_bound_of_another_time_dimension_than_the_clock_refuses` is traced to FR-194-AC-7. Fix: extend AC-7 with `past[0 m, 1 m]` and `past[0 s, 1 m]` (unit-mismatch at `m`) and a source `Time` second under a package-`Time` clock. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:75-78,194 |
| FND-002 | low | FR-337 and FR-194 say a bound "whose unit is not a time unit" refuses unit-mismatch, but the implementation keeps an unknown unit as missing_declaration/missing-name and an affine Time unit as ill_typed/type-mismatch (TimeQuantity). Fix: state the carve-out ("an unknown unit is a missing name, FR-142") or narrow the wording to "a unit of another dimension". | spec/functional/FR-337-emit-a-checked-temporal-clause-as-a-v2-temporal-clause-node.md:574-577; qsl-semantics/src/check/temporal_clause.rs:1193-1206 |

## Verdict

Conditional on FND-001 (one AC row). The FR-337 paragraph now matches QSpec FR-370: the clock unit node is `["scalar_type","unit",T]`, both bounds convert exactly to it, the AC-15 example `past[0 M::s, 300000 M::ms]` → `0`/`300000` of `M::ms` equals AG-07's encoding, and the emitter's InvalidTemporalClockValue guard is kept. The removed "Remaining work: QSL-1648" note is correctly deleted, and the "source-declared unit never names a clock unit" statement is consistent with FR-370's path encoding.

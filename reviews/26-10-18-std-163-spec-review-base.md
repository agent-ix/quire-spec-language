---
id: SR-12884
title: "base review of STD-163 FR-194 undefined window positions"
type: SpecReview
analysis: base
scope: "agent-ix/quire-spec-language@15eca179893ce3d5c95742758a2476f907133f4d; Ticket STD-163; PR #979; spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, spec/test-cases/TC-629-window-aggregates-in-monitors.md"
review_set: subset
---

## Summary

Ticket: STD-163. The change adds the FR-194 rule that an undefined value,
filter or `fraction` predicate at a window position makes the aggregate atom
Undefined with the underlying cause, never skipping the position; gives that
cause precedence over `InsufficientData`; rewords the work charge to "per
window position visited"; and adds FR-194-AC-5 with TC-629 step 4.

Hand-checked AC-5: `x = 2, 1, 0, 4` at positions 0 to 3. `6 / x` is 3, 6,
undefined, 1.5; skipping the zero would give `10.5` (true against `<= 20`),
so the vector separates skip from Undefined. `past[2, 3]` at position 3 holds
positions 0 and 1 (`3 - 3` to `3 - 2`), values 3 and 6, no zero. With
`min_count 5` over a 4-position window the precedence rule applies. A filter
rejecting 10 of 20 positions charging 20 matches PR #966's
`each_window_position_visited_charges_one_work_unit`. Cause names: #966's
`UndefinedCause::{Kernel, Family, InsufficientData}` passes the kernel cause
through (`Kernel`, documented "for example division by zero"); the spec's
`division-by-zero` label matches TC-537/TC-640 usage.

## Verdict

CONDITIONAL. The rule and AC-5 arithmetic are correct. TC-629's expected
result for the `past[2, 3]` case names the wrong positions and gives no
values, so fix it before merge.

## Examined Scope

```yaml
scope:
  - {id: FR-194-AC-5, path: spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, role: examined, excerpt: "Over event-position values 2, 1, 0, 4 of x in past[0, 3], at position 3: sum 6 / e.x over past[0, 3] <= 20 is Undefined(division-by-zero) at position 3, not the sum 10.5 of the other three positions ... A window of positions 0 to 1 only (past[2, 3] at position 3) holds no zero and evaluates normally. A filter that rejects 10 of 20 positions charges 20 work units, not 10."}
  - {id: FR-194-AC-3, path: spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, role: examined, excerpt: "The run's work meter counts 20 for one evaluation over 20 window positions."}
  - {id: TC-629, path: spec/test-cases/TC-629-window-aggregates-in-monitors.md, role: examined, excerpt: "Step 4: Undefined(division-by-zero) for all three terms, also under min_count 5; no InsufficientData; the sum is never 10.5; the window of positions 2 to 3 is read as defined by its values; 20 units."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | TC-629 step 4 expected result says "the window of positions 2 to 3 is read as defined", but `past[2, 3]` at position 3 holds positions 0 and 1 (FR-194-AC-5 says so). Positions 2 to 3 hold `x = 0`, so read literally the expected result contradicts the rule under test. It also gives no value. Expected: sum `3 + 6 = 9` (atom true for `<= 20`), count `2`, fraction `1`. | spec/test-cases/TC-629-window-aggregates-in-monitors.md:34 |
| FND-002 | low | FR-194-AC-5 says the `past[2, 3]` window "evaluates normally" with no expected value, so a wrong window (for example position 3 only, sum 1.5, also true) passes. State 9 / 2 / 1. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:134 |
| FND-003 | low | AC-5 spells `sum 6 / e.x over ...` without `of`, while AC-1 and ADR-024 WA-1 spell `quantile 0.95 of e.latency`. Use `sum of 6 / e.x`. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:134 |
| FND-004 | low | "charges 20 work units" (AC-5) and "the run's work meter counts 20" (AC-3) count the meter total, but #966's `WindowSource` methods charge the meter for their own work too, so a real filter's evaluation adds units. Say "the aggregate charges 20 `CollectionVisit` units", or that the fixture's source charges none. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:132,134 |

## Evidence and Limits

Read-only review of `git diff origin/main...HEAD` and of PR #966's diff
(`qsl-eval/src/temporal/window.rs`, `letters.rs`, tests). No cargo or make
ci run, per the brief.

## Dispositions

Round 1 reviewed at `7d264fc5b` (fix commits `0c8174539`, `7d264fc5b`).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | TC-629:35 now reads "at `past[2, 3]` the sum `9` true, count `2`, fraction `1`"; hand-checked (6/2 + 6/1 = 9; both > 1; 2/2). |
| FND-002 | fixed | FR-194-AC-5 states `9` and true, `count` `2`, `fraction` `1` for `past[2, 3]`. |
| FND-003 | fixed | AC-5 and AC-6 spell `sum of 6 / e.x`. |
| FND-004 | fixed | AC-3 and AC-6 count "visit units", and FR-194 says visit units are separate from the units a filter, value or predicate charges. |

## New findings (disposition pass 2)

Round 2 reviewed at `a173ab896` (#979) and `e938dee` (QSpec #277). The round-1 rows above stay current for FND-001 to FND-004.

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-005 | low | AC-5's new vector "with `min_count 5` and a window of three defined values" names no term or window. Over `x = 2, 1, 0, 4`, every three-position window at position 3 contains the zero. Name the term: the guarded `sum of 6 / e.x where holds(e.x != 0) over past[0, 3]` with `min_count 5` is `InsufficientData{3, 5}`. TC-629 step 4 has the same wording. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:146; spec/test-cases/TC-629-window-aggregates-in-monitors.md:35 |

## Dispositions (disposition pass 3)

Round 3 reviewed at `d6a9b2c2a` (#979) and `b825dda` (QSpec #277).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-005 | fixed | AC-5 and TC-629 step 4 name the guarded sum with `min_count 5` as `InsufficientData{count: 3, min_count: 5}`. Hand-checked: the filter admits positions 0, 1 and 3, so the count is 3. |

## New findings (disposition pass 3)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-006 | low | AC-6 and TC-629 step 5 say "the clause-level charge is 3 units (1 at position 0, 2 at position 1)". That counts window-visit units only. The clause's meter also carries FR-327's per-node visit units and the letter read's own unit (#966 `letters.rs`), so a test reading the meter sees more than 3. Say "clause-level visit charge", as AC-3 does. Also, the parenthetical "a newest-first visit would ... stop `Incomplete`" holds only when the limit is checked before the value is read (#966's order). The 2-unit charge is the discriminator that does not depend on order. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:147; spec/test-cases/TC-629-window-aggregates-in-monitors.md:36; quire-specification:spec/test-cases/TC-366-window-aggregates.md:32 |

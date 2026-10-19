---
id: SR-12885
title: "failure-domain review of STD-163 FR-194 undefined window positions"
type: SpecReview
analysis: failure-domain
scope: "agent-ix/quire-spec-language@15eca179893ce3d5c95742758a2476f907133f4d; Ticket STD-163; PR #979; spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, spec/test-cases/TC-629-window-aggregates-in-monitors.md"
review_set: subset
---

## Summary

Ticket: STD-163. Failure modes of the new undefined-position rule: the cause
precedence, multiple undefined positions, early stop, the window limit, and
filter-guarded values.

Precedence (undefined over `InsufficientData`) is sound. Both outcomes are
O-16 undefined at that position, so only the cause differs. The truth result
cannot improve, and an evaluation defect is the more informative cause to
report.

## Verdict

CONDITIONAL. The rule leaves which cause is reported, and what the evaluation
still does after the first undefined position, open. Two conforming evaluators
can report different results for one trace, which WA-6's "deterministic
function of the trace" forbids.

## Examined Scope

```yaml
scope:
  - {id: FR-194, path: spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, role: examined, excerpt: "When the value expression, the filter or the fraction predicate is undefined at a window position (for example a division by zero), the atom SHALL evaluate to Undefined carrying that undefined evaluation's own cause unchanged ... The evaluator SHALL NOT skip the position ... When the same window both holds an undefined position and fewer than min_count values, the atom carries the undefined position's cause, because the count is not known."}
  - {id: FR-194-AC-5, path: spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md, role: examined, excerpt: "With min_count 5 the same window gives Undefined(division-by-zero), not InsufficientData."}
  - {id: ADR-024, path: spec/decisions/ADR-024-statistical-and-probabilistic-properties.md, role: context_only, excerpt: "WA-6 Meaning. The truth of an aggregate atom at a position is a deterministic function of the trace."}
```

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | medium | Several outcomes are left open. (1) A window with two undefined positions of different causes: which cause wins (the oldest position, the first one visited, or another)? (2) Does evaluation stop at the first undefined position? That changes the work charged, since the rule charges "per window position visited". (3) If evaluation continues, a later `max_window_values` overflow (#966 checks it inside the loop) would make it `Incomplete` instead of `Undefined`. State one rule, for example: positions are visited oldest first, and evaluation stops at the first undefined position with its cause, charging the positions visited so far. Add a vector with two causes. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:104-116 |
| FND-002 | medium | The rule says a value expression "undefined at a window position" makes the atom Undefined. It does not say the value and the `fraction` predicate are read only at positions the filter admits. An evaluator that evaluates eagerly would make the guard idiom `sum of 6 / e.x where holds(e.x != 0)` undefined. #966 reads the value only after `admits`. Restrict the rule to admitted positions and add that guarded vector to AC-5. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:104-108 |
| FND-003 | low | The precedence rationale "because the count is not known" holds only for an undefined filter. With an undefined value under a defined filter the count is known, and in AC-5's `min_count 5` case the 4-position window is short whatever the zero gives. The decision is sound; give the real reason instead: an evaluation defect outranks lack of data, and both are O-16 undefined. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:111-113 |

## Evidence and Limits

Checked against PR #966's `AggregateAtom::evaluate` loop (charge, then
`admits`, then the limit check, then `take`). That code has no undefined
path yet; its review deferred that gap (#966 FND-002) to this ticket.

## Dispositions

Round 1 reviewed at `7d264fc5b` (fix commits `0c8174539`, `7d264fc5b`), with QSpec PR #277 at `66f9538`.

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-001 | fixed | FR-194 and FR-414 (#277) now say: visit oldest first, stop at the first undefined position, carry its cause, leave later positions and a later `max_window_values` overflow unreached, and charge the positions visited up to and including it. Hand-checked FR-194-AC-6: positions 0 and 1 are visited (6/1, then 6/0), 2 units are charged, and with `max_window_values` 2 the count reaches 2 only at position 1, so the overflow (a third admitted value at position 2) is never reached. New FND-004 covers that vector's second cause. |
| FND-002 | fixed | The rule now covers an undefined filter at a visited position and an undefined value or predicate at an admitted position. The guarded `sum of 6 / e.x where holds(e.x != 0)` = 3 + 6 + 1.5 = 10.5, which is `<= 20` and true; that vector is in AC-5 and TC-366 WA-08. |
| FND-003 | fixed | The reason is now "the first fault found and the count is not complete", which is true under stop-at-first. |

## New findings (disposition pass 1)

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-004 | medium | The second cause in FR-194-AC-6 and TC-366 WA-09, "no `x` at position 4 (a different undefined cause)", is not a defined undefined cause. QSL nowhere makes an absent observation field undefined; FR-098-AC-8 refuses an absent field as `WrongValueKind`. A trace with a missing field can therefore be refused before any atom is evaluated. The output also cannot show "from position 1", because the atom's cause carries no window position (`UndefinedEvaluation.at` is the clause position). Oldest-first is then observable only if the second cause is a named, distinct undefined cause, for example a family `absent-key` lookup at position 4, or another kernel undefined variant. Name it. | spec/functional/FR-194-evaluate-window-aggregates-in-monitors.md:145; quire-specification:spec/test-cases/TC-366-window-aggregates.md:32 |

## Dispositions (disposition pass 2)

Round 2 reviewed at `a173ab896` (#979) and `e938dee` (QSpec #277).

| FND | Outcome | Reason |
| --- | --- | --- |
| FND-004 | still-open | The second cause is now `none-value` from `value(o)` over `None`. FR-107:231 says that under `CheckMode::Linked` no clause reaches a kernel `Undefined` from `value(..)`, because it carries a presence obligation (FR-104: refused `undefined_expression`/`unproved-presence` without a presence fact). `quire-exact`'s `Undefined::NoneValue` doc says "Only direct kernel evaluation of an unlinked expression can meet it". So the vector may be refused at checking, or is out of reach in a monitor; FR-100 only spells the reason. Minimal fix: drop `o` and the second cause. With `sum of 6 / e.x over past[0, 3]` over `x = 1, 0, 2, 3` at position 3, oldest-first is already pinned: oldest first visits 0 and 1 and stops (2 units, `division-by-zero`); newest first would visit 3 and 2, then meet position 1 as the third admitted value with `max_window_values` 2 and give `Incomplete`, or charge 3 units without the limit. Same in QSpec TC-366 WA-09. |

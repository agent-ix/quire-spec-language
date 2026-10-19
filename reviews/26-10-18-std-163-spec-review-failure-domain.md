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

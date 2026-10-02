---
id: FR-249
title: "Lower a task automaton to a timed subject"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-230
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: depends_on
---
# FR-249: Lower a task automaton to a timed subject

## Description

A timed model whose operations release tasks of a task set is a task
automaton. For the decidable class, S3 SHALL lower the model and its
scheduling policy to a timed subject by the task-automaton encoding, and
EN-6 SHALL check "no task misses its deadline" as a timed invariant
(ADR-026 RT-8). A model in the undecidable class, interval execution times
together with completion feedback and preemption, SHALL settle
`unsupported`, `StopwatchRequired`.

## Use case

An embedded engineer's controller releases a sensor task on each sample
and an actuator task on each command. The engine checks that no released
task misses its deadline over every behaviour of the controller, and tells
the engineer when the model needs stopwatches that no decidable method
handles.

## Inputs

- A checked timed model whose operations carry `releases τ` members naming
  tasks of a checked task set (FR-246), and a scheduling policy.

## Outputs

- A derived timed subject with a ready queue and the clocks the encoding
  needs, and the derived item `always holds(not deadline_missed)`; or the
  disposition `Unsupported(StopwatchRequired)`.

## Behavior

- `releases τ` SHALL name a task of the task set the model references,
  refusing `missing_declaration`/`missing-name` otherwise.
- The checker SHALL classify the model as undecidable when its task set has
  interval execution times, its model reads task completion, and its
  policy is preemptive; the item SHALL then be disposed
  `Unsupported(StopwatchRequired)` and routed to no engine.
- Otherwise the checker SHALL lower the model by the task-automaton
  encoding, with the bounded clock subtraction it needs internal to the
  lowering, and give the derived timed subject to EN-6 with the item
  `always holds(not deadline_missed)`.
- The lowering SHALL derive the ready-queue length from the task set.
- A counterexample SHALL be a timed counterexample over the source model's
  operations with the release and completion steps it implies, replayable
  by FR-237 over the derived subject.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-249-AC-1 | A model with one operation `sample` (guard `x = 4 ms`, reset `x = 0`, `releases a`) and one `command` (guard `y = 6 ms`, reset `y = 0`, `releases b`) over `Ctl`'s first two tasks under fixed priority returns `Holds`. With `a`'s WCET raised to 4 it returns `Violated` with a counterexample that replays. | Test (TC-704) |
| FR-249-AC-2 | The same model with `a`'s WCET an interval `[1 ms, 2 ms]`, a `when done(a)` guard on `command`, and the fixed-priority preemptive policy is disposed `Unsupported(StopwatchRequired)`; with the WCET a single value it is lowered and checked. | Test (TC-704) |

## Dependencies

- ADR-026 §12 RT-8.
- [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md),
  [FR-246](FR-246-check-task-sets-and-schedulability-claims.md).

## References

- QSpec half: QSpec FR-419 (Linear STD-139) owns the `releases` grammar
  (ADR-026 OV-10).
- E. Fersman, P. Krcal, P. Pettersson and W. Yi, 2007 (ADR-026
  References).

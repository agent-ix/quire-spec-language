---
id: FR-175
title: "Evaluate a hyper body over a tuple of behaviours in lockstep"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-021
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-023
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-125
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-172
    type: depends_on
---
# FR-175: Evaluate a hyper body over a tuple of behaviours in lockstep

## Description

The layer-5 TemporalTrace evaluator SHALL evaluate a hyper clause's body
over a tuple of behaviours, one per trace variable, aligned in lockstep
(ADR-023 HM-1, HM-2, HM-4, HM-5, HM-8). Each indexed atom reads its own
component's observation at the joint position. On a tuple of lassos the
joint behaviour is ultimately periodic, and the evaluator returns the same
value SM-1's evaluator returns on one trace. Replay (FR-183) and the
engines' tests use this one evaluator.

## Use case

A verification operator, or an auditor, holds two lassos of a vault model.
The evaluator reads `v.l @ a = v.l @ b` at each joint position, with each
trace's loop repeated until both close together, and returns `false` at
the first position where the outputs differ.

## Inputs

- A checked hyper body (FR-172).
- A tuple of component traces, one per variable in quantifier order, each a
  model trace (FR-125): a finite prefix and, for a lasso, a loop entry, with
  terminal stutter where a component ends at a terminal state.
- The instance's object parameter binding.

## Outputs

- The body's value on the joint behaviour (`true`, `false` or no value, as
  SM-1's evaluator returns), with the first joint position at which a
  `false` is fixed, for `trace_position`.

## Behavior

- **Domain.** A variable `a of M` ranges over the behaviours of `M`'s
  subject that satisfy `a`'s fairness set, read on `a`'s own states and
  steps (ADR-023 HM-1).
- **Lockstep.** Joint position `j` SHALL be position `j` of every
  component, and joint step `j` the tuple of the components' steps into
  position `j`.
- **Terminal stutter.** A component at a terminal state SHALL take its
  stutter step at every later joint step, with step label `stutter`, outside
  every fairness constraint (ADR-023 HM-5).
- **Tuple of lassos.** The joint behaviour SHALL have as its stem the
  longest component stem and as its period the least common multiple of the
  component loop lengths, each component indexed by ADR-014 TR-2's
  prefix-then-loop rule and read by ADR-018 SM-8, so past operators see the
  same history at a loop position and at its unrolled copies.
- **Indexed atoms.** An atom SHALL evaluate through the one clause
  evaluator (FR-107), each member read `e @ t` reading component `t`'s
  observation at the joint position, with object parameters bound to their
  universe keys in every component of their alias.
- **Intervals.** An interval operator SHALL count joint positions.
- **Work.** The evaluator SHALL charge one TR-5 work unit per (temporal
  node, joint position) visit, and a clause meter SHALL stop it as FR-125
  states.
- The value SHALL be a function of the body, the traces and the binding.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-175-AC-1 | ADR-023 §8.1's leaky counterexample, `a` from `(0, 0)` and `b` from `(1, 0)` each taking `step(0)` with loop entry 1, evaluates `always holds(v.l @ a = v.l @ b)` `false` at joint position 1. The same pair over the secure model evaluates `true`. | Test (TC-600) |
| FR-175-AC-2 | A tuple of a lasso with loop length 2 and one with loop length 3 evaluates over a joint period of 6: `always holds(v.l @ a = v.l @ b)` is `false` on a pair whose outputs first differ at the fifth position of the joint loop, and `trace_position` names that joint position. A past operator under `always` gives the same value at a loop position and at its unrolled copy. | Test (TC-600) |
| FR-175-AC-3 | A tuple where `a` ends at a terminal state and `b` does not reads `a`'s step label as `stutter` at every later joint step and its atoms at its terminal state. `eventually[0,2] holds(v.l @ a = 1)` counts joint positions. | Test (TC-600) |

## Dependencies

- ADR-023 §2 HM-1, HM-2, HM-4, HM-5, HM-8; ADR-014 TR-2 and TR-5 and A-4
  as amended by ADR-023; ADR-018 SM-1, SM-3, SM-8.
- [FR-125](FR-125-read-a-model-subject-s-behaviours-as-temporal-traces.md)
  (one behaviour as a trace), [FR-172](FR-172-check-hyper-and-relation-clauses-over-model-subjects.md).
- QSpec owns lockstep alignment and tuple evaluation over behaviours
  (ADR-023 QS-2).

## References

- ADR-023. QSpec half: Linear STD-136 (ADR-023 QS-2).

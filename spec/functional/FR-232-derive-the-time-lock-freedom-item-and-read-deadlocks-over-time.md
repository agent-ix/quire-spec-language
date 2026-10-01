---
id: FR-232
title: "Derive the time-lock-freedom item and read deadlocks, fairness and vacuity over time"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-026
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-026
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-124
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-231
    type: depends_on
---
# FR-232: Derive the time-lock-freedom item and read deadlocks, fairness and vacuity over time

## Description

QSL's request writer SHALL add one derived **time-lock-freedom** item for
each distinct timed subject among a request's items, beside FR-124's
deadlock-freedom item (ADR-026 TD-2). A time-lock is a reachable timed
state from which no time-divergent behaviour exists; it silently removes
behaviours, so it is always reported. QSL SHALL also read deadlocks,
enabledness, weak fairness and vacuity over time (TD-4 to TD-6).

## Use case

A verification operator writes a server that replies strictly after its
own deadline. Every claim over the model would then hold vacuously from the
stuck state. The time-lock-freedom item reports the state with a prefix
that reaches it, and the deadlock-freedom item does not misreport it as a
deadlock.

## Inputs

- The request's temporal items over model subjects (FR-125), each with its
  checked package.

## Outputs

- A `TimeLockFreedom` request item per timed subject: requirement
  (`temporal-satisfaction`, `Unbounded{domains}`), an obligation identity
  made of the subject and the fixed item kind `time-lock-freedom`.
- The predicate `deadlocked` over a timed state, and enabledness and
  fairness over time, used by FR-239 and FR-237.

## Behavior

### The time-lock-freedom item

- The request writer SHALL add one `TimeLockFreedom` item per distinct
  timed subject among the request's items. Two items over equal subjects
  SHALL share one.
- The request writer SHALL add it regardless of the subject's `terminal`
  member; it has no opt-out.
- The item's obligation identity SHALL be the subject and the kind
  `time-lock-freedom` (ADR-013 O-09), distinct from the deadlock-freedom
  item's and every authored claim's.
- The item SHALL be routed and negotiated as an unbounded temporal item
  and settled by FR-235.
- A **local time-lock** SHALL be a reachable timed state at which no
  positive delay is admissible and no discrete step is enabled. A
  **non-local time-lock** SHALL be a reachable timed state, not local, all
  of whose continuations are Zeno.

### Deadlocks over time

- A timed state SHALL be terminal when no discrete step is enabled at it or
  after any admissible delay.
- A terminal timed state at a quiescent discrete state SHALL be deadlocked
  or intended by FR-124's rule, and the deadlock-freedom item reports it.
- A terminal timed state where delay is bounded SHALL be a local
  time-lock, reported by the time-lock-freedom item and not by the
  deadlock-freedom item.
- FR-124's `deadlocked` predicate SHALL quantify over admissible delays.

### Fairness over time

- A transition identity SHALL be enabled at a timed state when its data
  precondition and a convex part of its guard hold there and the step's
  target satisfies its time invariants.
- A behaviour SHALL satisfy weak fairness for a constraint when from no
  instant onward the constraint stays enabled at every instant, positions
  and the delays between them, while none of its transitions is taken.
- An accepting cycle SHALL refute a claim only when it is fair and its loop
  has positive total delay.

### Vacuity

- A proof over a timed subject whose initial state has no fair
  time-divergent behaviour SHALL settle `inconclusive`,
  `NoAdmittedBehaviour` (FR-235), never `proved`.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-232-AC-1 | A request with `NoLateReply` and `Settles` over one `Rpc` subject (ADR-026 §11) carries one deadlock-freedom item and one time-lock-freedom item. A request with one claim over `Rpc` and one over the untimed `Counter` subject carries one time-lock-freedom item, for `Rpc`. With `terminal any` added to `Rpc`, the time-lock-freedom item is still present. Its obligation identity differs from the deadlock-freedom item's. | Test (TC-687) |
| FR-232-AC-2 | In the strict-guard `Rpc` variant (`reply` needs `x > 3 ms`, `timeout` needs `x > T`, `T = 3 ms`), the timed state `(Waiting, inflight, x = 3)` is a local time-lock; `deadlocked` is `false` there, and the state `(Replied, x = 2)` of the unmodified model is quiescent and not terminal. | Test (TC-687) |
| FR-232-AC-3 | The `Stall` model (one object, clock `x` never reset, `time invariant when true { x <= 1 ms }`, and an operation `ping` with no guard that changes no data field) has no time-divergent behaviour from its initial state: `always holds(true)` under the timed profile settles `inconclusive`, `NoAdmittedBehaviour`, and the timed state `(s0, x = 1)` is a non-local time-lock, since `ping` is enabled there. | Test (TC-687) |
| FR-232-AC-4 | In the `Serve` model (clock `x` never reset; `idle` with no guard and no data change; `serve` with guard `x >= 1 ms` setting `done`), the timed lasso that takes only `idle` steps with `x > 1` and loop delay 1 is unfair under `fair weak serve`, because `serve` is enabled at every instant of the loop and never taken. In the variant where `serve`'s guard is `x <= 1 ms`, the same lasso is fair, because `serve` is disabled at every instant of the loop. | Test (TC-687) |

## Dependencies

- ADR-026 §4 TD-1 to TD-6; ADR-018 DL-1 to DL-3, FA-2 and V-6; ADR-013
  O-09 and O-20.
- [FR-124](FR-124-declare-intended-terminal-states-and-derive-deadlock-freedom.md),
  [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md),
  [FR-231](FR-231-read-a-timed-subject-s-behaviours-as-timed-traces.md).

## References

- QSpec half: QSpec FR-417 (Linear STD-139) owns the time-lock-freedom
  item, time-locks and the `TimeLock` counterexample kind on the wire
  (ADR-026 OV-5, OV-6).

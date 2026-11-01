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
  - target: ix://agent-ix/quire-spec-language/FR-237
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: depends_on
---
# FR-249: Lower a task automaton to a timed subject

## Description

A timed model whose `release S.τ on M::o;` declarations bind source
transitions to tasks of a checked task set is a task automaton. S3 SHALL
compile it with the selected scheduler to an ordinary unit-rate timed product
and EN-6 SHALL check `always holds(not deadline_missed)` on that product.
The product SHALL preserve the source state and exact job identities while
representing scheduler state, release and deadline clocks, selected execution
demand, and completed service. Preemption pauses service without introducing
stopwatch rates: all product clocks advance at rate 1, and bounded clock
subtraction plus diagonal constraints/resets represent service accumulated
across run segments (ADR-026 RT-8; QSpec FR-419, FR-415, FR-418).

The engine SHALL decide the exact class covered by the task-automata result:
fixed computation times, and the supported variable-demand/no-completion-
feedback cases for which the checked worst-case reduction applies, using a
finite checked region/product certificate. For variable or interval demand
with completion feedback and preemption, and for products with
mixed-criticality budget changes, it SHALL run sound bounded exploration.
It SHALL return a concrete
refutation only for a replayed deadline miss with a checked time-divergent
continuation. If bounded exploration cannot establish either a complete
proof or such a refutation, it SHALL return `incomplete` with replayable
`task_search` evidence. These cases are admitted; undecidability means that a
run may remain unknown, not that the input is unsupported.

## Use case

An embedded engineer checks that every task released by a timed controller
meets its deadline. A proof covers all admitted task choices; a refutation
contains a realizable late job and a time-divergent continuation; a bounded
run that cannot decide supplies evidence that an independent reader can
replay.

## Inputs

- A checked timed model, task set, `release S.τ on M::o;` declarations and
  scheduling policy. Release binding, arrival spacing, jitter, task/job
  identity, demand selection and budget-change semantics are those of QSpec
  FR-419. A fixed-work task has a singleton actual demand as defined by that
  requirement; a WCET by itself remains an upper bound and does not select
  that class.
- EN-6 exploration and certificate limits (FR-239).

## Outputs

- A task-derived timed subject and an EN-6 outcome: a checked exact
  `Holds`, a replayed `Violated`, `Undecided`, or `Stopped` outcome. A stopped
  bounded task search carries QSpec FR-331 `task_search` frontier evidence;
  it does not produce `Holds`.

## Behavior

- `release S.τ on M::o;` is the model-level declaration of QSpec's shared
  grammar, not a member of an operation. The checker SHALL resolve `S` to a
  task set of the unit, `τ` to a task of `S`, and `M::o` to an operation of a
  model, refusing `missing_declaration`/`missing-name` or
  `ambiguous_declaration`/`ambiguous-name` as applicable. It SHALL refuse a
  second declaration of the same task on the same operation as
  `conflicting_declaration`/`duplicate-declaration` at the second
  declaration. It SHALL record each checked release in
  `CheckedGraph::task_sets()` for lowering.
- The task product SHALL synchronize each admitted source release with its
  task-set activation contract. It SHALL retain the source control state,
  source transition identity, nominal activation, actual release after
  selected jitter, absolute deadline, task identity, job occurrence and
  generation, selected actual demand, completed service, remaining demand,
  scheduler mode and priority. Preemption, resumption, completion and budget
  change are producer-owned derived transitions with stable identities and
  replay provenance; they SHALL NOT be represented as source operations.
- During a product delay of `d`, every clock advances by `d`; the scheduled
  job's completed service advances by `d`, and every waiting or preempted
  job's completed service is unchanged. A preemption SHALL preserve the
  interrupted job's identity, selected demand, service, priority and
  absolute deadline. Completion feedback SHALL synchronize with the derived
  completion transition. The lowering SHALL implement this paused-service
  meaning using the bounded-subtraction/diagonal clock construction of
  FR-238 and QSpec FR-415, with the finite ceilings and subtraction guards
  supplied by the checked task product. It SHALL NOT add non-unit clock
  rates, infer service from a reset that loses prior segments, or equate WCET
  with actual demand.
- The **exact task-automata class** consists of (a) products whose actual
  demand is a singleton for each job, or (b) variable-demand products with
  no completion feedback for which the task-automata worst-case reduction
  applies, together with admitted arrivals, finite scheduler control and a
  refined region/product closure accepted by the independent certificate
  checker (FR-239; QSpec FR-418). EN-6 SHALL decide this class: an accepted
  closed certificate with no deadline miss settles `Holds`; a refutation requires a
  concrete deadline-miss prefix and a replay-checked time-divergent
  continuation. A finite positive-delay loop or QSpec FR-364 `queue_growth`
  continuation may establish that continuation. Queue-cap overflow alone is
  an internal search event, never a deadline miss and never a refutation.
- Products outside the exact fixed-work class, including variable/interval
  actual demand with completion feedback and preemption and active-job
  mixed-criticality budget deltas, SHALL remain admitted. EN-6 SHALL explore
  these products soundly within its limits, retaining the selected demand,
  service, source and derived transition identities, scheduler order and
  budget changes in every state and edge. A discovered miss can settle
  `Violated` only after FR-237 reproduces that exact job's positive remaining
  demand at its original deadline and verifies a positive-time divergent
  continuation. A run SHALL settle `Holds` only when its independently
  checked certificate covers every allowed demand choice and mode-change
  successor. If exploration reaches a limit, cannot close a sound finite
  abstraction, or lacks a checked continuation, it SHALL settle
  `incomplete`/`limit-reached` with a QSpec FR-331 `task_search` record that
  replays the selected effective bounds, explored prefix and remaining
  frontier. It SHALL NOT turn an unclosed search into `Holds`, a miss-only
  refutation, or `Unsupported(StopwatchRequired)`.
- The checker SHALL continue to admit a `release` declaration on a model
  without a `time dense` source. When S3 is asked to lower that model, the
  task automaton lowering SHALL return the existing typed timed-model
  refusal before invoking EN-6.
  For every admitted dense-time task product, the lowering SHALL preserve
  FR-231's scope: only
  time-divergent behaviours can refute the timed invariant, and FR-417
  time-lock outcomes remain separate.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-249-AC-1 | The `sample`/`command` model over `Ctl`'s first two tasks under fixed priority returns `Holds` when its exact fixed-work product has an accepted finite certificate. With `a`'s exact work raised to 4 and the command deadline missed on a time-divergent behaviour, it returns `Violated` with a task-derived counterexample that FR-237 replays. | Test |
| FR-249-AC-2 | With interval actual demand, a `when done(a)` source guard and preemptive fixed-priority scheduling, the model is admitted and bounded exploration retains the chosen demand across preemption and completion feedback. A limit reached before a complete certificate or replayable miss returns `incomplete` with task-search frontier evidence; it never returns `Unsupported(StopwatchRequired)` or `Holds` from a bounded prefix. | Test |
| FR-249-AC-3 | A preempted job's service does not increase during an elapsed delay, while the running job's service and all clocks increase by that delay; resumption preserves the original job identity, demand, priority and deadline. The resulting trace replays exactly. | Test |
| FR-249-AC-4 | A HI job's actual demand remains the value selected at activation across a LO-to-HI source transition; the budget allowance increases by exactly `C(HI)-C(LO)` while the same job's identity, service, priority and absolute deadline remain unchanged. A bounded result without exhaustive checked coverage settles incomplete. | Test |
| FR-249-AC-5 | The overload model with periodic `T=1`, exact work `W=2` and `D=1` may use a checked finite-loop or `queue_growth` continuation after a concrete missed-job prefix to refute; an overflow signal without that prefix and continuation does not refute. A task-search stop with an unexplored frontier replays as incomplete. | Test |
| FR-249-AC-6 | Two task releases bound to one operation are recorded in source order; a duplicate release of one task on that operation refuses `conflicting_declaration`/`duplicate-declaration`; unresolved task set, task or operation names refuse `missing_declaration`/`missing-name`; an ambiguous inherited operation refuses `ambiguous_declaration`/`ambiguous-name`; a `release` on a model without `time dense` is admitted by checking and refused only when lowering is requested. | Test |

## Dependencies

- ADR-026 §12 RT-8 and §8 EZ-3 to EZ-5.
- [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-237](FR-237-replay-a-timed-counterexample.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md),
  [FR-246](FR-246-check-task-sets-and-schedulability-claims.md).

## References

- QSpec FR-419 owns task automaton inputs, demand and activation semantics;
  FR-415 owns bounded subtraction and diagonal constraints; FR-418 owns the
  independently checked finite-product gate; FR-331 owns replayable bounded
  task-search evidence; FR-364 owns derived transition replay.
- E. Fersman, P. Krčál, P. Pettersson and W. Yi, “Task automata:
  Schedulability, decidability and undecidability” (Information and
  Computation 205(8), 2007), Theorems 2 and 5. Fixed computation times admit
  decidable schedulability analysis;
  interval execution with completion feedback and preemption does not admit
  that exact result. The latter is the reason bounded search may return
  unknown, not a refusal.

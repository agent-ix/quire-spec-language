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
  - target: ix://agent-ix/quire-spec-language/FR-238
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-239
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-246
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-364
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-415
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-418
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-419
    type: depends_on
---
# FR-249: Lower a task automaton to a timed subject

## Description

A timed model whose `release S.τ on M::o;` declarations bind source
transitions to tasks of a checked task set is a task automaton. S3 SHALL
compile it with the selected scheduler to an ordinary unit-rate timed product
and EN-6 SHALL check `always holds(not deadline_missed)` on that product.
The product SHALL preserve the source state and exact job identities while
representing the scheduler state and task-derived timed data. Its task
semantics, job identity, demand, release and mode-change rules, bounded
subtraction contract, and certificate/replay carriers are owned by QSpec
FR-419, FR-415, FR-418, FR-331 and FR-364. QSL owns lowering those checked
inputs into EN-6's timed subject, searching it, rebuilding it for replay, and
settling its outcome. EN-6 SHALL use ordinary unit-rate clocks and the
QSpec-defined bounded-subtraction/diagonal construction; it SHALL NOT add
stopwatch rates (ADR-026 RT-8).

The engine SHALL decide the exact fixed-work class admitted by the current
FR-418 reader, using its independently checked finite region/product
certificate. Products outside that exact gate SHALL remain admitted for
sound bounded exploration. The engine SHALL return a concrete refutation
only when FR-237 accepts the deadline-miss prefix and QSpec FR-364's checked
time-divergent continuation. If bounded exploration cannot establish either
a complete proof or such a refutation, it SHALL return `incomplete` with replayable
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
- When S3 lowers checked release declarations and a task set into the EN-6
  task-derived subject, S3 SHALL consume the QSpec FR-419 product contract and
  preserve source state, QSpec task/job identities, and derived-step
  references in the subject and counterexample. QSL SHALL NOT define competing
  task identity, arrival, demand, service, budget-change, subtraction,
  certificate, or replay contracts.
- EN-6 SHALL implement the QSpec FR-415 bounded-subtraction/diagonal
  transition semantics using unit-rate clocks, then apply QSpec FR-418's
  finite-product certificate gate. The QSL-specific obligations are to build
  the subject accepted by that reader, pass it through the existing EN-6
  search and provide the same subject to FR-237 for replay. QSL SHALL NOT add
  stopwatch rates or infer support from the ordinary aLU model-subject path.
- The exact fixed-work class is the class admitted by the current QSpec
  FR-418 reader. An accepted complete certificate with no deadline miss
  settles `Holds`; a concrete miss settles `Violated` only after FR-237
  accepts QSpec FR-364's deadline-miss prefix and time-divergent continuation.
  Queue-cap overflow alone SHALL NOT settle a miss.
- Inputs outside the current FR-418 exact gate, including variable/interval
  demand and mixed-criticality budget changes, SHALL remain admitted and use
  sound bounded exploration. The run SHALL settle `Holds` only when the
  checked certificate covers the complete admitted task-product state space;
  it SHALL settle `Violated` only after replay verifies the concrete miss and
  continuation. Otherwise the provider SHALL return `Stopped` with QSpec
  FR-331 `task_search` evidence, which FR-235 settles as incomplete. QSL SHALL
  use FR-331's and FR-364's readers and wire contracts rather than defining
  task-frontier or derived-continuation schemas of its own.
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
| FR-249-AC-3 | The lowered EN-6 subject preserves QSpec FR-419's task-product state and FR-364's derived-step identity across a preemption/resumption sequence; FR-237 replays the resulting existing `ModelTrace` arm against the rebuilt product. | Test |
| FR-249-AC-4 | The lowered subject and replay preserve the QSpec FR-419/FR-364 mode-change transition identity and captured job data. If the current FR-418 reader does not accept the mixed-criticality product as exact, a bounded run cannot settle `Holds` and settles incomplete with FR-331 `task_search` evidence when its limit is reached. | Test |
| FR-249-AC-5 | For the periodic-source overload task, whose model repeats the release transition every `T=1` (`W=2`, `D=1`), under each admitted `fixed_priority` and `edf` policy, a refutation requires a concrete missed-job prefix followed by QSpec FR-364's replay-checked `queue_growth` continuation with positive elapsed time. Queue-cap overflow or a finite queue loop without the queue-growth identity induction does not refute. A task-search stop with an unexplored frontier replays as incomplete under FR-331. | Test |
| FR-249-AC-6 | Two task releases bound to one operation are recorded in source order; a duplicate release of one task on that operation refuses `conflicting_declaration`/`duplicate-declaration`; unresolved task set, task or operation names refuse `missing_declaration`/`missing-name`; an ambiguous inherited operation refuses `ambiguous_declaration`/`ambiguous-name`; a `release` on a model without `time dense` is admitted by checking and refused only when lowering is requested. | Test |

## Dependencies

- ADR-026 §12 RT-8 and §8 EZ-3 to EZ-5.
- [FR-230](FR-230-check-time-declarations-clocks-and-clock-constraints.md),
  [FR-237](FR-237-replay-a-timed-counterexample.md),
  [FR-238](FR-238-represent-zones-as-difference-bound-matrices.md),
  [FR-239](FR-239-check-a-timed-claim-by-symbolic-zone-search.md),
  [FR-246](FR-246-check-task-sets-and-schedulability-claims.md).

## References

- **Scope boundary.** QSpec FR-419 owns task-set, task-automaton, demand,
  activation and mode-change semantics; FR-415 owns the bounded-subtraction
  and diagonal proof contract; FR-418 owns certificate schema and exact-class
  admission; FR-331 owns incomplete task-search evidence; FR-364 owns the
  derived-step and continuation witness schema/replay rules. This QSL
  requirement defines only QSL lowering, EN-6 integration and outcome
  settlement, and does not amend those QSpec semantics or wire contracts.
- E. Fersman, P. Krčál, P. Pettersson and W. Yi, “Task automata:
  Schedulability, decidability and undecidability” (Information and
  Computation 205(8), 2007), Theorems 2 and 5. Fixed computation times admit
  decidable schedulability analysis;
  interval execution with completion feedback and preemption does not admit
  that exact result. The latter is the reason bounded search may return
  unknown, not a refusal.

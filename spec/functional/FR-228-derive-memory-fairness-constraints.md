---
id: FR-228
title: "Derive flush fairness under tso and visibility fairness under ra"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-025
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-025
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-018
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-027
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-212
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-436
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-437
    type: depends_on
---
# FR-228: Derive flush fairness under tso and visibility fairness under ra

## Description

Every infinite-trace clause over a subject with a `tso` `parallel` SHALL
carry QSpec FR-436's flush fairness, `fair weak each flush(b)` per thread
(ADR-025 MF-1), and every infinite-trace clause over a subject with an `ra`
`parallel` SHALL carry QSpec FR-437's visibility fairness (ADR-025 MF-2),
each with a `memory` origin, beside the authored and scheduler constraints
(FR-212). FR-126's fairness filter and canonical loop SHALL read them as
weak constraints (ADR-025 MF-3 to MF-5).

## Use case

A verification operator checks that a store is eventually visible. Real
hardware drains store buffers and real C11 implementations eventually show
new values, so a counterexample in which a buffer is never flushed would
report the model, not the protocol. The memory constraints keep such
behaviours out, and they are listed in every result so the premise stays
visible.

## Inputs

- An infinite-trace clause over a protocol subject with its fairness set
  (FR-212) and the resolved models (FR-219).

## Outputs

- The memory constraints in the clause's fairness set, each with origin
  `memory`: `fair weak each flush(b)` per thread under `tso`; one
  visibility constraint per (thread, location) under `ra`.

## Behavior

- The checker SHALL add `fair weak each flush(b)` for each thread `b` of
  each `tso` `parallel`, and one visibility constraint per (thread,
  location) the thread loads for each `ra` `parallel`, each with origin
  `memory`.
- FR-126's fairness filter SHALL accept an SCC or lasso only when it meets
  each memory constraint as QSpec FR-436 and FR-437 define it.
- The memory constraints SHALL be part of the clause's fairness set, its
  obligation identity, every counterexample and replay (FR-227). ADR-018
  CX-5's walk SHALL take each as a weak obligation.
- The memory constraints SHALL change liveness verdicts only: a safety
  item's verdict SHALL be the same with and without them.
- An authored constraint SHALL name one of FR-212's targets, whose classes
  hold no memory step.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-228-AC-1 | `eventually holds(c1.r <= 1)` under infinite-trace over `SB` resolved to `tso` carries `fair weak each flush(left)` and `fair weak each flush(right)` with origin `memory` beside the three `scheduler` constraints; resolved to `ra` it carries one visibility constraint per (thread, loaded location) with origin `memory`; resolved to `sc` it carries no memory constraint. | Test (TC-673) |
| FR-228-AC-2 | A weak `parallel` with branch `w`, one `relaxed` store `x := 1`, and branch `r`, a `repeat` with no maximum whose body is one `relaxed` load of `x` into `c.r` and whose guard is that load's binder reading 0, so that `x` is shared, under `tso`: `eventually holds(c.r = 1)` settles `proved`. The lasso whose stem is `fork`, `w`'s store, and whose loop is `r`'s load reading 0, with `w`'s buffer `[x := 1]` throughout, meets every scheduler constraint, since `w` has no enabled step and `r` steps on every edge, and is rejected by the fairness filter only under `fair weak each flush(w)`; with the memory constraints ignored the claim settles `refuted` with that lasso. | Test (TC-673) |
| FR-228-AC-3 | The fairness filter, given an SCC on which thread `b` loads `ℓ` only from a message that is not mo-last, rejects it under visibility fairness; given one that also holds a load by `b` of `ℓ`'s mo-last message, it passes it. | Test (TC-673) |
| FR-228-AC-4 | The TP-1 claim of ADR-025 §10 settles the same verdicts under `tso` and `ra` with the memory constraints present as when the fairness filter ignores them. A `fair weak` constraint naming `flush` refuses `missing_declaration`/`missing-name`. | Test (TC-673) |

## Dependencies

- ADR-025 §5 MF-1 to MF-5 (as amended by ADR-027); ADR-018 §4 FA-1 to
  FA-6, CX-5; ADR-027 PA-1 to PA-3, SE-1 (h).
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (fairness filter), FR-212 (fairness set and scheduler constraints),
  FR-221, FR-222.
- QSpec FR-436 and FR-437 own memory fairness and its `memory` origin.

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-436, FR-437 (Linear STD-138).

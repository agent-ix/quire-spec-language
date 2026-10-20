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
  - target: ix://agent-ix/quire-specification/FR-365
    type: depends_on
---
# FR-228: Derive flush fairness under tso and visibility fairness under ra

## Description

Every infinite-trace clause over a subject with a `tso` `parallel` SHALL
carry QSpec FR-436's flush fairness, `fair weak each flush(b)` per thread
(ADR-025 MF-1), and every infinite-trace clause over a subject with an `ra`
`parallel` SHALL carry QSpec FR-437's visibility fairness (ADR-025 MF-2),
each with a `memory` origin, beside the authored and scheduler constraints
(FR-212). FR-126's fairness filter and canonical loop SHALL read flush fairness
as weak constraints and visibility fairness as ADR-025 MF-2's per-pair edge
Streett condition (ADR-025 MF-3 to MF-5).

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
  one visibility fairness member under `ra`, of kind `strong`, whose
  (thread, location) pairs are derived from the resolved program.

## Behavior

- The checker SHALL add `fair weak each flush(b)` for each thread `b` of
  each `tso` `parallel`, and, for the item, one visibility fairness member of kind `strong` with
  target `{"type":"visibility","model":"ra"}` when it has an `ra` `parallel`,
  each with origin `memory`; the member's (thread, location) pairs are
  derived from the resolved program, one per location a thread loads.
- FR-126's fairness filter SHALL accept a lasso only when it meets each
  memory constraint as QSpec FR-436 and FR-437 define it.
- For visibility fairness on an SCC, the filter SHALL remove the edges on
  which a thread `b` loads a location `ℓ`, for each pair (`b`, `ℓ`) whose
  edges in the SCC include a load of `ℓ` by `b` but none from `ℓ`'s mo-last
  message (ADR-025 MF-2).
- The filter SHALL then decompose what remains into SCCs and apply the same
  rules to each.
- The filter SHALL apply the removals of every failing strong constraint (the
  states that enable it) and of every failing visibility pair in one
  refinement step, so the first passing component is the same under every
  order (ADR-025 MF-2, QSpec FR-365).
- The filter SHALL accept a component only when, for every pair it still
  loads, it holds an edge on which `b` loads `ℓ` from `ℓ`'s mo-last message.
  A component is its states with the edges the decomposition kept.
- The memory constraints SHALL be part of the clause's fairness set, its
  obligation identity, every counterexample and replay (FR-227). ADR-018
  CX-5's walk SHALL take each flush constraint as a weak obligation, and each
  visibility pair the passing component still loads as ADR-025 MF-2 states,
  using only the edges the decomposition kept.
- The certificate of a `proved` item SHALL carry `UnfairVisibility{constraints,
  sub}` for each removal (ADR-018 PC-4, FR-339); a `refuted` claim carries no
  certificate.
- The memory constraints SHALL change liveness verdicts only: a safety
  item's verdict SHALL be the same with and without them.
- An authored constraint SHALL name one of FR-212's targets, whose classes
  hold no memory step.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-228-AC-1 | `eventually holds(c1.r <= 1)` under infinite-trace over `SB` resolved to `tso` carries `fair weak each flush(left)` and `fair weak each flush(right)` with origin `memory` beside the three `scheduler` constraints; resolved to `ra` it carries one visibility member of kind `strong` with origin `memory`, whose pairs are the (thread, loaded location) pairs of the resolved program; resolved to `sc` it carries no memory constraint. | Test (TC-673) |
| FR-228-AC-2 | A weak `parallel` with branch `w`, one `relaxed` store `x := 1`, and branch `r`, a `repeat` with no maximum whose body is one `relaxed` load of `x` into `c.r` and whose guard is that load's binder reading 0, so that `x` is shared, under `tso`: `eventually holds(c.r = 1)` settles `proved`. The lasso whose stem is `fork`, `w`'s store, and whose loop is `r`'s load reading 0, with `w`'s buffer `[x := 1]` throughout, meets every scheduler constraint, since `w` has no enabled step and `r` steps on every edge, and is rejected by the fairness filter only under `fair weak each flush(w)`; with the memory constraints ignored the claim settles `refuted` with that lasso. | Test (TC-673) |
| FR-228-AC-3 | The fairness filter, given an SCC every cycle of which takes a load by thread `b` of `ℓ` from a message that is not mo-last, and none of whose edges loads `ℓ`'s mo-last message by `b`, rejects it under visibility fairness; given one that also holds a load by `b` of `ℓ`'s mo-last message, it passes it. | Test (TC-673) |
| FR-228-AC-4 | The TP-1 claim of ADR-025 §10 settles the same verdicts under `tso` and `ra` with the memory constraints present as when the fairness filter ignores them. A `fair weak` constraint naming `flush` refuses `missing_declaration`/`missing-name`. | Test (TC-673) |
| FR-228-AC-5 | A synthesized accepting product SCC on states `p`, `q`, `r`, passed straight to the visibility filter as AC-3's SCCs are, with the edges `p → q`, `q → p` and `r → q` (thread `a`, which loads nothing) and `q → r` (thread `b` loads `ℓ` from a message that is not mo-last), `p` the only accepting state: the filter removes `q → r`, decomposes `{p, q, r}` into `{r}` then `{p, q}`, passes `{p, q}`, and the lasso `p q p q …` meets the lasso form of visibility fairness. (A protocol form of this vector needs `scheduling adversarial;`, since the default scheduler constraint for `b` would reject `{p, q}` unless `b` is disabled at `p`.) | Test (TC-673) |
| FR-228-AC-6 | Given an accepting SCC on `p`, `q` with the edges `p → p` (thread `b` loads `ℓ` from a message that is not mo-last), `p → q` and `q → p` (thread `a`, loading nothing), `p` accepting, the filter removes `p → p` and passes `{p, q}` with the edges `p → q`, `q → p` only; the loop CX-5 builds is `p q`, never `p p`, and replay accepts it. | Test (TC-673) |
| FR-228-AC-7 | Given an accepting SCC on `p`, `q`, `r`, `s` with the edges `p → q` (thread `c` loads `ℓ₂` from `ℓ₂`'s mo-last message), `q → p`, `r → s` (thread `a`), `s → q` (thread `a`), `q → r` (thread `b` loads `ℓ₁` from a message that is not mo-last) and `s → r` (thread `c` loads `ℓ₂` from a message that is not mo-last), `p` accepting, the filter removes `q → r` for the pair (`b`, `ℓ₁`) at the first level (it is the only pair that fails in the whole SCC), finds `{r, s}` holding a stale load of `ℓ₂` by `c` and no fresh one, removes `s → r` at the second level, and passes `{p, q}`. | Test (TC-673) |
| FR-228-AC-8 | Given a synthesized accepting SCC on states 0 to 4, with a strong constraint `s` enabled only at state 0 and taken by no edge, a visibility pair with the stale load edge `1 → 2` and no fresh load, a second pair with the stale load edge `3 → 3` and the fresh load edge `3 → 4`, and the edges `0 → 1`, `1 → 0`, `1 → 2`, `2 → 3`, `3 → 1`, `3 → 3`, `3 → 4` and `4 → 3`, state 3 accepting, the filter refines in one step that removes state 0 and the edge `1 → 2`, passes `{3, 4}`, and the loop CX-5 builds is `3 → 4 → 3`. | Test (TC-673) |
| FR-228-AC-9 | Given an SCC searched from state 0 (discovery order 0, 1, 2), with states 1 and 2 accepting, and the edges `e0` `1 → 1`, `e1` `2 → 2`, `e2` `0 → 1`, `e3` `0 → 2`, `e4` `2 → 0`, `e5` `2 → 1`, `e6` `1 → 2` and `e7` `2 → 2` (a second, stale self-loop), and two visibility pairs with no fresh load, pair A with stale load edges `e5`, `e6`, `e7` and pair B with stale load edges `e4`, `e5`, `e6`, the one refinement step removes the edges of both pairs, components are ranked by their earliest-discovered state (FS-6, FR-181 breadth-first order), and the first passing component is `{1}`. | Test (TC-673) |

## Dependencies

- ADR-025 §5 MF-1 to MF-5 (as amended by ADR-027); ADR-018 §4 FA-1 to
  FA-6, CX-5; ADR-027 PA-1 to PA-3, SE-1 (h).
- [FR-126](FR-126-check-a-temporal-clause-over-every-behaviour-of-a-model.md)
  (fairness filter), FR-212 (fairness set and scheduler constraints),
  FR-221, FR-222, FR-339 (the component certificate); QSpec FR-365 (the SCC search and its refinement order).
- QSpec FR-436 and FR-437 own memory fairness and its `memory` origin.

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-436, FR-437 (Linear STD-138).

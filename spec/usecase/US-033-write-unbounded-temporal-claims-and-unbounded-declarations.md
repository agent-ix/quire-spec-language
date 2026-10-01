---
id: US-033
title: "Write unbounded temporal claims and unbounded declarations"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-325
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-326
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-327
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-328
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-329
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-330
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-331
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-332
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-333
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-334
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-335
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-336
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-033: Write unbounded temporal claims and unbounded declarations

## Story

**As a** specification author writing QSL claims about a system
**I want** to write `eventually p` and `always (fail implies eventually
always[0,10] healthy)` with no artificial horizon, and to declare `Set<Account>`
or a population with no maximum, and have each one mean exactly what it says
**So that** I state the property I care about, evaluate it on the traces and
values I have, and learn from each settlement what was actually decided:
proved, refuted with a counterexample I can replay, evidence for one input,
or unsupported with the reason and what would help.

## Context

ADR-014 gives QSL one rule for absent bounds and one infinite-trace profile,
`quire.temporal.infinite-trace/v1`. ADR-018 lets interval operators nest
inside infinite-trace formulas and makes the trace evaluator the semantics
that every engine answers to. US-015 covers checking such a claim over every
behaviour of a model; this story covers writing it, evaluating it on a given
trace, replaying its counterexample, and settling claims over unbounded
declarations.

## Acceptance Examples (Illustrative)

### US-033-EX-1: An unbounded formula is admitted only where it means something

- **Given** a unit that selects `quire.temporal.infinite-trace/v1` and a
  clause `eventually holds(c.value = 3)`.
- **When** the author compiles it, and then compiles the same clause in a
  unit that selects the event-position false-extension profile.
- **Then** the first checks with an unbounded `eventually` and no horizon;
  the second refuses at the operator.

### US-033-EX-2: A finite trace never proves liveness

- **Given** the infinite-trace clause above and a three-position observed
  trace on which `c.value` reaches 3.
- **When** the author runs the clause on the trace.
- **Then** the run reports pending, `inconclusive`, never `proved`.

### US-033-EX-3: A lasso is decided exactly and its refutation replays

- **Given** a lasso whose loop never reaches `c.value = 3`.
- **When** the author runs the clause on it.
- **Then** the run reports a violation at position 0, and replaying the
  counterexample from source settles `reproduced-with-evaluated-witness`.

### US-033-EX-4: An unbounded declaration settles honestly

- **Given** a claim over a parameter `s: Set<Account>` with no bound.
- **When** the author requests it with no backend registered, and again
  with only a bounded-mode backend registered.
- **Then** the first settles `unsupported` with a warning naming the
  capability; the second settles `requires-bound`, and a request with
  `Cardinality{maximum: 8}` for `s` becomes its own bounded item.

## Priority and Risk (Informative)

Priority: High. Unbounded temporal operators and unbounded collections are
the forms authors reach for first; an artificial horizon or a default
maximum would change the meaning of what they wrote.

## Traceability (Informative)

- [FR-325](../functional/FR-325-parse-temporal-operators-with-an-optional-interval.md)
- [FR-326](../functional/FR-326-admit-temporal-operators-by-the-unit-s-temporal-profile.md)
- [FR-327](../functional/FR-327-evaluate-a-temporal-clause-over-a-finite-trace.md)
- [FR-328](../functional/FR-328-evaluate-an-infinite-trace-clause-over-a-finite-prefix.md)
- [FR-329](../functional/FR-329-evaluate-an-infinite-trace-clause-exactly-over-a-lasso.md)
- [FR-330](../functional/FR-330-run-a-temporal-clause-through-the-spine.md)
- [FR-331](../functional/FR-331-replay-a-temporal-counterexample-over-an-observed-trace.md)
- [FR-332](../functional/FR-332-settle-an-infinite-trace-item-through-negotiation.md)
- [FR-333](../functional/FR-333-read-an-optional-collection-bound-and-population-maximum.md)
- [FR-334](../functional/FR-334-key-collection-types-by-the-root-definitions-identity-preimage.md)
- [FR-335](../functional/FR-335-settle-a-claim-over-an-unbounded-declaration.md)
- [FR-336](../functional/FR-336-bound-a-model-subject-by-its-universes.md)

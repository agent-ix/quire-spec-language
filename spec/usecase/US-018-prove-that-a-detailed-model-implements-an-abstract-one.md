---
id: US-018
title: "Prove that a detailed model implements an abstract one"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-135
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-136
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-137
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-138
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-139
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-140
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-141
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-142
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-143
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-144
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-145
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-146
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-147
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-148
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-018: Prove that a detailed model implements an abstract one

## Story

**As a** specification author who has a small abstract QSL model and a more
detailed QSL model of the same system
**I want** to write one refinement declaration that maps the detailed
model's state and steps onto the abstract model's, and have QSL check that
every behaviour of the detailed model is a behaviour of the abstract one up
to stuttering, with the abstract model's fairness where I ask for it
**So that** I verify the design in layers, I get a counterexample that
replays when the detailed model does something the abstract model forbids,
and I get a stated reason when the check cannot decide.

## Context

ADR-020 designs the check. ADR-018 gives every-behaviour verdicts over one
model subject (US-015). A refinement relates two model subjects through an
authored mapping and step map, decided step by step by one function on the
same explicit-state product. Auxiliary history fields and hidden abstract
fields stay inside the refinement declaration, so the detailed model and
its other verdicts are unchanged. ADR-027 extends the concrete side to a
protocol subject.

## Acceptance Examples (Illustrative)

### US-018-EX-1: A compare-and-set counter refines a counter

- **Given** ADR-020 §8's abstract `Counter` and its compare-and-set
  implementation, with every concrete operation mapped by an explicit row.
- **When** the author requests the refinement.
- **Then** it settles `proved`, basis `closed-scope`, over the concrete
  subject.

### US-018-EX-2: A lost update is refuted with a replayable prefix

- **Given** the same refinement over the implementation with the
  compare-and-set guard dropped.
- **When** the author requests it.
- **Then** it settles `refuted` at the second commit, which is mapped to
  `inc` but does not increment, and replaying the prefix reproduces the
  failure.

### US-018-EX-3: Abstract fairness turns spinning into a refutation

- **Given** an implementation that can loop on a read-only step forever
  while the abstract counter can still move.
- **When** the author requests the refinement with an `ensure` row for the
  abstract increment, and again without it.
- **Then** it settles `refuted` with failure `Divergence` with the row, and
  `proved` as a safety refinement without it.

### US-018-EX-4: An abstract value built from a concrete population

- **Given** ADR-020 §8's ring buffer, whose abstract queue is built from the
  slots population.
- **When** the author requests the refinement.
- **Then** it settles `proved`, and a broken `take` settles `refuted`.

### US-018-EX-5: A concrete model refines an abstract protocol

- **Given** an abstract protocol that increments twice and then finishes,
  and the compare-and-set implementation with each commit mapped to the
  increment.
- **When** the author requests the refinement.
- **Then** it settles `proved`, and an abstract protocol that increments
  only once is refuted at the second commit.

### US-018-EX-6: A missing step row is a compile error

- **Given** a refinement declaration with no row for one concrete operation.
- **When** the unit is checked.
- **Then** S3 refuses it, naming the operation.

## Priority and Risk (Informative)

Priority: High. Layered refinement is how TLA+ users verify designs, and
step labels let QSL refute bugs that a state-only mapping reads as
stuttering.

## Traceability (Informative)

- [FR-135](../functional/FR-135-check-a-refinement-declaration-s-subjects-and-state-mapping.md)
- [FR-136](../functional/FR-136-check-a-refinement-s-step-rows.md)
- [FR-137](../functional/FR-137-check-a-refinement-s-fairness-rows.md)
- [FR-138](../functional/FR-138-check-and-compute-history-fields.md)
- [FR-139](../functional/FR-139-type-and-evaluate-population-valued-expressions.md)
- [FR-140](../functional/FR-140-evaluate-the-refinement-mapping-at-a-state.md)
- [FR-141](../functional/FR-141-decide-one-concrete-step-against-the-abstract-model.md)
- [FR-142](../functional/FR-142-check-a-refinement-s-safety-half-on-the-explicit-state-product.md)
- [FR-143](../functional/FR-143-check-a-refinement-s-liveness-half-under-abstract-fairness.md)
- [FR-144](../functional/FR-144-request-and-settle-a-refinement-item.md)
- [FR-145](../functional/FR-145-replay-a-refinement-counterexample.md)
- [FR-146](../functional/FR-146-write-per-step-simulation-records-for-a-refinement.md)
- [FR-147](../functional/FR-147-check-a-refinement-whose-abstract-side-is-a-protocol.md)
- [FR-148](../functional/FR-148-decide-steps-against-an-abstract-protocol.md)

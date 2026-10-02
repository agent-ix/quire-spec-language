---
id: US-019
title: "Check a larger model with state-space reduction"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-150
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-151
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-152
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-153
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-154
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-156
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-157
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-158
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-159
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-160
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-161
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-162
    type: exercises
  - target: ix://agent-ix/quire-spec-language/US-015
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-019: Check a larger model with state-space reduction

## Story

**As a** verification operator whose model has several interchangeable
objects, or operations that touch disjoint state, so that its every-behaviour
check (US-015) runs out of memory
**I want** to mark a population `symmetric`, scope an operation's frame to
its receiver, and select symmetry, partial-order reduction or a state
constraint on the request
**So that** the check explores far fewer states, every reduction I select is
checked against my model and my claim before it runs, a reduced proof says
it is reduced, a counterexample is an ordinary model trace I can replay, and
a constrained search that found nothing never reads as a proof.

## Context

ADR-021 designs the reductions on ADR-018's explicit-state engine. Symmetry
stores one representative per orbit of interchangeable keys; partial-order
reduction expands a subset of the enabled transitions where they are
independent; a state constraint cuts the search at states outside a
predicate. The model's half of symmetry, that no clause observes identity,
is checked when the unit is checked. The request's half, which keys are
interchangeable and whether the initial states respect that, is checked
before the first expansion. Partial-order reduction rests on read and write
footprints that the engine enforces on every step.

## Acceptance Examples (Illustrative)

### US-019-EX-1: Symmetry shrinks a liveness proof

- **Given** ADR-021 §7.1's ConfigVersion unit with `config_history`
  declared `symmetric`, universe `{a, b, c}`, one key class `[a, b, c]`,
  and `always eventually holds(c.versionNumber = 2)` under
  `fair weak each attemptUpdate`.
- **When** the operator requests the claim with symmetry selected.
- **Then** it settles `proved`, basis `closed-scope`, proof basis `reduced`
  naming the symmetry group, over 30 product states where the unreduced
  check explores 135.

### US-019-EX-2: An identity-observing clause is refused while authoring

- **Given** the same unit with a post clause that folds over a set of
  `ConfigVersion` references.
- **When** the operator checks the unit.
- **Then** S3 refuses it at the `fold`, naming `config_history`.

### US-019-EX-3: A form the reduction does not preserve is named

- **Given** a bounded MLTL claim and partial-order reduction selected.
- **When** the operator requests it.
- **Then** it settles `inconclusive`, `ReductionNotPreserving`, before any
  state is explored.

### US-019-EX-4: A counterexample from a reduced search replays

- **Given** a symmetry-reduced liveness check that finds a fair lasso.
- **When** the operator replays the refutation.
- **Then** the lasso is a concrete model trace with no permutation in it,
  and it replays through the model like any other counterexample.

### US-019-EX-5: A constrained search that finds nothing is not a proof

- **Given** a state constraint that cuts the search before the violating
  states.
- **When** the operator requests a safety claim.
- **Then** it settles `inconclusive`, `ConstraintReached`, with the number
  of boundary states.

## Priority and Risk (Informative)

Priority: High. Exhaustive checking of models with a handful of objects
exhausts memory without reduction. The risk is a reduction that hides a
violation; every reduction here is checked or enforced rather than trusted.

## Traceability (Informative)

- [FR-150](../functional/FR-150-check-the-symmetric-population-annotation.md)
- [FR-151](../functional/FR-151-admit-a-request-s-symmetry-declarations.md)
- [FR-152](../functional/FR-152-canonicalise-a-model-state-by-sorting-its-symmetry-classes.md)
- [FR-153](../functional/FR-153-decide-each-fairness-on-the-annotated-quotient.md)
- [FR-154](../functional/FR-154-scope-a-modifies-entry-to-the-receiver.md)
- [FR-155](../functional/FR-155-derive-and-enforce-read-and-write-footprints.md)
- [FR-156](../functional/FR-156-expand-ample-sets-with-the-breadth-first-proviso.md)
- [FR-157](../functional/FR-157-make-fairness-enabling-writes-visible.md)
- [FR-158](../functional/FR-158-cut-the-search-with-a-state-constraint.md)
- [FR-159](../functional/FR-159-pre-check-selected-reductions-against-the-preservation-table.md)
- [FR-160](../functional/FR-160-settle-reduced-verdicts-and-reduction-causes.md)
- [FR-161](../functional/FR-161-concretise-a-reduced-counterexample.md)
- [FR-162](../functional/FR-162-offer-reductions-through-transition-system-hooks.md)

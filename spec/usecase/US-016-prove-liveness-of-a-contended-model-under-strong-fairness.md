---
id: US-016
title: "Prove liveness of a contended model under strong fairness"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-129
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-130
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-131
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-134
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-016: Prove liveness of a contended model under strong fairness

## Story

**As a** verification operator modelling a lock, a retry loop or an
election, where an operation is enabled only now and then
**I want** to state that such an operation is strongly fair, so that a
behaviour in which it is enabled infinitely often but never taken does not
count
**So that** the model checker proves the liveness claim the system relies
on, or returns a lasso that is fair under the strong premise and replays.

## Context

ADR-019 adds strong fairness beside ADR-018's weak fairness. Weak fairness
admits a behaviour in which an operation is enabled and disabled
infinitely often and never taken, which is the shape of contention. The
explicit-state model checker (ADR-018 EN-1) decides strong fairness by
refining the strongly connected components of its product graph.

## Acceptance Examples (Illustrative)

### US-016-EX-1: Strong fairness proves what weak fairness cannot

- **Given** ADR-019 §6's mutex with processes 1 and 2 and the claim
  `always eventually holds(m.owner = 1)`.
- **When** the operator requests it under weak `each` fairness of
  `acquire`.
- **Then** it settles `refuted` with the lasso in which process 2 acquires
  and releases forever.
- **When** the operator requests it under strong `each` fairness of
  `acquire`.
- **Then** it settles `proved` over that subject.

### US-016-EX-2: The granularity still matters

- **Given** the same claim under strong fairness of `acquire` with no
  granularity, which reads as `whole`.
- **When** the operator requests it.
- **Then** it settles `refuted` with the same lasso, since process 2
  acquiring satisfies fairness for the operation as a whole.

### US-016-EX-3: Replay decides fairness for itself

- **Given** the weak-fairness lasso of EX-1.
- **When** an auditor replays it against the strong `each` claim.
- **Then** replay refuses it as unfair, because process 1's acquire is
  enabled in the loop and never taken.

## Priority and Risk (Informative)

Priority: High. Liveness of contended and retrying systems is a common
reason to reach for TLA+'s `SF`; without it those claims cannot be proved.

## Traceability (Informative)

- [FR-129](../functional/FR-129-check-strong-fairness-constraints.md)
- [FR-130](../functional/FR-130-decide-strong-fairness-on-the-explicit-state-product.md)
- [FR-131](../functional/FR-131-replay-checks-strong-fairness-on-a-model-counterexample.md)
- [FR-134](../functional/FR-134-advertise-the-fairness-kinds-en-1-decides.md)

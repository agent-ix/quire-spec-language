---
id: US-023
title: "Prove a probabilistic property exactly, under a workload or over every scheduler"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-195
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-196
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-197
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-198
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-199
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-200
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-201
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-202
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-203
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-204
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-023: Prove a probabilistic property exactly, under a workload or over every scheduler

## Story

**As a** verification operator with a probabilistic QSL model
**I want** a probabilistic claim proved or refuted exactly, under a stated
workload, over every scheduler, or over every fair scheduler, including
claims sampling cannot settle (rare events, thresholds near the true value,
long-run availability, expected times, deadlines of timed models)
**So that** a proof carries a certificate a small checker verifies in exact
rationals, a refutation carries an adversary and paths I can replay, and no
floating-point number decides a verdict.

## Context

ADR-028 designs the exact engine EN-5 over ADR-024's probabilistic models.
EN-1's product, monitors and SCC decomposition and FR-187's step
probabilities are what it builds on. The certificate checker sits in the
qualified core (ADR-029 RU-2), so an engine proof counts only once the
checker accepts it.

## Acceptance Examples (Illustrative)

### US-023-EX-1: A latency bound is proved exactly

- **Given** ADR-024 §7.2's `P95` claim with no confidence parameters.
- **When** the operator requests it with exact evidence.
- **Then** it settles `proved`, `ExactValue{24233/25000}`, after the checker
  accepts its certificate.

### US-023-EX-2: An adversary refutes a delivery bound

- **Given** ADR-028 §15.4's `Deliver` claim over every scheduler.
- **When** the operator requests it.
- **Then** it settles `refuted` with a witness scheduler that always picks
  the worse link and a path of probability `1/25` that replays.

### US-023-EX-3: Termination holds against a fair adversary

- **Given** ADR-028 §15.6's `Coin` model.
- **When** the operator requests termination over every scheduler, and then
  over every scheduler that is strongly fair to `flip`.
- **Then** the first is `refuted`, and the second is `proved` with value 1.

### US-023-EX-4: A timed deadline is proved at equality

- **Given** ADR-028 §15.5's `Deadline` claim.
- **When** the operator requests it.
- **Then** it settles `proved`, `ExactValue{99/100}`, a value equal to the
  threshold that no interval method decides.

## Priority and Risk (Informative)

Priority: High. Every-scheduler and fair-scheduler claims are what
probabilistic protocols need, and exact results remove the sample cost of
rare events. Risk: the product's size bounds what EN-5 can decide; every
limit is a caller-set budget.

## Traceability (Informative)

- [FR-195](../functional/FR-195-check-exact-only-forms-and-route-exact-evidence.md)
- [FR-196](../functional/FR-196-build-the-probabilistic-product.md)
- [FR-197](../functional/FR-197-decide-finite-horizon-forms-by-exact-backward-induction.md)
- [FR-198](../functional/FR-198-decide-unbounded-reachability-and-expected-rewards.md)
- [FR-199](../functional/FR-199-decide-long-run-fractions-by-bottom-components.md)
- [FR-200](../functional/FR-200-decide-every-scheduler-claims-over-fair-schedulers.md)
- [FR-201](../functional/FR-201-check-a-probability-certificate.md)
- [FR-202](../functional/FR-202-replay-a-probabilistic-witness.md)
- [FR-203](../functional/FR-203-bound-an-exact-run-and-settle-its-verdict.md)
- [FR-204](../functional/FR-204-check-probabilistic-timed-automata-through-digital-clocks.md)

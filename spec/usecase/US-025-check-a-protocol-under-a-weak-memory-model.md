---
id: US-025
title: "Check a protocol under a weak memory model"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-219
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-220
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-221
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-222
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-224
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-225
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-226
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-227
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-228
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-229
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-025: Check a protocol under a weak memory model

## Story

**As a** verification operator whose protocol stands for Rust or C code
built on atomics, or for code that runs on x86
**I want** to state the memory model the code assumes on each `parallel`,
annotate each access with its ordering, and check the same protocol under
`sc`, `tso` or `ra` from the request, with data races reported
**So that** a claim that holds only under sequential consistency is refuted
with a counterexample that shows the store buffers or messages that break
it, a proof says which memory bound it reached, and I know which conditions
the code must meet for the proof to carry over to it.

## Context

ADR-025 designs weak memory models for `parallel` on ADR-027's protocol
transition system. TLA+ and PlusCal assume sequential consistency, so an
author who models x86 or C11 code writes the store buffer by hand. Here the
memory model is a parameter, and the explicit-state model checker explores
its operational semantics.

## Acceptance Examples (Illustrative)

### US-025-EX-1: Store buffering under three models

- **Given** ADR-025 §10's store-buffering protocol, source default `sc`,
  and the claim that the two loads never both read 0.
- **When** the operator requests it with no `memory` member, then with
  `tso`, then with `ra`.
- **Then** it settles `proved` under `sc`, and `refuted` under `tso` and
  `ra` with replayable counterexamples, each stating the memory bound 4
  as not reached.

### US-025-EX-2: Fences repair it

- **Given** the same protocol with a `seq_cst` fence between each store
  and load.
- **When** the operator requests it under `tso` and `ra`.
- **Then** both settle `proved`.

### US-025-EX-3: A data race is reported once

- **Given** ADR-025 §10's message-passing shape with non-atomic data and a
  `relaxed` flag, under `ra`.
- **When** the operator requests any claim.
- **Then** the request also carries the race-freedom item, which settles
  `refuted` with a race counterexample on the data location.

## Priority and Risk (Informative)

Priority: Medium. Weak memory is what links a protocol proof to concurrent
code built on atomics; it multiplies the state space, so examples stay
small.

## Traceability (Informative)

- [FR-219](../functional/FR-219-declare-and-select-the-memory-model-of-a-parallel.md)
- [FR-220](../functional/FR-220-check-access-orderings-fences-and-access-classification.md)
- [FR-221](../functional/FR-221-explore-the-x86-tso-memory-model.md)
- [FR-222](../functional/FR-222-explore-the-release-acquire-memory-model.md)
- [FR-223](../functional/FR-223-order-seq-cst-events-by-the-rc11-partial-sc-order.md)
- [FR-224](../functional/FR-224-derive-the-race-freedom-item-for-non-atomic-locations.md)
- [FR-225](../functional/FR-225-bound-the-memory-component-by-modelchecklimits-budgets.md)
- [FR-226](../functional/FR-226-state-the-code-link-preconditions-of-a-weak-memory-verdict.md)
- [FR-227](../functional/FR-227-carry-and-replay-a-weak-memory-counterexample.md)
- [FR-228](../functional/FR-228-derive-memory-fairness-constraints.md)
- [FR-229](../functional/FR-229-carry-the-memory-component-through-the-protocol-system.md)

## References

- Owning ticket: Linear QSL-372. QSpec half: QSpec FR-434 to
  FR-439 (Linear STD-138).

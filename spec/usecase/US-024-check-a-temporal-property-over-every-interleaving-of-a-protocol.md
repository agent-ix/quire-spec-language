---
id: US-024
title: "Check a temporal property over every interleaving of a protocol"
type: US
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-205
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-206
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-207
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-208
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-209
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-210
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-211
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-212
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-213
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-214
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-215
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-217
    type: exercises
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: exercises
  - target: ix://agent-ix/quire-spec-language/StR-001
    type: traces_to
---
# US-024: Check a temporal property over every interleaving of a protocol

## Story

**As a** verification operator with a QSL protocol whose `run` control forks
`parallel` branches, exchanges messages on channels, registers
compensations, spawns replicated roles or starts one instance per trigger
**I want** a temporal claim about that protocol, and its deadlock freedom,
checked over every interleaving its causal edges and its operations'
contracts allow, under a fair scheduler unless I ask for an adversarial one
**So that** I get `proved` when no interleaving violates the claim, a
counterexample I can replay that names each step and, for a stuck protocol,
each waiting thread and why it waits, and a stated reason, with the bound
used, when the check cannot decide.

## Context

ADR-027 designs the protocol transition system. ADR-018 gives the model
checker over a model subject; ADR-027 makes a protocol a model subject whose
steps are its attempts, events, sends, receives, forks, joins, compensation
attempts, role spawns and retirements, activations and memory steps. The
explicit-state model checker explores it through FR-101's engine.

## Acceptance Examples (Illustrative)

### US-024-EX-1: A stuck join is a deadlock with its cause

- **Given** ADR-027 §7's `Fill` protocol over a `Cell` at 0, with `TwoPre:
  self.v = 0`.
- **When** the operator requests any temporal claim over it.
- **Then** the request also carries the deadlock-freedom item, which
  settles `refuted` with the prefix `fork(Both)`, `attempt(A)`, and names
  `right` waiting at `B` on `TwoPre` and the root waiting at `join Both` on
  `right`; replaying the counterexample reproduces it.

### US-024-EX-2: The repaired protocol is proved deadlock-free

- **Given** the same protocol with `TwoPre: self.v <= 1`.
- **When** the operator requests the deadlock-freedom item.
- **Then** it settles `proved` over ten states.

### US-024-EX-3: A compensation runs when the failure event happens

- **Given** ADR-027 §7.1's `Pay` protocol.
- **When** the operator requests `eventually[0,2] holds(k.bal = 0)` at
  position 2 of the failure path.
- **Then** it holds there: the refund's compensation attempt is two counted
  steps after the failure, and `cend` and `finish` do not count.

### US-024-EX-4: Per-trigger activation states the bound it used

- **Given** a protocol with `activation on each` whose trigger is always
  available, and a request that leaves `max_live_instances` unset.
- **When** the operator requests a safety claim that no violation reaches.
- **Then** it settles `inconclusive`, cause `InstanceBoundReached`, and
  the result states that `max_live_instances` was 3 and was reached.

## Priority and Risk (Informative)

Priority: High. Concurrency over `parallel` is the reason to model a
protocol at all, and the refinement, state-space reduction and weak-memory
work builds on this system.

## Traceability (Informative)

- [FR-205](../functional/FR-205-define-the-protocol-subject-and-its-state-key.md)
- [FR-206](../functional/FR-206-take-the-protocol-step-kinds-with-folded-structural-moves.md)
- [FR-207](../functional/FR-207-run-compensation-templates-as-protocol-threads.md)
- [FR-208](../functional/FR-208-spawn-and-retire-replicated-role-instances.md)
- [FR-209](../functional/FR-209-start-a-protocol-instance-per-trigger-under-the-instance-budget.md)
- [FR-210](../functional/FR-210-admit-exactly-the-causal-interleavings.md)
- [FR-211](../functional/FR-211-declare-protocol-terminal-states-and-report-protocol-deadlocks.md)
- [FR-212](../functional/FR-212-derive-scheduler-fairness-with-an-adversarial-opt-out.md)
- [FR-213](../functional/FR-213-measure-protocol-intervals-in-counted-steps.md)
- [FR-214](../functional/FR-214-require-an-explicit-refinement-row-for-every-protocol-step-class.md)
- [FR-215](../functional/FR-215-implement-protocolsystem-as-a-transitionsystem.md)
- [FR-216](../functional/FR-216-give-every-protocol-step-a-static-footprint.md)
- [FR-217](../functional/FR-217-replay-a-protocol-counterexample.md)
- [FR-218](../functional/FR-218-check-every-protocol-control-construct-at-s3.md)

## References

- Owning ticket: Linear QSL-396. QSpec half: QSpec FR-425 to
  FR-433 and the causal interleavings of QSpec FR-052 (Linear STD-140).

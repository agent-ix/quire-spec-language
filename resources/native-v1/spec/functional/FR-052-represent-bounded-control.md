---
id: FR-052
title: "Represent bounded causal protocol control"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-006
    type: implements
  - target: ix://agent-ix/quire-specification/FR-050
    type: depends_on
---
## Description

When linking a protocol body, the protocol linker SHALL construct a finite causal graph for authored sequence, labeled choice, parallel branches, join, bounded iteration and termination without deriving causal edges from source order or timestamps.

## Inputs

Typed protocol nodes, authored control edges, finite iteration maxima, branch
join rules and termination declarations.

## Outputs

An acyclic finite control representation with explicit branch and progress edges,
or a located typed refusal.

## Behavior

Sequence SHALL add only its authored order. Parallel branches SHALL admit every
interleaving that preserves each branch's causal edges. A join SHALL name the
branches and completion rule it awaits. Iteration SHALL carry an authored finite
maximum and progress edge. The linker SHALL refuse missing joins, cyclic causal
dependencies, unbounded repetition and termination conditions that cannot be
reached within the declared finite graph.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-052-AC-1 | Split shipments S1 and S2 may interleave while each branch preserves its own order and both satisfy the authored join before completion. | Test (TC-052) |
| FR-052-AC-2 | Reordering equal-time events does not add a causal edge absent from the protocol. | Test (TC-052) |
| FR-052-AC-3 | A missing join input, causal cycle, zero-progress loop or absent finite iteration maximum refuses admission. | Test (TC-052) |
| FR-052-AC-4 | Changing a branch, join rule or iteration maximum changes protocol identity and preserves the original source span. | Test (TC-052) |

## Dependencies

- [FR-050](./FR-050-bind-protocol-instances.md).
- Agent A's shared grammar and predicate contracts.

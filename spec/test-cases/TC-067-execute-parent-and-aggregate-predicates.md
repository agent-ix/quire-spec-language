---
id: TC-067
title: "Execute parent and aggregate predicates"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-067: Execute parent and aggregate predicates

## Description

Integration, priority P1. Verifies FR-008-AC-1, FR-008-AC-2. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Use the actual source-derived Node model and native clauses for parent version ordering and forall over its ordered items sequence of Version scalars, comparing members with self.n. Execute healthy acyclic populations, a lower/equal violating parent, an aggregate member that violates, an empty aggregate and repeated healthy members. Establish successful checking and validation separately before asserting truth.

## Expected Results

Healthy parent/aggregate inputs are true and actual order/member violations false. Empty universal is true. Every completed result retains original authored/source/model/input bindings; no setup failure is interpreted as false.

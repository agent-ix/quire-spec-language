---
id: TC-130
title: "Evaluate positive-length reachability over finite cycles"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/FR-047, type: verifies }
---
# TC-130: Evaluate positive-length reachability over finite cycles

## Description

Verify deterministic positive-length reachability, cycle termination and
ordered edge semantics through the public admitted-artifact evaluator.

## Test Procedure

Evaluate isolated, direct, self-loop, two-node cycle, diamond, disconnected,
optional-absent, empty-sequence and duplicate-edge graphs. Use graph shapes for
which depth-first and breadth-first discovery would consume different work.
Independently count the expected storage-key expansions, active depth and edge
occurrences under NFR-009's recursive depth-first, charge-before-work order.
Reverse an authored sibling-edge sequence so a target moves before and after a
non-target subtree, then substitute a scalar, foreign or heterogeneous edge
field.

## Expected Results

An isolated object does not reach itself; a self-loop and returning cycle do.
Direct and longer paths return true and disconnected declared-complete graphs
return false. Each full storage identity is expanded at most once. Duplicate
and ordered edge occurrences consume their independent charges, and changing
sibling order changes usage exactly when the deterministic early target permits
it. The outcome is Boolean and exposes no path witness. Unsupported edge shapes
and foreign identities refuse rather than becoming false.

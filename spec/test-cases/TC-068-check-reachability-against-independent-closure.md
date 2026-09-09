---
id: TC-068
title: "Check reachability against independent closure"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-068: Check reachability against independent closure

## Description

Property, priority P1. Verifies FR-008-AC-3, FR-008-AC-11. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Enumerate every functional graph on one, two and three vertices with each parent either absent or one vertex. Compute positive-length transitive closure using an independently authored Boolean adjacency-matrix closure algorithm. For every source/target pair invoke the actual reaches evaluator under ample budgets; rename identities and include disconnected/equal-valued nodes.

## Expected Results

Actual truth agrees with positive-length mathematical closure. Isolated start=target is false, self-loop true and a cycle returning to start true. Renaming does not change truth; distinct keys never merge. The complete finite generated domain is reported, without a universal proof claim.

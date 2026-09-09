---
id: TC-065
title: "Bound validation and cancellation"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
---
# TC-065: Bound validation and cancellation

## Description

Property, priority P1. Verifies FR-007-AC-12. Qualified at 45ed1b4 by SR-097 through the public runtime validation tests. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Generate valid inventories, selected populations, shared arenas, Unicode comparisons and many independent defects. Exercise every ValidationLimits counter at measured exact, one below and zero and hard-limit clamping. Include unselected inventory bytes versus selected object work. Poll deterministic cancellation before work and during loops. In a test-only catch_unwind boundary, make the caller poll panic, then run a fresh valid request.

## Expected Results

Exhaustion/cancellation returns actual usage and diagnostics without excess charged work or a context. Valid inputs may complete with zero unused counters or zero diagnostic capacity. Panic unwinds without a successful result or poisoned global state. No sleeps or timing claims are used.

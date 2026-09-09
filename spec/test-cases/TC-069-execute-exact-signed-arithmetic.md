---
id: TC-069
title: "Execute exact signed arithmetic"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-008
    type: verifies
---
# TC-069: Execute exact signed arithmetic

## Description

Integration, priority P1. Verifies FR-008-AC-12. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Evaluate already checked expressions over positive/negative bounded scalars, signed endpoints, add/subtract/multiply, negation, division and remainder with independent integer expectations. Cover all sign pairs and zero numerators; static divide-by-zero and overflow controls must fail checking. Exercise the defensive runtime-invariant result path inside a private unit test without exposing an unchecked public context constructor.

## Expected Results

Admitted results use exact checked i64 arithmetic, truncation toward zero and dividend-signed remainder, preserving authored result intervals. Invalid runtime scalar data fails validation and undefined source fails checking. Defensive execution failure yields runtime_invariant and no Boolean; setup refuses at its actual stage.

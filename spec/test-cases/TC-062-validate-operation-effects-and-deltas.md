---
id: TC-062
title: "Validate operation effects and deltas"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-007
    type: verifies
---
# TC-062: Validate operation effects and deltas

## Description

Integration, priority P1. Verifies FR-007-AC-4, FR-007-AC-11. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Admit empty and explicit field/create/delete frames. Supply complete pre/post populations with permitted and forbidden changes, unchanged survivors, added/deleted objects and wrong, duplicate, intersecting or missing delta entries. Include other supplied model populations and unavailable counterpart populations.

## Expected Results

Actual complete identity differences equal the declared sets. Incorrect declarations receive population_delta_mismatch; unauthorized effects receive frame_violation. Same-identity references across observations are unchanged storage values. Missing/incomplete counterparts cannot establish an empty difference or successful frame.

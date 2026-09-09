---
id: TC-057
title: "Bound runtime artifact construction"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: verifies
---
# TC-057: Bound runtime artifact construction

## Description

Property, priority P1. Verifies FR-018-AC-6. Planned until actual Rust
execution. Setup uses qualified public model/checker APIs; unrelated setup
failure cannot stand in for the intended phase's outcome.

## Test Procedure

Generate flat primitive/shared/container arenas and each metadata-entry class. For every ArtifactLimits dimension measure valid work, test exact, one below and zero, and try options above hard ceilings. Include exact depth 64 and depth 65, unused deep nodes, long multibyte/escaped text and output expansion. Construct a large flat arena on a small Rust thread stack and drop both successful and refused requests.

## Expected Results

Each tested boundary has actual charge-before-work behavior, no partial artifact and bounded structural traversal/destruction. Distinguish text-content and encoded-byte ceilings. Record coupled hard ceilings honestly when another dimension precludes an exact hard-boundary success; lowered isolated boundaries still execute.

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

Property, priority P1. Verifies FR-018-AC-6.
Qualified at c8fa41f with actual Rust evidence and code/Rust review in SR-096.
Setup uses the public runtime input constructors and existing IR
identity types. Construction precedes model-aware validation and needs no
CheckedPackage. Unrelated setup failure cannot stand in for the intended
construction outcome.

## Test Procedure

Generate flat primitive/shared/container arenas and each metadata-entry class. For every ArtifactLimits dimension measure valid work, test exact, one below and zero, and try options above hard ceilings. Include exact depth 64 and depth 65, unused deep nodes, long multibyte/escaped text and output expansion. Construct a large flat arena on a small Rust thread stack and drop both successful and refused requests.

## Expected Results

Each tested boundary has actual charge-before-work behavior, no partial artifact and bounded structural traversal/destruction. Distinguish text-content and encoded-byte ceilings. Record coupled hard ceilings honestly when another dimension precludes an exact hard-boundary success; lowered isolated boundaries still execute.

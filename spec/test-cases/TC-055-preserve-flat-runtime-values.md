---
id: TC-055
title: "Preserve flat runtime values"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: verifies
---
# TC-055: Preserve flat runtime values

## Description

Integration, priority P1. Verifies FR-018-AC-1, FR-018-AC-2, FR-018-AC-5, FR-018-AC-7. Planned until actual Rust
execution. Setup uses the public runtime input constructors and existing IR
identity types. Construction precedes model-aware validation and needs no
CheckedPackage. Unrelated setup failure cannot stand in for the intended
construction outcome.

## Test Procedure

Construct snapshots and invocations with all ValueNode variants, shared earlier children, empty containers, duplicate fields/objects, optional absence and every root position. Add separate forward/self/out-of-range child and root indices, including unused nodes. Retain an independent clone of each draft before consuming the request.

## Expected Results

Valid structural inputs preserve complete vectors and local indices. Invalid edges/roots return invalid_runtime_input with the actual draft path, labels and no artifact or fabricated native source span. Model-invalid duplicate/type inputs can construct but have no validated context. Caller-retained drafts are unchanged.

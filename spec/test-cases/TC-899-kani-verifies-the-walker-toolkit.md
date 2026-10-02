---
id: TC-899
title: "Kani verifies the walker toolkit"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: verifies
---
# TC-899: Kani verifies the walker toolkit

## Description

Verify the walker toolkit's traversal with Kani for every tree within each
harness's bound.

Scope: FR-356-AC-4.

## Test Procedure

1. Run `cargo kani` on the toolkit crate.
2. Read each harness's result.

## Expected Results

- Every harness reports `VERIFICATION:- SUCCESSFUL` with at least one
  SUCCESS check, proving: each node is entered and exited exactly once; each
  exit receives the frame its enter pushed; exits come in reverse enter
  order along each path; a stop ends the walk at once; and no traversal
  panics.

## Status

🚧 Planned.

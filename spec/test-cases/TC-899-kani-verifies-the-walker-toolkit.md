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
- The bound is every tree of at most 4 nodes, enumerated by parent array
  (node 0 is the root, and each other node's parent comes before it): 10
  harnesses, one per parent array. Each harness runs the complete walk, a
  stop on entering and on exiting each node, and the arena's bottom-up
  computation, with a symbolic (`kani::any`) payload in every frame. The
  arena check also proves that an id of another arena, at a position this
  arena holds, reads as `None` from `Arena::get` and from the results view. The
  tree shape is not symbolic, since CBMC does not finish on a symbolic
  shape.

## Status

Implemented. The harnesses are in `quire-walk/src/proofs.rs`; run `cargo kani -p quire-walk`.

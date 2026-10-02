---
id: TC-898
title: "The walker toolkit walks deep trees on a small stack"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: verifies
---
# TC-898: The walker toolkit walks deep trees on a small stack

## Description

Verify that the walker toolkit `quire-walk` is a `no_std` shared leaf, that it enters and exits
every node of a 100,000-deep tree in order with typed frames, that a stop
ends the walk, and that arena order computes bottom-up with no stack.

Scope: FR-356-AC-1, FR-356-AC-2, FR-356-AC-3.

## Test Procedure

Run steps 2 to 5 on a thread spawned with a 512 KiB stack.

1. Build `quire-walk` for `thumbv7em-none-eabihf`, and list its features
   and its dependency tree. Run arch-lint's direction check over a
   manifest set where IR, RT and CG each depend on `quire-walk`, then over
   one where CG depends on `qsl-semantics`.
2. Walk a 100,000-deep chain, recording each enter and exit with the depth
   carried in its frame.
3. Walk a 100,000-level chain alternating an expression node and a type node,
   each with its own frame type, recording enters and exits.
4. Walk the chain of step 2 with an enter callback that stops at depth
   50,000 with a value.
5. Compute each node's subtree size over a 100,000-node arena in one forward
   loop.

## Expected Results

- Step 1: the build succeeds, it has no features, the tree holds only
  `core` and `alloc`, the
  check admits the `quire-walk` edges and refuses the `qsl-semantics` one.
- Steps 2 and 3: every node is entered once in pre-order and exited once in
  post-order, and each exit's frame equals its enter's frame.
- Step 4: the walk returns the value, and no callback runs after the stop.
- Step 5: the root's size is 100,000.

## Status

Implemented. Step 1 is backed by `make quire-walk-no-std`, `tests/it/quire_walk_leaf.rs` and `tools/arch-lint`'s `tc_arch_lint_metadata_009` and `tc_arch_lint_direction_004`; steps 2 to 5 by `quire-walk/tests/it/deep.rs`.

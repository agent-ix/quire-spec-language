---
id: TC-725
title: "Deep expressions check, lower and emit on a small stack"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-258
    type: verifies
---
# TC-725: Deep expressions check, lower and emit on a small stack

## Description

Verify that the checker types, lowers and emits 100,000-deep expressions
over arenas and explicit stacks, and that every emitted body is in FR-322's
five-stratum grammar.

Scope: FR-258-AC-1, FR-258-AC-5.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Check, lower and emit a function whose body is a 100,000-term sum, one
   whose body is a 100,000-long `else if` chain and one whose body is
   100,000 nested `let`s, each with S1 and S3 limits raised to fit it.
   Recompute each emitted node's key from its preimage. Clone each checked
   function body and the lowered graph, compare each with its clone, format
   the checked package for debug and drop all of them.
2. Walk every node body of the checked packages of step 1 and of TC-415,
   classifying each position as Leaf, Group, Tuple, Member or Body.

Tag the tests `#[trace("TC-725", "FR-258-AC-1")]`, `#[trace("TC-725", "FR-258-AC-5")]`.

## Expected Results

- Step 1: each checks and emits; every recomputed key equals its
  `node_id`; the clone compares equal; formatting and drops complete.
- Step 2: no `application` argument, `aggregate` member or `binding` value
  holds a term of its own stratum or a higher one, and every composite
  subterm is a `reference` to its own node.

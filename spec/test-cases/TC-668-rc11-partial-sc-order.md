---
id: TC-668
title: "The SC event graph orders seq_cst events and prunes cyclic choices"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-223
    type: verifies
---
# TC-668: The SC event graph orders seq_cst events and prunes cyclic choices

## Description

Verify the SC event graph's edges and pruning, its collection on garbage collection, and its place in the canonical key.

Scope: FR-223-AC-1 to FR-223-AC-4.

## Test Procedure

1. Check `SB` with `seq_cst` accesses under `ra`; at the state after `Sx`,
   `Ly` and `Sy`, list `Lx`'s enabled message choices.
2. Check IRIW all `seq_cst`, IRIW release/acquire, and `SB` with `seq_cst`
   fences, under `ra`.
3. Run FR-223-AC-3's looping protocol and measure `P` at each state.
4. Run steps 1 and 3 twice and compare.

Tag the tests `#[trace("TC-668", "FR-223-AC-n")]`.

## Expected Results

- Step 1: `proved`; `Lx` enabled for `x1` only.
- Step 2: weak outcome unreachable, reachable, and `proved`.
- Step 3: live SC events at most messages times threads; none on removed
  messages.
- Step 4: equal counts and byte-equal keys.

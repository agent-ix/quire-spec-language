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

Scope: FR-223-AC-1 to FR-223-AC-10.

## Test Procedure

1. Check `SB` with `seq_cst` accesses under `ra`; at the state after `Sx`,
   `Ly` and `Sy`, list `Lx`'s enabled message choices.
2. Check IRIW all `seq_cst`, IRIW release/acquire, and `SB` with `seq_cst`
   fences, under `ra`.
3. Run FR-223-AC-3's looping protocol and measure `P` at each state.
4. Run steps 1 and 3 twice and compare.
5. Run ADR-025 §15's LT-12 and LT-13 under `sc`, `tso` and `ra`, and under
   an independent RC11 execution-graph checker.
6. Run ADR-025 §15's LT-14 under `ra` and list `w`'s load of `X` at the
   state after `u` has run, `v` has stored `P := 1` after reading `Y = 1`, and
   `w` has stored `P := 2` after it in modification order. Run LT-18 under
   `ra`.
7. Run ADR-025 §15's LT-15 and LT-16 under `ra`, and at each state in which
   `L := 1` or `L := 2` has been collected list the live SC events, the
   `owed` sets and the pairs of the views.
8. Run ADR-025 §15's LT-17 under `ra` in every interleaving, and list the live
   fences after `u`'s second fence with and without `v` having loaded `Y`.
9. Run a thread that fences twice with `seq_cst`, with no other thread; then
   a thread running `fence seq_cst; Y := 1; fence seq_cst` with `Y := 1` a
   `release` store, with no other thread. List the live fences after the
   second fence of each.

Tag the tests `#[trace("TC-668", "FR-223-AC-n")]`.

## Expected Results

- Step 1: `proved`; `Lx` enabled for `x1` only.
- Step 2: weak outcome unreachable, reachable, and `proved`.
- Step 3: live SC events at most messages times threads; none on removed
  messages.
- Step 4: equal counts and byte-equal keys.
- Step 5: both outcomes reachable under `ra` only; every outcome the checker
  allows is reachable under `ra`.
- Step 6: the LT-14 outcome unreachable and the load enabled for `X = 1` only;
  the LT-18 outcome unreachable.
- Step 7: both outcomes unreachable; each collected event absent from `P`, and
  its live predecessors present in `owed` or in the pairs of the views that
  covered it.
- Step 8: the outcome unreachable; the first fence present while `Y`'s message
  view covers it and not the second, absent once no view does.
- Step 9: in the first run the first fence is absent after the second fence;
  in the second it is present while `Y`'s message is live and its view covers
  the first fence and not the second.

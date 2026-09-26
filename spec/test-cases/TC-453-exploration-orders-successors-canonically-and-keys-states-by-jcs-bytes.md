---
id: TC-453
title: "Exploration orders successors canonically and keys states by their JCS bytes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: verifies
---
# TC-453: Exploration orders successors canonically and keys states by their JCS bytes

## Description

Verify the canonical successor order and the state key. Scope: FR-101-AC-1
and FR-101-AC-2.

## Test Procedure

1. Explore a system whose state 0 lists transitions `z` then `a`, with
   `max_depth` 1.
2. Explore a system whose state 0 lists `step(9)` then `step(10)`, integer
   arguments, with `max_depth` 1.
3. Explore a system whose state 0 lists two successors with the same
   transition identity and different post-states, the larger state key
   first, with `max_depth` 1.
4. Explore a system that lists its initial states in descending state-key
   order, with `max_states` equal to their count and `max_depth` 0.
5. Explore the graph 0 → {1, 2}, 1 → 3, 2 → 4, with generous limits and a
   poll that cancels at the fourth expansion.
6. Compute the state key and digest of the FR-101-AC-2 states with
   `float64` bits `0000000000000000` and `8000000000000000`, and of two
   `float64` NaNs with bits `7ff8000000000000` and `7ff8000000000001`.
   Explore a system with an initial state and one successor for each of
   those two value pairs.
7. Explore two paths 0 → 1 → 3 and 0 → 2 → 3 that reach an equal state
   key.

## Expected Results

- Step 1: the frontier lists `a`'s post-state before `z`'s.
- Step 2: the frontier lists `step(10)`'s post-state before `step(9)`'s.
- Step 3: the frontier lists the smaller state key first.
- Step 4: the frontier lists the initial states in ascending state-key
  order.
- Step 5: the frontier is 3's key then 4's: level-2 parents keep the FIFO
  order 1, 2.
- Step 6: the two digests are
  `943ae638f84583f2a35a7a92f1eac7f58c380c045298892fb1412756a1cc95e6` and
  `92a3e9557f9aaf672a61aaab71eb2bfad13a0d88662953998de4057ed54e4b01`, each a
  `DigestRecord` under `quire.simulation.state-key/v1`; the NaN keys differ;
  each exploration counts two states.
- Step 7: `Exhaustive` with 4 states and 4 transitions.

Tag each test `#[trace("TC-453", "FR-101-AC-n")]` with its AC.

## Status

🚧 Planned (QSL-272). Step 7 exists today as
`key_equal_coalescing_merges_two_paths_to_the_same_state`, tagged with QSpec
ids; it is retagged. `canonical_order_is_the_systems_authored_successor_order`
asserts the opposite of step 1 and is replaced.

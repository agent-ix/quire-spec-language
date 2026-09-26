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

Verify the canonical successor order, the order and coalescing of initial
states, and the state key. Scope: FR-101-AC-1, FR-101-AC-2, FR-101-AC-9 and
FR-101-AC-11.

Every fixture fixes its post-state keys against the order under test, so a
rule that sorts by post-state key, or re-sorts a level, gives a different
frontier.

## Test Procedure

Every step is an integration test in `qsl-eval/tests/it/` through the
public entries; `explore` is `pub(crate)` (FR-101).

1. Call `explore_request` with an empty `domains` set, `max_depth` 1, on a system whose state 0 lists transitions
   `z` then `a`, where `a`'s post-state key is greater than `z`'s.
2. Call `explore_request` with an empty `domains` set, `max_depth` 1, on a system whose state 0 lists `step(9)` then
   `step(10)`, integer arguments, where `step(10)`'s post-state key is
   greater than `step(9)`'s.
3. Call `explore_request` with an empty `domains` set, `max_depth` 1, on a system whose state 0 lists two successors
   with the same transition identity and different post-states, the larger
   state key first.
4. Call `explore_request` with an empty `domains` set, `max_states` 3 and `max_depth` 0, on a system that lists three
   distinct initial states in descending state-key order and then repeats
   one of them.
5. Call `explore_request` with an empty `domains` set, generous limits and a poll that cancels at the fourth
   expansion, on the graph 0 -`a`→ 1, 0 -`b`→ 2, 1 → 3, 2 → 4, with states
   chosen so that key(1) > key(2) and key(3) > key(4).
6. Call `explore_request` with an empty `domains` set, `max_depth` 0, on systems whose one initial state is the
   FR-101-AC-2 state with `float64` bits `0000000000000000`,
   `8000000000000000`, `7ff8000000000000` and `7ff8000000000001` in turn,
   and read each frontier digest. Then call `explore_request` with an empty `domains` set, generous limits, on a
   system with an initial state and one successor for each of the two
   value pairs (the signed zeros, the NaNs).
7. Call `explore_request` with an empty `domains` set, generous limits, on two paths 0 → 1 → 3 and 0 → 2 → 3 that
   reach an equal state key.
8. Call `explore_request` with an empty `domains` set, `max_states` 2, on a system that lists four distinct initial
   states a < b < c < d (by key) in the order d, b, a, c; then with
   `max_states` 0; then on a system with no initial state.
9. Call `explore_request` and `sample_request` on a system whose one state's key is a bare `u64` above `2^53`, and
   call `replay` on that same system with a trace of any shape.

## Expected Results

- Step 1: the frontier lists `a`'s post-state digest before `z`'s.
- Step 2: the frontier lists `step(10)`'s post-state digest before
  `step(9)`'s.
- Step 3: the frontier lists the smaller state key's digest first.
- Step 4: `Bounded` at `Limit::Depth` with 3 states; the frontier is the
  three distinct initial states' digests in ascending key order. The
  duplicate is one state.
- Step 5: `Cancelled`, frontier 3's digest then 4's: level-2 parents keep
  the FIFO order 1, 2.
- Step 6: the digests are
  `943ae638f84583f2a35a7a92f1eac7f58c380c045298892fb1412756a1cc95e6`,
  `92a3e9557f9aaf672a61aaab71eb2bfad13a0d88662953998de4057ed54e4b01`,
  `a3d5ecff68c7cfb60a687aa72b743a04e1dc8513e348b9c3f64393dd96f4bdec` and
  `62c344cca9a4942b80644ca8527bc7ccced905f01bf67262a9bd5e824356a955`, each a
  `DigestRecord` under `quire.simulation.state-key/v1`; each exploration
  counts two states.
- Step 7: `Exhaustive` with 4 states and 4 transitions.
- Step 8: `Bounded` at `Limit::States` with 2 states and frontier a, b, c,
  d (admitted a, b; refused c; then d), as digests; with `max_states` 0,
  `Bounded` at `Limit::States` with 0 states and frontier a, b, c, d; with
  no initial state, `Exhaustive` with zero states, transitions and depth.
- Step 9: `explore_request` and `sample_request` refuse
  `NotSimulated::KeyEncoding`, and `replay` refuses `ReplayError::KeyEncoding`,
  instead of aborting the process.

Tag each test `#[trace("TC-453", "FR-101-AC-n")]` with its AC.

## Status

✅ Implemented (QSL-272). FR-101's "Existing test disposition" table maps
each existing test to its step here. Step 9 (FR-101-AC-11) was added in the
QSL-272 review round (SR-672 FND-002, SR-673 FND-002).

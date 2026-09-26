---
id: TC-455
title: "Stopped explorations stay incomplete, and unbounded requests require a bound"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: verifies
---
# TC-455: Stopped explorations stay incomplete, and unbounded requests require a bound

## Description

Verify cancellation, limit exhaustion, exhaustive counts and the
`requires-bound` disposition. Scope: FR-101-AC-6, FR-101-AC-7 and
FR-101-AC-8.

## Test Procedure

1. Explore the graph 0 → {1, 2}, 1 → 3 with a poll that cancels at the
   second expansion.
2. Explore the chain 0 → 1 → 2 with `max_depth` 2 and then 3, `max_states`
   2 and then 3, and `max_transitions` 1 and then 2.
3. Explore the graph 0 → {1, 2}, 1 → 3 with `max_states` 3.
4. Call `explore_request` and `sample_request` with `domains` holding one
   parameter typed `Integer` (no range), on a `TransitionSystem` that
   records every method call. Repeat `explore_request` with `Limits` at
   `usize::MAX`. Then call `explore_request` with a bounded domain and
   `position_limit` 0.
5. Explore the graph 0 → {1, 2}, 1 → 3, 2 → 4 with generous limits.
6. Call `explore_request` with the parameter typed `Int[0, 9]`.

## Expected Results

- Step 1: `Cancelled` with `cause` `CatalogCode::new("cancelled",
  "caller-cancelled")`, category incomplete. State 1 was about to be
  expanded, so the frontier is 1's digest then 2's.
- Step 2: each smaller limit returns `Bounded` with its `Limit` and a
  frontier of digests, category incomplete; each larger limit returns
  `Exhaustive`. For this chain those are the counts FR-101-AC-7 names: 3
  states, 2 transitions, deepest depth 2.
- Step 3: `Bounded` at `Limit::States` with frontier 1's, 2's, 3's digests:
  the blocked successor follows the queue.
- Step 4: `NotSimulated::RequiresBound` naming the parameter's domain key
  and kind `Integer`, no `Outcome` or `Trace`, and no recorded
  `TransitionSystem` call, under both `Limits`. The `position_limit` 0 call
  returns `NotSimulated::Extent` with a node-count stage limit.
- Step 5: `Exhaustive` with 5 states, 4 transitions and depth 2.
- Step 6: an `Outcome` from exploration.

Tag each test `#[trace("TC-455", "FR-101-AC-n")]` with its AC.

## Status

🚧 Planned (QSL-272). FR-101's "Existing test disposition" table maps each
existing test to its step here. The cancellation cause, digest frontiers
and `NotSimulated` are not implemented.

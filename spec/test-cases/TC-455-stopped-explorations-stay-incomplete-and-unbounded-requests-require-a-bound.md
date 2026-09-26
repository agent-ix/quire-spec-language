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

Verify cancellation, limit exhaustion and the `requires-bound` disposition.
Scope: FR-101-AC-6, FR-101-AC-7 and FR-101-AC-8.

## Test Procedure

1. Explore the graph 0 → {1, 2}, 1 → 3 with a poll that cancels at the
   second expansion.
2. Explore the chain 0 → 1 → 2 with `max_depth` 2 and then 3, `max_states`
   2 and then 3, and `max_transitions` 1 and then 2.
3. Explore the graph 0 → {1, 2}, 1 → 3 with `max_states` 3.
4. Submit a simulation request over a parameter typed `Integer` (no range),
   with a `TransitionSystem` that records every method call. Repeat with
   `Limits` at `usize::MAX`.
5. Submit the same request with the parameter typed `Int[0, 9]`.

## Expected Results

- Step 1: `Cancelled` with cause `cancelled`/`caller-cancelled`, category
  incomplete. State 1 was about to be expanded, so the frontier is 1's
  digest then 2's.
- Step 2: each smaller limit returns `Bounded` with its `Limit` and a
  frontier of digests, category incomplete; each larger limit returns
  `Exhaustive`.
- Step 3: `Bounded` at `Limit::States` with frontier 1's, 2's, 3's digests.
- Step 4: `RequiresBound` naming the parameter's domain key and kind
  `Integer`, no `Outcome`, and no recorded `TransitionSystem` call, under
  both `Limits`.
- Step 5: an `Outcome` from exploration.

Tag each test `#[trace("TC-455", "FR-101-AC-n")]` with its AC.

## Status

🚧 Planned (QSL-272). The limit and frontier tests in
`qsl-eval/tests/it/finite_simulation.rs` back steps 2 and 3 today, untagged
or under QSpec ids, and are retagged with frontier digests. The cancellation
cause and `RequiresBound` are not implemented.

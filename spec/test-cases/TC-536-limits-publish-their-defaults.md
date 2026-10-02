---
id: TC-536
title: "Exploration and model-check limits publish their defaults"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-101
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-126
    type: verifies
---
# TC-536: Exploration and model-check limits publish their defaults

## Description

Verify the published defaults of FR-101's `Limits` and FR-126's
`ModelCheckLimits`, that a run under the defaults completes a small model,
and that a run stopped by a limit names the limit, its value and the member
that raises it.

Scope: FR-101-AC-15, FR-126-AC-8.

## Test Procedure

1. Read `Limits::default()` and `ModelCheckLimits::default()`.
2. Explore a 3-state chain with `Limits::default()`, then with `max_states`
   2 and the other members at their defaults.
3. Run FR-126-AC-1's requests with `ModelCheckLimits::default()`, then the
   weak-`whole` request with `max_states` 2 and the other members at their
   defaults.

Tag the tests `#[trace("TC-536", "FR-101-AC-15")]` and
`#[trace("TC-536", "FR-126-AC-8")]`.

## Expected Results

- Step 1: `Limits::default()` is `max_states` 10,000,000 and
  `max_transitions` 100,000,000; `ModelCheckLimits::default()` has
  `limits` equal to `Limits::default()` and `max_automaton_states`
  1,048,576; neither limits type has a depth member, and
  `max_live_instances`, `max_store_buffer` and `max_messages` are request
  members, `None` by default.
- Step 2: `Exhaustive`; `Outcome::Bounded` at `Limit::States`, value 2.
- Step 3: the FR-126-AC-1 outcomes; `Stopped{ResourceExhausted,
  {MaxStates, 2}}`.

## Status

🚧 Steps 1 and 2 for `Limits` pass locally (`tc_536_exploration_limits_publish_their_defaults`); the `ModelCheckLimits` half (FR-126-AC-8) is not yet implemented.

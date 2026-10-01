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

- Step 1: `max_states` 10,000,000, `max_transitions` 100,000,000,
  `max_depth` `usize::MAX`; `max_automaton_states` 1,048,576.
- Step 2: `Exhaustive`; `Outcome::Bounded` at `Limit::States`, value 2.
- Step 3: the FR-126-AC-1 outcomes; `Stopped{ResourceExhausted,
  {MaxStates, 2}}`.

---
id: TC-439
title: "An exploration outcome maps to its O-16 category and keeps its frontier"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: verifies
---
# TC-439: An exploration outcome maps to its O-16 category and keeps its frontier

## Description

Verify `explore::Outcome::category()`. Scope: FR-097-AC-5.

## Test Procedure

1. Explore the graph 0 → 1 → 2 with generous limits.
2. Explore it with `max_depth` 1.
3. Explore it with a poll that cancels at once.

## Expected Results

- Step 1: `Exhaustive`, category success.
- Step 2: `Bounded` at `Limit::Depth` with frontier [1], category
  incomplete.
- Step 3: `Cancelled` with frontier [0], category incomplete.

## Status

Backed: `qsl-eval/tests/it/finite_simulation.rs`, `tc_439_explore_outcomes_map_to_their_o16_category`.

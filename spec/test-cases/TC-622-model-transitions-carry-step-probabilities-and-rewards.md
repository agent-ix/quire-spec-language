---
id: TC-622
title: "ModelSystem gives actions, step probabilities and rewards, and reports NotMarkov"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-187
    type: verifies
---
# TC-622: ModelSystem gives actions, step probabilities and rewards, and reports NotMarkov

## Description

Verify FR-187's `actions` and `weighted_steps` on `Service` and `Link`, `NotMarkov` on the `Health` variants, the reward non-negativity check and support-graph equality.

Scope: FR-187-AC-1 to FR-187-AC-4.

## Test Procedure

Fixtures: `Service` (ADR-024 §7.2); `Link` (ADR-028 §15.4's model text) with workload `Even`; the two `Health` variants of FR-187-AC-3; the `refund` model of FR-187-AC-4.

1. `weighted_steps` on `Service` at `Busy, attempts = 0` and at `Idle` under `Steady`.
2. `weighted_steps` and `actions` on `Link` at its initial state under `Even`, and at a delivered state.
3. `weighted_steps` and `actions` on each `Health` variant at its initial state.
4. Admit the `refund` model; compare positive-probability transitions with FR-120's successors over every state of `Service`.

Tag the tests `#[trace("TC-622", "FR-187-AC-n")]`.

## Expected Results

- Step 1: six steps with the probabilities of FR-187-AC-1 summing to 1 and `duration = d`; one `request` step with probability 1.
- Step 2: `9/20`, `1/20`, `2/5`, `1/10`; two actions with draws summing to 1; the stutter step with probability 1.
- Step 3: `NotMarkov` with 2 post-states and an action whose draws carry two successors; `NotMarkov` with 0 post-states.
- Step 4: `NegativeReward` naming `n = 0`; equal sets.

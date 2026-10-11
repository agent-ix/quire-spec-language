---
id: TC-975
title: "Exact evidence kind is admitted for probabilistic backends"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-975: Exact evidence kind is admitted for probabilistic backends

## Description

Verify a probabilistic backend may advertise `exact`, alone or together with
`statistical`, and descriptor admission retains the exact set. Scope:
FR-075-AC-9.

## Test Procedure

1. Admit a descriptor advertising `probabilistic-satisfaction` with
   `{exact}`.
2. Admit another with `{statistical, exact}`.
3. Inspect each admitted descriptor's evidence-kind set.

## Expected Results

Both descriptors are admitted. Their retained sets equal the respective input
sets exactly; neither label substitutes for the other.

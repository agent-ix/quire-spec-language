---
id: TC-980
title: "Probabilistic candidates require the exact requested evidence kind"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-980: Probabilistic candidates require the exact requested evidence kind

## Description

Verify candidate computation does not substitute one probabilistic evidence
kind for another. Scope: FR-075-AC-1 and FR-075-AC-12.

## Test Procedure

1. Register backend `stat` advertising `probabilistic-satisfaction` with
   `{statistical}` and backend `ex` advertising the same capability with
   `{exact}`.
2. Compute unnamed candidates for one probabilistic item naming `statistical`
   and one naming `exact`.
3. Repeat with only `stat` registered for an item naming `exact`, then with
   only `ex` registered for an item naming `statistical`.

## Expected Results

- The `statistical` item has exactly candidate `stat`; the `exact` item has
  exactly candidate `ex`.
- In each single-backend control, the candidate set is empty. Neither evidence
  kind substitutes for the other, even when the other kind has a registered
  backend.

---
id: TC-568
title: "Initial states not closed under the generators settle SymmetryBroken; a finer class admits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-151
    type: verifies
---
# TC-568: Initial states not closed under the generators settle SymmetryBroken; a finer class admits

## Description

Verify generator closure of the initial states and the instance groups of a finer declaration.

Scope: FR-151-AC-3 to FR-151-AC-4.

## Test Procedure

Fixtures: ADR-021 §7.1's annotated unit; initial states `(1,0,0)`; `{(0,0,0), (1,0,0)}`; and that set with `(0,1,0)` and `(0,0,1)` added.

1. Pre-check `[[a, b, c]]` and `[[a], [b, c]]` with initial state `(1,0,0)`.
2. Pre-check `[[a, b, c]]` with the two-state and the four-state initial sets.

Tag the tests `#[trace("TC-568", "FR-151-AC-n")]`.

## Expected Results

- Step 1: `SymmetryBroken{config_history, InitialStatesNotClosed{0, (a b)}}` with no state expanded; then admitted with instances `a` (group `{id, (b c)}`) and `b` (orbit `[b, c]`, trivial group).
- Step 2: `SymmetryBroken` for the two-state set; admitted for the four-state set.

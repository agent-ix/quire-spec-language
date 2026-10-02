---
id: TC-574
title: "Read, write and enabling footprints derive from clauses and frames, and decide independence"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-155
    type: verifies
---
# TC-574: Read, write and enabling footprints derive from clauses and frames, and decide independence

## Description

Verify symbolic and instantiated footprints, meeting and independence over receiver, navigation, parameter and membership reads.

Scope: FR-155-AC-1 to FR-155-AC-2.

## Test Procedure

Fixtures: ADR-021 §7.2's `Counter` unit with the operations of FR-155-AC-2 added.

1. Derive and instantiate `bump`'s footprint; test independence of `bump(a)` with `bump(b)` and with itself.
2. Derive the footprints of every-object `bump`, `copy`, `give` and `spawn`, and of an operation whose pre quantifies over `counters`; test the stated pairs.

Tag the tests `#[trace("TC-574", "FR-155-AC-n")]`.

## Expected Results

- Step 1: `(Receiver, v)` in each set, `Field{a, v}` instantiated; independent; dependent.
- Step 2: the footprints and dependences exactly as FR-155-AC-2 states.

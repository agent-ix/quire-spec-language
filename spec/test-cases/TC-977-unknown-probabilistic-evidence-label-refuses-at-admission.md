---
id: TC-977
title: "Unknown probabilistic evidence labels refuse at admission"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-977: Unknown probabilistic evidence labels refuse at admission

## Description

Verify every evidence label outside the exact FR-290 set refuses at descriptor
admission. Scope: FR-075-AC-10.

## Test Procedure

1. Register a valid descriptor for backend `p` advertising
   `probabilistic-satisfaction` with `{statistical}`.
2. For that same identity, separately submit the evidence-kind sets
   `{approximate}`, `{Statistical}`, `{statistical }` and
   `{statistical, approximate}`.

## Expected Results

Each descriptor refuses as a whole before registry insertion with
`invalid_capability`/`unknown-evidence-kind`, keyed by `BackendId`. No label is
normalized, aliased or replaced, and none of these refusals withdraws the
valid descriptor already held for `p`.

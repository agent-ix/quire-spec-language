---
id: TC-971
title: "Capability family projection matches FR-290"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-971: Capability family projection matches FR-290

## Description

Verify each capability's kind-group projection matches the family assigned by
FR-290, while declaration applicability is the declaration-family mapping
FR-507 states, not the kind group. Scope: FR-057-AC-11 and FR-057-AC-13.

## Test Procedure

1. For each of FR-057's twelve labels, compare `families()` with that label's
   FR-290 family column.
2. Submit `probabilistic-satisfaction` and `schedulability` requests against a
   probabilistic claim, a task-set claim and a state declaration.
3. Submit a `finite-replay` request against a state declaration as a control.

## Expected Results

- Each `families()` result equals FR-290's exact family: `value`,
  `state-model`, `finite-replay`, `temporal-trace`, `protocol`,
  `probabilistic` or `real-time` as listed in FR-057.
- `probabilistic-satisfaction` applies on the probabilistic claim and
  `schedulability` on the task-set claim; both are inapplicable to the state
  declaration and name the declaration family `state-model`; neither is
  relabeled.
- The control is inapplicable to the state family, and the declaration body
  still reaches its family checker.

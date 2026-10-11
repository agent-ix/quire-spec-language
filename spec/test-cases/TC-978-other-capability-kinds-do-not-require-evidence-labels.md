---
id: TC-978
title: "Other capability kinds do not require evidence labels"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-075
    type: verifies
---
# TC-978: Other capability kinds do not require evidence labels

## Description

Verify registrations that do not advertise `probabilistic-satisfaction` need
no evidence-kind labels. Scope: FR-075-AC-11.

## Test Procedure

For each of the other eleven FR-290 capability kinds, submit a descriptor that
advertises that kind with a valid mode and no evidence-kind set.

## Expected Results

Each descriptor is admitted. No evidence-kind set is required for any of the
eleven kinds, including `schedulability`.

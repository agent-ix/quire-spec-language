---
id: TC-1328
title: "A hybrid claim emits a name row and no clock binding"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-447
    type: verifies
  - target: ix://agent-ix/quire-specification/FR-322
    type: verifies
---
# TC-1328: A hybrid claim emits a name row and no clock binding

## Description

Verify that hybrid claims are named per `claim` occurrence and carry no `ClockConfiguration` binding.

Scope: FR-447-AC-18 and FR-447-AC-19.

## Test Procedure

1. Compile the use case's `Braking` and `SlowEnough` and read the name and binding members.
2. Compile two hybrid claims with equal bodies and different names.
3. Compile a unit holding a hybrid claim, a clock-configured temporal clause and a hyper clause.

Tag the tests `#[trace("TC-1328", "FR-447-AC-18", "FR-447-AC-19", "FR-322-AC-344", "FR-322-AC-346")]`.

## Expected Results

- Step 1: one name row for `SlowEnough` at its node and occurrence ordinal; no `temporal_bindings` member.
- Step 2: one node, two name rows at distinct ordinals in (NodeId, occurrence) order.
- Step 3: a name row per occurrence of all three, one array sorted by (NodeId, occurrence); a binding row only for the clock-configured clause.

---
id: TC-154
title: "Refuse a capability carrier with an unsupported vocabulary version"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: verifies
---
# TC-154: Refuse a capability carrier with an unsupported vocabulary version

## Description

Verify that a serialized carrier of capability labels is read only under
capability vocabulary version `quire.capability-kind/v1`. Scope: FR-057-AC-3.

## Test Procedure

1. Read a carrier that declares `quire.capability-kind/v1` and holds two
   admitted labels, as the control.
2. Remove the version declaration and read it again.
3. Declare, one at a time, `quire.capability-kind/v0`,
   `quire.capability-kind/v2` and `quire.capability-kind/V1`, keeping the same
   labels.

## Expected Results

- Step 1 reads both labels.
- Steps 2 and 3 each refuse the carrier with
  `invalid_capability`/`unsupported-version`, naming the received version or
  its absence. No label of the refused carrier is read or reported as a kind.
- Assertions compare typed codes, causes and received bytes, never message
  text.

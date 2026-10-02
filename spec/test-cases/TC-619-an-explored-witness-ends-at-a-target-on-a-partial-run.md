---
id: TC-619
title: "An explored witness ends at a target node on a run with open nodes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-168
    type: verifies
---
# TC-619: An explored witness ends at a target node on a run with open nodes

## Description

Verify that the explored witness descends the distance to target nodes
alone, so on a run with open nodes it ends at a target node and never at an
open node.

Scope: FR-168-AC-9.

## Test Procedure

Fixture: FR-168-AC-9's `Fork` unit and `possible x.pos = 4`.

1. Run the item with `witness_samples` 0 under horizon `max_depth` 2,
   recording the open nodes, each node's `d` and `d_P`, and the outcome.

Tag the tests `#[trace("TC-619", "FR-168-AC-9")]`.

## Expected Results

- Step 1: the node with `pos = 3` is open; `pos = 1` and `pos = 2` have `d`
  1; the explored witness is `toB, fromB`, ending at `pos = 4`; the outcome
  is `WitnessedUnchecked`.

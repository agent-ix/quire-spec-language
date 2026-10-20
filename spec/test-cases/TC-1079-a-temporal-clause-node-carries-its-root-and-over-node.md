---
id: TC-1079
title: "A temporal clause node carries the root and over node a closure check resolves"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-337
    type: verifies
---
# TC-1079: A temporal clause node carries the root and over node a closure check resolves

## Description

Verify that the S4 clause node is the only source of a clause's identity triple and `over` node, and that the I2 reader returns them and refuses a foreign lookup.

Scope: FR-337-AC-5.

## Test Procedure

Use ADR-018 §6's ConfigVersion example unit.

1. Compile the unit and read `ReachesTwo` back through the I2 reader.
2. Resolve the returned triple in the recompiled package.
3. Compile two structurally identical clause declarations and read both
   triples; compile a clause with no `over` parameter (a clause such as
   `eventually holds(true)` with no bound variable; waits on STD-186, since
   S1 requires `over` today) and read it.
4. Resolve a triple with an unknown NodeId, with a non-temporal node's
   NodeId, with role `invariant`, and with another ordinal.

Tag the tests `#[trace("TC-1079", "FR-337-AC-5")]`.

## Expected Results

- Step 1: the triple `[node, "claim", ordinal]` with `node` the emitted NodeId, and the `over` node equal to `c`'s parameter node.
- Step 2: the clause is found.
- Step 3: one node, two triples with distinct ordinals, each resolving to
  its own declaration; the parameter-free clause returns its triple and no
  `over` node, with no refusal.
- Step 4: each lookup returns `clause-root-unresolved` at that triple.

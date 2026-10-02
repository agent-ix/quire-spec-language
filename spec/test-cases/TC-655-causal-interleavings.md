---
id: TC-655
title: "The protocol system admits exactly the causal interleavings"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-210
    type: verifies
---
# TC-655: The protocol system admits exactly the causal interleavings

## Description

Verify that event projections of the protocol system's behaviours are exactly the linear extensions of the causal edge relation.

Scope: FR-210-AC-1 to FR-210-AC-4.

## Test Procedure

For each protocol below, enumerate every behaviour with `ProtocolSystem<Sc>`
and collect the set of event projections.

1. Three single-attempt branches, in two source orders.
2. Branches `a` (`X` then `Y`) and `b` (`Z`).
3. A send in `s` and its receive in `r`, then with an opposite second
   channel.
4. FR-052-AC-1's split shipments.

Tag the tests `#[trace("TC-655", "FR-210-AC-n")]`.

## Expected Results

- Step 1: six projections, the same set in both source orders.
- Step 2: exactly `X Y Z`, `X Z Y`, `Z X Y`.
- Step 3: no `receive` before `send`, every other extension present, the
  set unchanged by the second channel.
- Step 4: the six order-preserving interleavings, each followed by
  `finish`.

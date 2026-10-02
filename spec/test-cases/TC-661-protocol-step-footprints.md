---
id: TC-661
title: "Protocol steps carry static footprints, enabling footprints and visibility"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-216
    type: verifies
---
# TC-661: Protocol steps carry static footprints, enabling footprints and visibility

## Description

Verify per-step read and write footprints, enabling footprints, independence and visibility over the protocol locations.

Scope: FR-216-AC-1 to FR-216-AC-4.

## Test Procedure

1. Read the footprints of `attempt(A)` and `attempt(B)` in `Fill` and in
   the two-object variant; compute independence and visibility for a claim
   over `d.v`.
2. Read `fork(Both)`'s and `join(Both)`'s footprints; compute visibility
   for `always holds(k.v <= 3)`.
3. Read `join`'s write footprint under `cancel` and `continue`; read the
   `receive` enabling footprint and the `Chatter` independence pairs.
4. Read §7.1's registration footprints; for each step, perturb a location
   outside its `R` and diff the post-states.
5. Read the enabling footprints of FR-216-AC-5's `bump` attempt and of
   `duplicate(ch, Tx)`.

Tag the tests `#[trace("TC-661", "FR-216-AC-n")]`.

## Expected Results

- Step 1: dependent in `Fill`; independent in the variant, `attempt(A)`
  invisible and `attempt(B)` visible.
- Step 2: the footprints of FR-216-AC-2; control steps invisible.
- Step 3: the footprints and pairs of FR-216-AC-3.
- Step 4: the `reg` footprints of FR-216-AC-4; every change confined to
  the step's `W`.
- Step 5: exactly the enabling locations FR-216-AC-5 lists, as
  `Footprint` values with `Location::Protocol` for protocol locations.

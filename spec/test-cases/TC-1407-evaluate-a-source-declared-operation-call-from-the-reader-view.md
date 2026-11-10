---
id: TC-1407
title: "Evaluate a source-declared operation call from the reader view"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
---
# TC-1407: Evaluate a source-declared operation call from the reader view

## Description

Verify that the emission carries a source-operation identity row for each
source-declared operation anchor, that a slot calling such an operation
evaluates on the reader and bytes routes to the source route's value and charge
log, that a missing, unknown or foreign row is refused before any node is
projected, and that until IR's FR-352 witness kind exists the call reports the
typed `MissingSelectedJoin`. Scope: FR-390-AC-70 to FR-390-AC-73 (QSL-1400).

## Test Procedure

Fixture E-S is TC-1070's fixture E with the type `Node` and its operation
`step` declared by the unit's own source; fixture E (model-owned `step`) is the
control.

1. Emit E-S and E. Read the `source_operations` rows of the emission and of
   `identities.json` (`identities/4`). (AC-70)
2. Vector S1: call `Node::step` from a slot of E-S with TC-1070's row-1 supply;
   S2: the same with the supply of its row 2 (a guard that short-circuits before
   the call); S3: with the supply that makes the call's argument `Unavailable`
   (TC-1072). Evaluate each on the source route, the in-process reader route and
   the bytes route; compare value and charge log. Repeat TC-1070's row 1 on E.
   (AC-71; runs when IR's witness kind is in the lock.)
3. Vector R1: a supplier with the `step` row removed. R2: the row with its
   `source_identity` changed to `Node::halt`. R3: the row of a second emission
   of E-S under another source authority. R4: the row's `structural_node` set
   to a `state`/`state_clause` node. (AC-72)
4. With no IR witness kind in the lock, call `step` from the E-S slot of S1 and
   read the result; evaluate a slot of E-S that names no operation. (AC-73)

Tag the test `#[trace("FR-390-AC-70", "FR-390-AC-71", "FR-390-AC-72", "FR-390-AC-73", "TC-1407")]`.
Vectors reference TC-1070 and TC-1072 rows, never copy them.

## Expected Results

- Step 1: E-S has one row (`Node::step`, the unit's authority, the anchor's full
  id); E has none; the file round-trips equal.
- Step 2: S1 to S3 give equal values and equal charge logs on all three routes;
  E's row 1 is unchanged.
- Step 3: R1 `MissingIdentityWitness`; R2 and R4 `UnknownIdentityWitness`; R3
  `ForeignIdentityWitness`; each at the `Package` locus, no view, no charge.
- Step 4: `ViewFailure::Unsupported`/`MissingSelectedJoin` at the call's `Node`
  locus naming QSL-1398 and IR-1213; the operation-free slot evaluates.

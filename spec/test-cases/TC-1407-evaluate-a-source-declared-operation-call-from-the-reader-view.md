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
source-declared operation anchor, that the slots of a protocol node reaching
such an anchor evaluate on the reader and bytes routes to the source route's
value and charge log, that a missing, unknown or foreign row is refused before
any node is projected and in the order FR-390 states, and that until IR's
FR-352 witness kind exists view creation reports the typed
`MissingSelectedJoin`. Scope: FR-390-AC-70 to FR-390-AC-73 (QSL-1400).

## Test Procedure

Fixture E-S is TC-1070's fixture E, text unchanged, in a unit whose own source
declares the object type `Node` (with the field `total: Total`, the population
`nodes` and the operation `step`) instead of a domain package document. The
operation call is the operation reference `by Service on Node::step` of its
`compensate Refund` and `compensate Partial` nodes and the `attempt Tried`
node; the slots are the ones TC-1070 lists for those nodes. Fixture E, with
`step` model-owned, is the control.

1. Emit E-S and E. Read the `source_operations` rows of the emission and of
   `identities.json` (`identities/4`). (AC-70)
2. Vector S1: evaluate `Refund.retry` of E-S with the supply of TC-1070's retry
   row; S2: `Refund.recover` with the supply of TC-1070's recover row; S3:
   `Refund.recover` with the supply of TC-1072 that makes `target`
   `Unavailable`. Evaluate each on the source route, the in-process reader
   route and the bytes route; compare value and charge log. Repeat the same
   slots on E. (AC-71; runs when IR's witness kind is in the lock.)
3. Vector R1: a supplier with the `step` row removed. R2: the row with
   `source_identity` changed to `Node::halt`. R3: the `step` row of a second
   emission of E-S under another source authority. R4: the row's
   `structural_node` set to a `state`/`state_clause` node. (AC-72)
4. With every row admitted and no IR witness kind in the lock, create the view
   over E-S and over E; evaluate a slot of each on the source route. (AC-73)

Tag the test `#[trace("FR-390-AC-70", "FR-390-AC-71", "FR-390-AC-72", "FR-390-AC-73", "TC-1407")]`.
Vectors reference TC-1070 and TC-1072 rows, never copy them.

## Expected Results

- Step 1: E-S has one row (`Node::step`, the unit's authority, the anchor's
  full id); E has none; the file round-trips equal.
- Step 2: S1 to S3 give equal values and equal charge logs on all three
  routes; E's slots are unchanged.
- Step 3: R1 `MissingIdentityWitness`; R2 and R4 `UnknownIdentityWitness`
  only; R3 `ForeignIdentityWitness` only; each at the `Package` locus, no
  view, no charge.
- Step 4: over E-S, `ViewFailure::Unsupported`/`MissingSelectedJoin` at the
  `Node` locus of the `step` anchor reference and no view; the source route
  evaluates the slot; over E the view is created and the slot evaluates.

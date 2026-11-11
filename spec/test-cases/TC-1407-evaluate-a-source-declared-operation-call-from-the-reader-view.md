---
id: TC-1407
title: "Evaluate a source-declared operation call from the reader view"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-390
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-383
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-569
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
`MissingSelectedJoin`. Scope: FR-390-AC-70 to FR-390-AC-73 (QSL-1400),
FR-383-AC-67 and the E-S part of FR-569-AC-11.

Each step is marked **now** (runs and passes today: the source route and
the S4 refusal) or **waits** (runs once ADR-013 QC-30 is adopted by QSpec and
IR; step 2 also waits on IR's FR-352 witness kind). Until QC-30 lands, S4
refuses every unit that declares a source object type, E-S and TC-1408's
fixture S included, with the single `SourceObjectEmission` refusal and writes
no package, so every waits step has no package to read.

## Test Procedure

Fixture **E-S** is real QSL source: TC-1070's fixture E with the object type
`Node` declared by the unit's own source (FR-569) instead of by the domain
package `M`. Its domain package is fixture E's `M` without the object type
`Node` (and its `owns` relationship, which only fixture L reads) and without
the population `nodes`, and keeps every other declaration of fixture E's `M`. The unit adds this declaration after its `model M` selection:

```
object Node identity (total) {
  total: M::Total;
  population nodes;
  operation step(): M::Flag;
}
```

and its protocol `Refunds` is fixture E's text with every `M::Node` written
`Node` and every `M::Node::step` written `Node::step`: `over (view: Node)`,
`role Service on Node`, the operation references `by Service on Node::step`
of `compensate Refund` and `compensate Partial`, and `attempt Tried by Service
on Node::step`. Those three are the operation call; the slots are the ones
TC-1070 lists for those nodes. The checked graph's `Node` node is the
`model`/`object_type` node S4 emits for the declaration, with `declaration`
`["Node"]` and the unit's `SourceOwner`, and the `step` anchor's `context` is
that node; no node of the graph is edited after S4. The observation is
TC-1070's, with `nodes` and `Node` named by their source identities (FR-106).
Fixture E, with `Node` and `step` model-owned, is the control.

0. **now.** Compile E-S: check it, evaluate vectors S1 to S3 below on the
   source route, and run S4 over E-S and over TC-1408 fixture S. Emit
   fixture E and evaluate its slots on all three routes. (FR-383-AC-67,
   FR-569-AC-11, AC-70, AC-71, AC-73)
1. **waits.** Emit E-S, S and E. Read the `source_operations` and
   `source_objects` rows of each emission and of `identities.json`
   (`identities/4`). (AC-70)
2. **waits** (also on FR-352). Vector S1: evaluate `Refund.retry` of E-S with
   the supply of TC-1070's retry row; S2: `Refund.recover` with the supply of
   TC-1070's recover row; S3: `Refund.recover` with the supply of TC-1072 that
   makes `target` `Unavailable`. Evaluate each on the in-process reader route
   and the bytes route and compare value and charge log with step 0's source
   route. (AC-71)
3. **waits.** Vectors over E-S, and over S with `Order::bump` in place of
   `Node::step`: R1 a supplier with the operation's row removed
   (E-S only: S's anchor is reached by no protocol); R2 the row with
   `source_identity` changed to `Node::halt` (`Order::halt`); R3 the package's
   complete rows plus the row of a second emission under another source
   authority; R3b R3 with that foreign row's `source_identity` also changed to
   the `halt` name; R4 the row's `structural_node` set to a
   `state`/`state_clause` node. (AC-72)
4. **waits.** With every row admitted and no IR witness kind in the lock,
   create the view over E-S. **now**: create it over E, and evaluate a slot
   of E and of E-S on the source route. (AC-73)

Tag the tests with the criteria they assert among `FR-390-AC-70` to
`FR-390-AC-73`, `FR-383-AC-67`, `FR-569-AC-11` and `TC-1407`, for example
`#[trace("FR-390-AC-72", "TC-1407")]`.
Vectors reference TC-1070 and TC-1072 rows, never copy them.

## Expected Results

- Step 0: E-S checks and S1 to S3 evaluate on the source route; S4 over E-S
  refuses `unsupported_construct`/`not-yet-implemented` naming
  `SourceObjectEmission` at `object Node`, and over S at `object Order`, each
  the only refusal, with no package, identity file or handoff directory.
  Fixture E emits, carries no source-operation row, and its slots give the
  results they gave before on all three routes.
- Step 1: E-S has one source-operation row (`Node::step`, the unit's
  authority, the anchor's full id) and one source-object row (`Node`); S has
  one source-operation row (`Order::bump`); E has neither; each file
  round-trips equal.
- Step 2: S1 to S3 give equal values and equal charge logs on all three
  routes; E's slots are unchanged.
- Step 3: R1 `MissingIdentityWitness`; R2 and R4 `UnknownIdentityWitness`
  only; R3 and R3b `ForeignIdentityWitness` only; each at the `Package`
  locus, no view, no charge.
- Step 4: over E-S, `ViewFailure::Unsupported`/`MissingSelectedJoin` at the
  `Node` locus of the `step` anchor reference and no view; the source route
  evaluates the slot; over E the view is created and the slot evaluates.

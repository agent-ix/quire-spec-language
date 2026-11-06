---
id: TC-1333
title: "Evaluate a protocol slot from the checked package's own nodes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-533
    type: verifies
---
# TC-1333: Evaluate a protocol slot from the checked package's own nodes

## Description

Verify protocol-slot evaluation from a bare package and its equality with the source routes. Scope: FR-533-AC-1 to FR-533-AC-20. **R** is the package of FR-517.

## Test Procedure

| Row | AC | Input | Expected |
| --- | --- | --- | --- |
| 1 | 1 | every slot of FR-390's table **R** holds, over the TC-1070 and TC-1315 supplies, on the bare, in-process and reader-backed routes | equal value, category, exit status, catalog code and cause, unavailable binder, location, losses and ordered charge log |
| 2 | 2 | each outcome row of FR-520's map; `retry.relation` at exact usage and one below | the successor outcomes; completes / `Incomplete` naming the counter |
| 3 | 3 | the calls of row 1 in a run that never compiled the source; the in-process `CheckedPackage` held beside the bare package replaced by another unit's with a different expression at the slot | the same results; the bare result is unchanged, the in-process one changes |
| 4 | 4 | search the bare route's reachable functions for `CheckedPackage`, `CheckedBody`, `source()`, source `Graph` and `protocol_selection::select`; the `NodeKind` match | none; no wildcard arm |
| 5 | 5 | bare observation admission over **R**'s model documents; a model selection the package lacks | the snapshot the source route admits with equal usage; `wrong-model-selection` with equal records |
| 6 | 6 | a package whose `model_root` selects another model than the supply's | the source route's record, before any charge |
| 7 | 7 | slots over a plain-Option record, a record with a plain Collection member and a record with an Optional Collection member; a record whose field rows are absent | bare: the first three evaluate with charges equal to the source route's, each binder's value type equal, and the same record value usable unchanged in both environments in original selected declaration-key field order; the last refuses `MissingTypeRepresentation` at the binder, no charge; source route: evaluates |
| 8 | 8 | an `Unavailable` binder that is read | `Unavailable` with the package's location; `None` where the wire has none while the in-process route keeps its source location, recorded as unmet parity |
| 9 | 9 | three slot calls on one meter against three on fresh meters; against the source route | sum for `work_units` and `result_units`, maximum for the other eight; equal to the source route |
| 10 | 10 | `new`, `with_supply`, view creation, selection and supply admission | no counter charged, no `Meter` taken; a refusal leaves the meter identical |
| 11 | 11 | fixture L (TC-1070): select `Decide.visible[0]`, `Decide.cases.guard[0]` and `Decide.cases.guard[1]` by (node, member, index) with a descriptor that carries no span | origins `... 2641..2650`, `... 2735..2748`, `... 2676..2685` respectively; one origin per slot; no ambiguity refusal |
| 12 | 12 | every slot of fixtures E, E-C and L on both routes; the fixture L package with its three `decision` origins renamed to `999..1005`, `1000..1004`, `1001..1002` | equal triple, root, binders and origin; `Got.related.*[0]` have no origin; the renamed package gives `999..1005`, `1000..1004`, `1001..1002` to `visible[0]`, `cases.guard[1]`, `cases.guard[0]` (numeric order, not byte order) |
| 13 | 13 | two package copies of fixture L edited so that `Decide.cases.guard[0]` and `Decide.cases.guard[1]` both have the `got.ready` application as root, retaining the emitted negation node while removing its protocol-location rows: labels `no` and `yes` kept; labels renamed to `b` and `a` | actual call values and charge logs of the two slots equal; completed evaluations have no stop location; `guard[0]` takes `2676..2685` and `guard[1]` `2735..2748` in the first (exact projected-root read `Location` values exchanged, recorded unmet parity), the same origins each on its own case's span in the second |
| 14 | 14 | fixture L's package with the rows of `decision 2641..2650` removed; with the rows of every `got.ready` origin removed; a package copy whose `related` ends are one application under `related 99 100` and `related 100 105` | `visible[0]` takes `2676..2685`, `cases.guard[1]` refuses `MissingLocationProjection` at the `Node` locus of `got.ready`; refusal for both in the second package; `related.from[0]` takes `99 100` and `related.to[0]` `100 105` (byte order would give `100 105` to `from[0]`); no charge |
| 15 | 15 | bare slot calls with no supply, foreign rows, unknown rows, for identity rows and for equality rows; a missing node, a missing member, `MissingTypeRepresentation` | `Package` locus for every witness refusal; `Node`, `Member`, `Node` for the others; exit 21, meter identical |
| 16 | 16 | fixture E's `Refund` retry relation and recovery predicate, the TC-1289 slot and the TC-1290 slot, with equality rows read from the handoff directory; the same slots with the file omitted; `Refund`'s `trigger.guard` with the file omitted | the in-process route's value and charge log; `MissingEqualityWitness`; evaluates |
| 17 | 17 | search the bare route for derivation of equality rows, reads of the three `Emission` row accessors and name-text selection of an origin | none outside the stated rule |
| 18 | 18 | a unit compiled from source in which `trigger.guard` of a copy of fixture E calls a function whose body holds `==`; equality rows omitted, then the rows its handoff writes; a slot that calls no such function, rows omitted | `trigger.guard` refuses `MissingEqualityWitness` at `Package`, meter untouched, then evaluates equal to the in-process route; the other slot evaluates |
| 19 | 19 | **R**'s `Full` and `Partial` `recovery.predicate` over the bare route, with supplies that differ in receipts, target and activation; `Full` with its sum inside `Shape::Circle` and a `Circle(r)` case arm, over matching, mismatching, empty and activated supplies; a `recovered` supply that is an integer; a supply without `target` | the values FR-533-AC-19 states; the case-arm package is strictly admitted, `r` is not a source on either route, and its Boolean outcomes and ordered charge logs equal the source route's; both malformed supplies refuse as `CallFailure::Admission` |
| 20 | 20 | Strictly admitted **R** with the source `Shape` union constructed and matched in `Full`'s recovery predicate; vary receipts, target and activation, then add an unrelated union; compare the bare and source type environments | Equal Boolean values and ordered charges; original union type handle, sealed member/declaration identities, ordered members and positional payload types agree; missing required projections refuse before any charge |

Tag the test `#[trace("FR-533-AC-1", ..., "FR-533-AC-20", "TC-1333")]`; rows 1 and 2 also carry FR-390-AC-11 and FR-520-AC-9.

## Expected Results

Each row's Expected column.

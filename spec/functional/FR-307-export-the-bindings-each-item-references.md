---
id: FR-307
title: "Export the bindings each requested item references, refusing unbound elements per item"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-031
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-305
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-306
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-331
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
---
# FR-307: Export the bindings each requested item references, refusing unbound elements per item

## Description

When the orchestrating driver requests the abstraction bindings for a set of
items of a checked package, the layer-4 `package` export SHALL return, per
item, either the item with every binding it references or a refusal naming
every model element it references that has no binding. A refused item SHALL
NOT affect any other item.

## Inputs

- A `CheckedPackage` carrying a `CheckedAbstractionRelation`
  ([FR-306](FR-306-carry-the-abstraction-relation-in-the-checked-package.md)).
- The requested items, each a requirement record's occurrence key (ADR-012
  §13.5).

## Outputs

One result per requested item: the item with the bindings it references, or
a `missing_declaration`/`missing-name` refusal.

## Behavior

### Caller and position

The export SHALL be callable after S4 and before `route` and E7, and SHALL
name no `BackendId` (ADR-012 §11). The orchestrating driver (ADR-011 T-13)
builds the QSpec FR-331 request from the bound items; the end-to-end run
lives in quire-integration.

### Referenced elements

An item references:

- for every model member read in its checked claim, the receiver's static
  object type, resolved to its `DeclarationKey`, even when a supertype
  declares the member;
- the population of every extent domain of the claim (ADR-012 §15.7);
- when the record's node is a frame, a precondition or a postcondition, the
  `OperationKey` of its operation
  ([FR-305](FR-305-relate-a-frame-binding-to-its-operation-s-frame.md)).

### Per-item results

- A requested occurrence key that names no requirement record SHALL refuse
  `missing_declaration`/`missing-name`, naming the key.
- An item that references an element with no binding SHALL refuse
  `missing_declaration`/`missing-name`. The refusal SHALL name every unbound
  element of the item, in ascending key order, each with its owning
  `DomainPackageRef` identity (QSpec FR-353-AC-3). A refused item returns no
  bindings.
- A bound item SHALL return every binding it references.
- Each item's result depends only on the checked package and that item.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-307-AC-1 | With only the ConfigVersion object type bound, a request for a clause item that reads only ConfigVersion fields and an item whose extent domain ranges over the unbound ConfigVersion population returns the first item with its one `ObjectBinding` and refuses the second `missing_declaration`/`missing-name`, naming the population's key and its `DomainPackageRef` identity, with no bindings. | Test (TC-806) |
| FR-307-AC-2 | An item that references two unbound elements refuses naming both, in ascending key order, each with its owning `DomainPackageRef` identity. | Test (TC-806) |
| FR-307-AC-3 | A requested occurrence key that names no requirement record refuses `missing_declaration`/`missing-name` naming the key, and the other requested items' results are unchanged. | Test (TC-806) |
| FR-307-AC-4 | An item reading a member that `Base` declares through a receiver of static type `Sub` references `Sub`: with only `Base` bound it refuses naming `Sub`, and with `Sub` bound it returns `Sub`'s binding. | Test (TC-807) |
| FR-307-AC-5 | A frame item and a postcondition item of `attemptUpdate` each reference `OperationKey { ConfigVersion, attemptUpdate }`: with the frame bound each returns that `FrameBinding`, and without it each refuses naming that key. | Test (TC-807) |
| FR-307-AC-6 | Over a QSL fixture checked package, exporting one bound and one refused item gives exactly one bound result, carrying that item and its bindings, and one refused result, naming the refused item with its `missing_declaration`/`missing-name` refusal; the bound results are the items a driver's QSpec FR-331 request holds. The driver's request and report are verified end to end in quire-integration. | Test (TC-808) |

## Dependencies

- **Upstream:** QSpec FR-353 owns the per-item unbound refusal; QSpec FR-331
  owns the CG request; ADR-017 AR-4 and AR-6 fix the export, its referenced
  elements and what CG receives; ADR-011 T-13 names the driver.
- **Downstream:** CG's Kani and Verus generators receive only bound items and
  keep their own unbound guard (ADR-017 AR-4).

## References

- ADR-017 §3 AR-4 ("Implementation export", "Referenced elements",
  "Refusals") and AR-6.
- Linear QSL-388 (specification), QSL-36 (implementation).

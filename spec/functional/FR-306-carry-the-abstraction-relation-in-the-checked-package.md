---
id: FR-306
title: "Carry the abstraction relation in the checked package"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-031
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-011
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-017
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-057
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-290
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-353
    type: depends_on
---
# FR-306: Carry the abstraction relation in the checked package

## Description

When the S4 emitter writes a checked package whose unit declares an
abstraction relation, the emitter SHALL write the `CheckedAbstractionRelation`
as one `quire.checked-package/v2` node, so the relation enters the
`package_id`, and the checked package SHALL be the relation's only authority.

## Inputs

- The `CheckedAbstractionRelation` of
  [FR-304](FR-304-check-an-authored-abstraction-relation.md) in the
  in-process `CheckedPackage`.

## Outputs

The v2 relation node, carrying every binding with its key and value, in the
emitted package.

## Behavior

### One authority

The relation SHALL be emitted as the v2 node whose tag, form and body QSpec
spells for the QSpec FR-353 relation (QSpec FR-451), carrying ADR-017 AR-2's
keys and values. Its node ids enter the `package_id` (ADR-013 O-02). Kani and
Verus generation read the relation only from the v2 bytes (ADR-011 FB-05).

### Rebinding is a new package

Changing any binding changes the relation node and so the `package_id`.
Every generated contract carries the `package_id` it was generated from
(ADR-013 O-25), so a rebinding is a new relation revision and never
reinterprets a contract generated from the earlier package (QSpec
FR-353-AC-5).

### The relation is a premise, not a claim

The relation SHALL record no requirement record (QSpec FR-290: it has no
capability kind; [FR-057](FR-057-admit-shared-capability-kinds.md)-AC-10).
Each claim that references bound elements keeps its own kind
(`operation-contract` or `value-validity`) and its extent, and `route`
routes it as ADR-012 §7.2 states. The relation is not an S6a input: a
counterexample from a claim that used it replays as clause expressions
through S6a (ADR-012 §8; ADR-017 AR-5).

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-306-AC-1 | A unit with FR-304-AC-1's relation emits one relation node, and reading the emitted v2 bytes yields the same three keys and binding values that the in-process `CheckedAbstractionRelation` holds. | Test (TC-804) |
| FR-306-AC-2 | Recompiling the unit unchanged gives the same `package_id`; changing one `ObjectBinding` field's `RustField` gives a different `package_id`. | Test (TC-804) |
| FR-306-AC-3 | The unit's requirement records and their `route` results are equal with and without its relation declaration. | Test (TC-805) |
| FR-306-AC-4 | A clause run of the unit's ConfigVersion clauses through the spine run entry gives the same disposition with and without its relation declaration. | Test (TC-805) |

## Dependencies

- **Upstream:** QSpec FR-353 and FR-290 own the relation's semantics and its
  absence of a capability kind; QSpec FR-451 spells the v2 node; ADR-017
  AR-4 and AR-5 fix the authority and the dispatch rules.
- **Downstream:** [FR-307](FR-307-export-the-bindings-each-item-references.md)
  exports bindings from the checked package; CG's Kani and Verus generators
  read the v2 node.

## References

- ADR-017 §3 AR-4 ("One authority", "Relation revision") and AR-5.
- QSpec FR-451 (specification ticket STD-116, Q-2).
- Linear QSL-388 (specification), QSL-36 (implementation).

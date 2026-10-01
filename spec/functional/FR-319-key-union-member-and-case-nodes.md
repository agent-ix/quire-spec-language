---
id: FR-319
title: "Key union, member, construction and case nodes and carry member identity as a VariantId"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-032
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-012
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-316
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-317
    type: traces_to
  - target: ix://agent-ix/quire-spec-language/FR-318
    type: traces_to
  - target: ix://agent-ix/quire-specification/FR-143
    type: depends_on
---
# FR-319: Key union, member, construction and case nodes and carry member identity as a VariantId

## Description

When S3 lowers a checked union declaration, union construction or `case`
(FR-093), the compiler SHALL key each node by its content under the
existing identity schemes (ADR-012 §16.3), SHALL give each union member the
identity (union node key, member identifier) of QSpec FR-143, and SHALL
carry that identity in the kernel as a `VariantId` whose bytes are the
member's node key under QSpec's union member preimage, retyped with no fresh
computation, exactly as an enum member's `VariantId` is its
`quire.enum-member-node/v1` key retyped (`quire_semantic_value::enumeration`).

## Inputs

- Checked union declarations, constructions and `case` nodes; the declaring
  source's `SourceOwner`.

## Outputs

- Node ids for the union type node, construction nodes and `case` nodes;
  each member's identity and `VariantId`; one occurrence per source
  occurrence (node id, role, ordinal).

## Behavior

- The union declaration's id SHALL be the checked node id of its
  (`composite_type`, `union`) node, keyed by QSL's
  `quire.structural-node/v1` preimage with the declaring source's
  `SourceOwner`, as record and tuple declarations are (FR-092).
- The `check` stage SHALL mint a member's node key over QSpec's union member
  preimage (References).
- The compiler SHALL use the member node key's bytes, retyped, as the
  member's `VariantId`.
- The `quire-semantic-value` layer SHALL take member keys as given, minting
  none.
- The `check` stage SHALL build the `VariantId` → member resolution once, as
  an index from `VariantId` to (union, member, declared position), in the
  same manner as the enum member index; no other stage re-derives it.
- The compiler SHALL key a `case` node and a construction node as undeclared
  nodes, by content with no owner (FR-092, FR-093).
- S3 lowering SHALL lower a `case` node's arms in `U`'s declared member
  order, so the arm order in source does not reach its preimage.
- The kernel SHALL compare union values by QSpec FR-143's union equality,
  over the pair (active member identity, payload values in declared position
  order).
- A `VariantId` of another union, or of an enum, SHALL never select an arm.

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-319-AC-1 | Compiling one unit twice, and compiling it with whitespace and comments changed and the arms of one `case` reordered, gives the same union node id, the same `VariantId` for every member, the same `case` node id and the same `package_id`. Renaming a member, adding a member or changing a payload type changes the union's node id and the id of every value, expression and function node that names the union. | Test (TC-826) |
| FR-319-AC-2 | For each member of `Shape`, the `VariantId` computed at check time equals the one computed at argument admission for the same member, and equals that member's node key bytes. Two unions `A { X }` and `B { X }` give two different `VariantId`s for `X`; an enum `E { X }` gives a third. A `case` over `A` evaluated with a scrutinee whose `VariantId` belongs to `B` or `E` selects no arm (it is refused at admission, FR-321). | Test (TC-826) |
| FR-319-AC-3 | `Shape::Rect(2, 3) = Shape::Rect(2, 3)` is `true`, `Shape::Rect(2, 3) = Shape::Rect(3, 2)` is `false`, and `Shape::Empty = Shape::Circle(0)` is `false`. With `union A { X(Integer) }` and `union B { X(Integer) }`, comparing `A::X(1)` with `B::X(1)` is refused by type checking (QSpec TC-262's equality cases each give TC-262's verdict). | Test (TC-826) |

## Dependencies

- QSpec FR-143 ("Declarations and identity"), FR-143-AC-12; FR-141 (the enum
  member preimage, the precedent).
- FR-092, FR-093 (keys and lowering); ADR-013 O-04, O-06, O-07, O-14, C-26,
  C-30.

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.3 (SC-R2, SC-R4, SC-R5).
- Union member identity preimage (SC-G3): STD-142.

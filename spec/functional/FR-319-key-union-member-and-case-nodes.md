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
member's key under QSpec FR-441's `quire.union-member-node/v1` preimage, in
domain `quire.checked-semantic-node/v1`, retyped with no fresh computation, exactly as an enum member's `VariantId` is its
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
  (`composite_type`, `union`) node, keyed by the node identity QSpec FR-440
  and FR-322 give a source-declared union node.
- The `check` stage SHALL mint a member's key over QSpec FR-441's preimage
  `{version: "quire.union-member-node/v1", declaration_node_id, member}`,
  where `declaration_node_id` is the union declaration's node id, in domain
  `quire.checked-semantic-node/v1`.
- The compiler SHALL use the member node key's bytes, retyped, as the
  member's `VariantId`.
- The `quire-semantic-value` layer SHALL take member keys as given, minting
  none.
- The `check` stage SHALL build the `VariantId` → member resolution once, as
  an index from `VariantId` to (union, member, declared position), in the
  same manner as the enum member index; no other stage re-derives it.
- The compiler SHALL key a `case` node and a construction (`union_value`)
  node as undeclared nodes, by content with no owner, under the
  `quire.application-node/v1` preimage (QSpec FR-440; FR-092, FR-093).
  For `union_value`, this selection applies even when its body contains no
  `application` term. The preimage SHALL contain exactly
  `{version: "quire.application-node/v1", node_tag: "value",
  semantic_form: "union_value", semantic_type, declaration: null,
  recursion, body}`, with `semantic_type` the union node id and `recursion`
  as QSpec FR-322 defines it. The body SHALL remain QSpec FR-440's
  `aggregate` of one member-named `binding` over the payload `aggregate`:
  `Empty` holds an empty aggregate, and `Rect` holds its two payload terms
  in position order. Payloads lowered to separate nodes are `reference`
  terms, as [FR-093](FR-093-lower-checked-value-expressions-to-fr-322-terms.md)
  spells a present `record_value` slot. This selection introduces no
  construction application, operator or operation identity.
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
| FR-319-AC-2 | For each member of `Shape`, the `VariantId` computed at check time equals the one computed at argument admission for the same member, and equals that member's FR-441 member key bytes (QSpec FR-441-AC-1's vectors give the expected digests for their inputs). Two unions `A { X }` and `B { X }` give two different `VariantId`s for `X`; an enum `E { X }` gives a third. A `case` over `A` evaluated with a scrutinee whose `VariantId` belongs to `B` or `E` selects no arm (it is refused at admission, FR-321). | Test (TC-826) |
| FR-319-AC-3 | `Shape::Rect(2, 3) = Shape::Rect(2, 3)` is `true`, `Shape::Rect(2, 3) = Shape::Rect(3, 2)` is `false`, and `Shape::Empty = Shape::Circle(0)` is `false`. With `union A { X(Integer) }` and `union B { X(Integer) }`, comparing `A::X(1)` with `B::X(1)` is refused by type checking (QSpec TC-262's equality cases each give TC-262's verdict). | Test (TC-826) |
| FR-319-AC-4 | For fixed union and payload node ids, the nullary `Shape::Empty` and a `Shape::Rect` whose two payload terms are references each produce independently fixed RFC 8785 preimage bytes and SHA-256 node keys under `quire.application-node/v1`. Neither body contains an application, both have `declaration: null`, neither preimage has `owner`, and a node outside a recursion group has `recursion: null`. The expected bytes and keys are fixed independently of the producer and reader under test. | Test |
| FR-319-AC-5 | The same fixed `record_value` and `tuple_value` inputs whose bodies contain only bindings, references and literals retain their existing `quire.structural-node/v1` bytes and keys. Reordering the source arms of the `case` in AC-1 still gives the same declared-order body, node id and `package_id`. | Test |

## Dependencies

- QSpec FR-143 ("Declarations and identity"), FR-143-AC-12; FR-440 (union
  node identity); FR-441 (member key); FR-141 (the enum member preimage, the
  precedent).
- FR-092, FR-093 (keys and lowering); ADR-013 O-04, O-06, O-07, O-14, C-26,
  C-30.

## References

- Owning ticket: QSL-383. Design: ADR-012 §16.3 (SC-R2, SC-R4, SC-R5).
- Union member identity preimage (SC-G3): QSpec FR-441 (specification
  ticket STD-142).
- Construction preimage selector reconciliation: QSL-679; the matching
  QSpec FR-440 and FR-322 selectors and independent producer/reader vector
  qualification are required before this slice is claimed complete.

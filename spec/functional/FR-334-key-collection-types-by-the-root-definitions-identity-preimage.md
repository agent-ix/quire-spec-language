---
id: FR-334
title: "Key collection and population types by the root definitions' identity preimage"
type: FR
relationships:
  - target: ix://agent-ix/quire-spec-language/US-033
    type: implements
  - target: ix://agent-ix/quire-spec-language/ADR-013
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/ADR-014
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-097
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-111
    type: depends_on
  - target: ix://agent-ix/quire-spec-language/FR-333
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-144
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-334: Key collection and population types by the root definitions' identity preimage

## Description

S4 SHALL key every collection type node and every population type node by
the collection-type identity preimage that QSpec's root definitions
`quire.value.complete/v1` and `quire.model.complete/v1` state at `1-draft.2`,
in which bound presence and bound values are part of type identity (ADR-014
N-1, N-2; QSpec FR-144-AC-9, AC-13; FR-153-AC-9). QSL reads those
definitions from the definition bundle it links (FR-111) and holds no copy
of them. Every package with a collection type gets the node ids and
`package_id` this preimage gives.

## Use case

Two teams exchange packages, one declaring `Set<Account>` and one
`Set<Account>[0, 100]`. The two types must never be confused by a consumer
that compares node identities, and a package's identity must follow from
its content under the one rule QSpec publishes.

## Inputs

- Checked types (FR-333) and the linked definition bundle (FR-111).

## Outputs

- v2 type nodes and their node keys (FR-092), and the package's
  `package_id`.

## Behavior

- An unbounded collection type SHALL lower to its composite collection node
  alone; a bounded collection type SHALL lower to its composite node with a
  `collection_bounds` member holding `minimum` and `maximum`. The two SHALL
  have different node keys for every kind and element type.
- A population type with no maximum SHALL lower to its own population node
  with no maximum member, and a population with maximum `N` to its
  population node with `N`. Each SHALL have a node key distinct from every
  `Set<Reference<T>>` node, bounded or unbounded (FR-097-AC-8).
- The preimage of each such node SHALL be the one the linked root
  definitions state, applied to the node's content; QSL SHALL take the
  preimage's member names and order from those definitions as FR-111 links
  them.
- A package's `package_id` SHALL follow from these node keys, so changing
  only a bound's presence or value changes the `package_id`.
- Bounded corpora SHALL keep their source spelling and meaning; their
  expected node keys and `package_id`s are those this preimage gives
  (ADR-014 N-2).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-334-AC-1 | A unit whose only difference from another is `s: Set<Int>` against `s: Set<Int>[0, 18446744073709551615]` emits a different node key for `s`'s type and a different `package_id`; the unbounded node has no `collection_bounds` member and the bounded node has one with `minimum` 0 and `maximum` 18446744073709551615. | Test (TC-844) |
| FR-334-AC-2 | `p: Population<Account>` lowers to a population node with no maximum, whose node key differs from those of `Population<Account>[4]` and of an unbounded `Set<Reference<Account>>`; it no longer refuses with `UnrepresentableBound`. | Test (TC-844) |
| FR-334-AC-3 | Compiling the same bounded unit twice gives the same node keys and `package_id`; the compiled package's definition selections name the root definitions the linked bundle supplies. | Test (TC-844) |

## Dependencies

- ADR-014 §9 N-1 to N-3, §11 (QSL-42 interfaces); ADR-013 O-04, O-09.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md) (node keys),
  [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md) AC-7
  and AC-8, [FR-111](FR-111-link-a-complete-v1-definition-bundle.md)
  (definition bundle), [FR-333](FR-333-read-an-optional-collection-bound-and-population-maximum.md).
- QSpec FR-144-AC-9 and AC-13, FR-153-AC-9, and the root definitions.

## References

- Linear QSL-385 (spec ticket); QSL-42 (implementation).
- QSpec half: QSpec FR-144 (merged).

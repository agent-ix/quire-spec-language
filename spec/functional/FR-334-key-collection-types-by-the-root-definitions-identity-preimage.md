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
  - target: ix://agent-ix/quire-spec-language/FR-333
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-144
    type: depends_on
  - target: ix://agent-ix/quire-specification/FR-153
    type: depends_on
---
# FR-334: Key collection and population types by the root definitions' identity preimage

## Description

QSL's S4 emitter SHALL emit collection and population type nodes whose
identity conforms to QSpec's complete value and model type rules (FR-144-AC-9
and AC-13; FR-153-AC-9). The emitter computes structural node keys with QSL's own code.
QSL's conformance tests compare those keys with QSpec's published node
identity vectors in FR-322. Packages are compared by their emitted JSON values,
including those structural node ids, under FR-322's package equality rule.

## Use case

Two teams exchange packages, one declaring `Set<Account>` and one
`Set<Account>[0, 100]`. The two types must never be confused by a consumer
that compares node identities, and a package's identity must follow from
its name and contents under the one rule QSpec publishes.

## Inputs

- Checked types (FR-333).

## Outputs

- v2 type nodes and their structural node keys (FR-092), and the emitted
  package value.

## Behavior

- An unbounded collection type SHALL lower to its composite collection node
  alone; a bounded collection type SHALL lower to its composite node with a
  `collection_bounds` member holding `min` and `max`. The two SHALL have
  different node keys for every collection kind and element type (QSpec
  FR-144-AC-9 and AC-13; FR-322).
- A population type with no maximum SHALL lower to its own population node
  with no maximum member, and a population with maximum `N` to its own
  population node carrying `N`. Each SHALL have a node key distinct from
  every bounded or unbounded `Set<Reference<T>>` node (FR-097-AC-8; QSpec
  FR-153-AC-9; FR-322).
- QSL SHALL compute the node keys using its own code, conforming to QSpec
  FR-322.
- Emitted packages SHALL compare structurally under QSpec FR-322, so changing
  only a bound's presence or value changes the emitted package value.
- The emitter SHALL preserve a bounded corpus's source spelling and meaning;
  the test compares its output with the published QSpec vectors (FR-334-AC-4).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-334-AC-1 | A unit whose only difference from another is `s: Set<Integer>` against `s: Set<Integer>[0, 18446744073709551615]` emits a different structural node key for `s`'s type and a structurally unequal package value; the unbounded node has no `collection_bounds` member and the bounded node has one with `min` 0 and `max` 18446744073709551615. | Test (TC-844) |
| FR-334-AC-2 | `p: Population<Account>` lowers to a population node with no maximum, whose node key differs from those of `Population<Account>[4]` and of an unbounded `Set<Reference<Account>>`; it lowers with no refusal. | Test (TC-844) |
| FR-334-AC-3 | Compiling the same bounded unit twice gives the same structural node keys and structurally equal emitted package values. | Test (TC-844) |
| FR-334-AC-4 | Conformance: for each collection and population type vector in QSpec FR-322, read in place from the QSpec repository by the test, QSL's node key equals the vector's expected key; the vectors exercise collection and population types covered by FR-144-AC-9, AC-13 and FR-153-AC-9. | Test (TC-844) |

## Dependencies

- ADR-014 §9 N-1 to N-3, §11; ADR-013 O-04, O-09.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md) (node keys),
  [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md) AC-7
  and AC-8, [FR-333](FR-333-read-an-optional-collection-bound-and-population-maximum.md).
- QSpec FR-322 defines the node identity vectors and node-key preimage used
  by QSL's conformance checks.
- QSpec FR-144-AC-9 and AC-13, FR-153-AC-9, and the root definitions.

## References

- Structural-identity erratum: QSpec FR-322, Properties, states: "two checked
  packages are the same package exactly when their JSON values are equal member
  by member and, for arrays, element by element in order." It also states:
  "No identifier, hash, digest or seal of a package exists."
- Linear QSL-385 (spec ticket); QSL-42 (implementation).
- QSpec half: QSpec FR-144 (merged).

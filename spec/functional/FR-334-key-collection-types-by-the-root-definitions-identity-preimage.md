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
identity conforms to QSpec's root definitions for complete value and model
types (FR-144-AC-9 and AC-13; FR-153-AC-9). The emitter computes node keys
with QSL's own code. QSL's conformance tests compare those keys with the
published QSpec vectors. Every package with a collection type gets the node
ids and `package_id` determined by those keys.

## Use case

Two teams exchange packages, one declaring `Set<Account>` and one
`Set<Account>[0, 100]`. The two types must never be confused by a consumer
that compares node identities, and a package's identity must follow from
its content under the one rule QSpec publishes.

## Inputs

- Checked types (FR-333).

## Outputs

- v2 type nodes and their node keys (FR-092), and the package's
  `package_id`.

## Behavior

- The S4 emitter SHALL emit collection and population nodes in conformance
  with QSpec's root definitions (FR-144-AC-9 and AC-13; FR-153-AC-9),
  computing each node key with QSL's own code. The emitter's bound-sensitive
  and population-node behavior is specified by FR-334-AC-1 and AC-2.
- A package's `package_id` SHALL follow from these node keys, so changing
  only a bound's presence or value changes the `package_id`.
- The emitter SHALL preserve a bounded corpus's source spelling and meaning;
  the test compares its output with the published QSpec vectors (FR-334-AC-4).

## Acceptance Criteria

| ID | Criteria | Verification |
|----|----------|--------------|
| FR-334-AC-1 | A unit whose only difference from another is `s: Set<Integer>` against `s: Set<Integer>[0, 18446744073709551615]` emits a different node key for `s`'s type and a different `package_id`; the unbounded node has no `collection_bounds` member and the bounded node has one with `min` 0 and `max` 18446744073709551615. | Test (TC-844) |
| FR-334-AC-2 | `p: Population<Account>` lowers to a population node with no maximum, whose node key differs from those of `Population<Account>[4]` and of an unbounded `Set<Reference<Account>>`; it lowers with no refusal. | Test (TC-844) |
| FR-334-AC-3 | Compiling the same bounded unit twice gives the same node keys and `package_id`. | Test (TC-844) |
| FR-334-AC-4 | Conformance: for each collection and population type vector published by QSpec under FR-144-AC-9, AC-13 and FR-153-AC-9, QSL's node key equals the vector's expected key. | Test (TC-844) |

## Dependencies

- ADR-014 §9 N-1 to N-3, §11; ADR-013 O-04, O-09.
- [FR-092](FR-092-key-type-parameter-and-declared-nodes.md) (node keys),
  [FR-097](FR-097-classify-claim-extent-and-write-bounded-requests.md) AC-7
  and AC-8, [FR-333](FR-333-read-an-optional-collection-bound-and-population-maximum.md).
- QSpec FR-144-AC-9 and AC-13, FR-153-AC-9, and the root definitions.

## References

- Linear QSL-385 (spec ticket); QSL-42 (implementation).
- QSpec half: QSpec FR-144 (merged).

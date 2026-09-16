---
id: FR-144
title: "Preserve complete collection-kind semantics"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-008
    type: references
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
---

# FR-144: Preserve complete collection-kind semantics

## Description

When constructing or comparing a collection, the evaluator SHALL preserve the
selected sequence, set, bag or ordered-set algebra.

## Inputs

Element type, collection kind, declared cardinality bound and occurrences.

## Outputs

A typed bounded collection/equality result or refusal.

## Behavior

Sequences preserve order and duplicates. Sets preserve membership without
duplicates and expose no semantic iteration order. Bags preserve multiplicity
without semantic order. Ordered sets preserve first-occurrence order and
uniqueness. Equality follows those rules and all materialization checks the
declared maximum before allocation.

## Collection-kind table

| Kind | Occurrence model | Equality | Canonical representation |
| --- | --- | --- | --- |
| `Sequence<T>` | ordered occurrences; duplicates retained | equal length and pairwise element equality | occurrence order |
| `Set<T>` | unique members; no semantic order | identical member set | ascending type-owned total key |
| `Bag<T>` | member-to-positive-multiplicity map; no semantic order | identical members and multiplicities | ascending key plus multiplicity |
| `OrderedSet<T>` | first-occurrence order plus uniqueness | identical ordered member sequence | semantic occurrence order |

Set/bag canonical ordering exists only when `T` supplies a total canonical key;
otherwise canonical serialization refuses rather than using insertion or hash
order. Bounds are inclusive (`min..max`), checked before materialization, and
count occurrences for sequences/bags and members for sets/ordered sets.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-144-AC-1 | Permuting a set or bag does not change equality, while permuting a sequence or ordered set can. | Test (TC-189) |
| FR-144-AC-2 | Duplicate insertion is preserved by sequence/bag and eliminated by set/ordered set according to their defined order rule. | Test (TC-189) |
| FR-144-AC-3 | Missing or exceeded cardinality bounds refuse without partial materialization. | Test (TC-189) |
| FR-144-AC-4 | Set/bag insertion order never affects equality or canonical bytes; lack of a total element key refuses canonical serialization. | Test (TC-189) |
| FR-144-AC-5 | Bound counting follows occurrence versus unique-member semantics for each collection kind. | Test (TC-189) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

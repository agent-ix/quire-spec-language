---
id: FR-145
title: "Convert and query every collection kind explicitly"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-041
    type: references
  - target: ix://agent-ix/quire-specification/FR-144
    type: references
---

# FR-145: Convert and query every collection kind explicitly

## Description

When applying a collection conversion or query, the evaluator SHALL derive the
result kind, order, uniqueness, multiplicity and bound from the selected
operation contract.

## Inputs

Bounded collection, typed pure function/predicate, target kind and result bound.

## Outputs

A typed collection/scalar result, explicit loss record, refusal or incomplete
outcome.

## Behavior

`map` and `collect` are one operation. Filter preserves the source kind where
closed under filtering. Flatten defines source/element kind combinations and
combines occurrence order/multiplicity without implicit deduplication. Fold and
reduce select a catalog operation, empty identity, associativity requirement and
intermediate domain. Conversions that lose order or duplicates report that loss.

## Operation matrix

| Operation | Sequence | Set | Bag | Ordered set |
| --- | --- | --- | --- | --- |
| `map` / `collect` | Preserve source occurrence order and multiplicity. | Produce a set and coalesce equal results. | Preserve result multiplicity. | Preserve first-result order and coalesce later equal results. |
| `filter` | Preserve surviving occurrence order and multiplicity. | Preserve the set of surviving values. | Preserve the multiplicity of surviving values. | Preserve the order of surviving first occurrences. |
| `flatten` | Concatenate nested semantic occurrences in outer then inner order. | Union nested values. | Sum nested multiplicities. | Preserve first occurrence while traversing outer then inner order. |
| `count` | Count matching occurrences. | Return zero or one for a value. | Return the stored multiplicity. | Return zero or one for a value. |
| `fold` | Traverse occurrence order from the declared identity. | Require a commutative and associative function. | Require a commutative and associative function and apply it once per occurrence. | Traverse declared order from the declared identity. |
| `reduce` | Traverse occurrence order; empty input is undefined. | Require a commutative and associative function; empty input is undefined. | Require a commutative and associative function, apply once per occurrence and refuse empty input. | Traverse declared order and refuse empty input. |

`map` and `collect` are one semantic operation. `flatMap` is exactly
`flatten(map(source, function))`; it is not a separately extensible operator.
An explicit target-kind conversion records each discarded property from
`order`, `uniqueness` and `multiplicity`. An empty fold returns only its declared
identity. An empty reduction has no value and returns a located undefined
outcome.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-145-AC-1 | Map/filter/flatten over each collection kind returns the specified kind and deterministic semantic occurrences. | Test (TC-190) |
| FR-145-AC-2 | Fold on an empty collection uses only a declared identity; a reduction without one refuses. | Test (TC-190) |
| FR-145-AC-3 | A conversion that loses order or multiplicity requires explicit acceptance and records the exact loss. | Test (TC-190) |
| FR-145-AC-4 | The operation matrix determines result kind, occurrence order, uniqueness and multiplicity for every source kind. | Test (TC-190) |
| FR-145-AC-5 | Set and bag folds or reductions refuse a function that is not both commutative and associative. | Test (TC-190) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

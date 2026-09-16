---
id: FR-143
title: "Evaluate records, tuples and finite recursive values"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/FR-009
    type: references
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
---

# FR-143: Evaluate records, tuples and finite recursive values

## Description

When checking a record, tuple or recursive value, the semantic kernel SHALL
apply its declared field order, types, structural equality and finite-value
construction rules.

## Inputs

Named record or positional tuple definitions, recursive constructors, values
and traversal limits.

## Outputs

A typed finite value/equality result, refusal or incomplete traversal outcome.

## Behavior

Record fields use declaration-owned identities; tuple positions are ordered.
Structural equality recursively compares the same declared value type and does
not compare object-reference targets structurally. Recursive definitions are
allowed, but each runtime value must be finite and cycles require explicit
object identity rather than value containment.

## Composite-value contract

A record value contains exactly one value or explicit absence/null state for
each declaration-owned field, ordered by field declaration for canonical
serialization only. A tuple contains exactly its declared arity in positional
order. Missing required fields, extra fields, wrong arity and conflated
absence/null refuse construction before evaluation.

A recursive value definition is legal when every recursion edge crosses a
named constructor field. A runtime value is a finite constructor tree/DAG; a
back-edge in value containment refuses. Sharing an immutable contained value
does not create object identity. Cycles are representable only by explicit
typed `Reference<T>` values resolved in a closed object environment.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-143-AC-1 | Equal finite values of the same declared record/tuple type compare structurally equal. | Test (TC-188) |
| FR-143-AC-2 | Equal field shapes from different declared types do not become interchangeable. | Test (TC-188) |
| FR-143-AC-3 | Cyclic value containment refuses, while bounded recursive construction and object-reference cycles remain distinct. | Test (TC-188) |
| FR-143-AC-4 | Missing/extra record fields, wrong tuple arity and absence/null substitution each refuse at the originating component. | Test (TC-188) |
| FR-143-AC-5 | Canonical field ordering does not make equal-shaped values of different declared record types equal. | Test (TC-188) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

---
id: FR-149
title: "Apply the complete typed equality matrix"
type: FR
relationships:
  - target: ix://agent-ix/quire-specification/US-010
    type: implements
  - target: ix://agent-ix/quire-specification/AD-005
    type: references
  - target: ix://agent-ix/quire-specification/FR-009
    type: references
  - target: ix://agent-ix/quire-specification/FR-143
    type: references
  - target: ix://agent-ix/quire-specification/FR-144
    type: references
  - target: ix://agent-ix/quire-specification/FR-140
    type: references
  - target: ix://agent-ix/quire-specification/FR-141
    type: references
  - target: ix://agent-ix/quire-specification/FR-142
    type: references
  - target: ix://agent-ix/quire-specification/FR-148
    type: references
---

# FR-149: Apply the complete typed equality matrix

## Description

When evaluating equality, the semantic kernel SHALL select exactly the equality
relation declared for the operands' common complete-V1 type.

## Inputs

Two typed values and the exact selected equality operation/profile.

## Outputs

A completed Boolean, or an undefined, refused or incomplete evaluator outcome.
Static type-check refusal and I13 dispositions remain staged separately.

## Behavior

The matrix distinguishes scalar numeric rules, enum declaration identity, text
profile, structural records/tuples, collection algebra, object-reference
identity and IEEE numeric/total/bitwise relations. No structural resemblance,
implicit conversion or local-name equality creates a common type.

## Equality matrix

| Value kind | Equality relation |
| --- | --- |
| Boolean | identical truth value |
| Integer, rational and decimal | exact mathematical value after an explicitly declared lossless common-type conversion |
| Quantity | identical dimension and exact equality after explicit conversion to the selected canonical unit |
| Text | the FR-141 relation under the same pinned profile: normalized scalar sequence for scalar profiles and original UTF-8 bytes for `binary-utf8` |
| Enumeration | the same qualified enumeration declaration and identical case; different declarations have no implicit common type |
| Presence, absence and null | same declared type and state; two absent values are equal, two explicit null values are equal, and two present values recursively compare their payload; absent and explicit null are distinct |
| Record and tuple | same declared structural type and recursively equal fields in declaration order |
| Sequence | equal length and recursively equal values at every index |
| Set | recursively equal members independent of iteration order |
| Bag | recursively equal members with equal multiplicity |
| Ordered set | recursively equal unique values in the same occurrence order |
| Object reference | identical qualified object identity; referenced state is not inspected |
| IEEE binary32/binary64 | the explicitly selected numeric equality, `totalOrder` equality or bit identity from FR-148 |

Operands of different types are comparable only after an explicit conversion
declares a common type. If conversion is lossy, undefined or absent, type
checking returns `refused { code: ill_typed }`; it never produces Boolean
`false`. Conversion creates a comparison value but
does not rewrite the source value's declaration or object identity.

Before descent, `equality.plan` computes the complete semantic occurrence-pair
plan and reserves its pair/work/result allowance atomically. Records, tuples,
sequences, ordered sets, options and recursive variants use declaration/index
depth-first order. Sets and bags with a total canonical element key use that
key. Without such a key, equality remains defined: the plan contains the full
left-by-right element cross-product (including recursive subpairs), evaluates
all pairs without early exit and decides set equality by a bijection or bag
equality by equal multiplicity classes. Its pair count and Boolean are
independent of iteration order; canonical serialization may still refuse under
FR-144. Once the plan is reserved, an implementation's evaluation order is
unobservable.

Every planned semantic occurrence-path pair receives one `equality.pair`
charge. Immutable DAG sharing never reduces that normative charge: shared and
duplicated representations of the same occurrence tree have identical
accounting under `quire.value.accounting/v1`. An implementation may memoize as
an unobservable optimization but cannot change charges, outcome or identity.
Object references are terminal and
do not descend into referenced state. If evaluation required to construct a nested operand returns
undefined, refused or incomplete, no completed outer value exists and the outer
expression returns that same disposition. If comparison traversal exhausts its
bound, it returns incomplete and never substitutes `false`. Scalar equality primitives from FR-140 through
FR-148 are reusable inputs; they do not by themselves satisfy the composite,
collection and object-reference rows of this requirement.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-149-AC-1 | Every complete-V1 value kind has positive, negative and cross-type equality vectors. | Test (TC-194) |
| FR-149-AC-2 | Records compare structurally while object references compare qualified identity even when referenced fields match. | Test (TC-194) |
| FR-149-AC-3 | Values without a selected common equality relation refuse rather than returning false. | Test (TC-194) |
| FR-149-AC-4 | Positive, negative and cross-type vectors exercise every row of the equality matrix. | Test (TC-194) |
| FR-149-AC-5 | Explicit conversion does not mutate either source value's type, declaration identity or object-reference identity. | Test (TC-194) |
| FR-149-AC-6 | Absence and explicit null remain distinct, while equal present values recursively compare their payload under the declared option/field type. | Test (TC-194) |
| FR-149-AC-7 | A nested operand-construction non-result or exhausted recursive comparison propagates its typed disposition before any outer value/Boolean is produced. | Test (TC-194) |
| FR-149-AC-8 | Set/bag equality without a total canonical key uses the full order-independent cross-product plan; lack of a serialization key does not change equality or accounting. | Test (TC-194) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

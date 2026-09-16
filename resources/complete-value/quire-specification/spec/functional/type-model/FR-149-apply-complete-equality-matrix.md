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
  - target: ix://agent-ix/quire-specification/FR-204
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
| Integer, rational and decimal | exact mathematical value; operands of different numeric types first require an admitted [equality conversion](#equality-conversions) |
| Quantity | identical unit and exact value equality; a top-level quantity equality expression uses the FR-142 root-value comparison schedule, while a quantity leaf inside a composite equality plan compares by exact value in its identical unit and charges only its `equality.pair`; distinct units require an explicit prior FR-142 conversion |
| Text | the FR-141 relation under the same pinned profile: normalized scalar sequence for scalar profiles and original UTF-8 bytes for `binary-utf8` |
| Enumeration | the same qualified enumeration declaration and identical case; different declarations have no implicit common type |
| Presence, absence and null | same declared type and state; two absent values are equal, two explicit null values are equal, and two present values recursively compare their payload; absent and explicit null are distinct |
| Record and tuple | same FR-143 declaration key and recursively equal fields in declaration order or positions in order |
| Sequence | equal length and recursively equal values at every index |
| Set | equal member count and recursively equal members independent of iteration order |
| Bag | equal occurrence count and recursively equal members with equal multiplicity |
| Ordered set | recursively equal unique values in the same occurrence order |
| Object reference | identical FR-143 identity triple (universe, object-type declaration, object identity); referenced state and observation are not inspected; different universes return `refused { code: foreign_reference }` |
| IEEE binary32/binary64 | the explicitly selected FR-148 `numericEqual`, `totalOrder` equivalence or `bitIdentical` intrinsic; the grammar's `=` and `!=` select none of them and are `ill_typed` for any operand type containing `Float32` or `Float64` at any depth |

Operands of different types are comparable only after an explicit conversion
declares a common type. If conversion is lossy, undefined or absent, type
checking returns `refused { code: ill_typed }`; it never produces Boolean
`false`. Conversion creates a comparison value but
does not rewrite the source value's declaration or object identity.

## Selected schedules

The grammar's comparison operators `=` and `!=` select one schedule from the
operands' common type, after any
admitted conversion; `!=` uses the same schedule and retains the negated
Boolean. A top-level text operand pair uses the FR-141 text schedule, an
enumeration pair `enum.*` and a quantity pair the FR-142 top-level comparison
schedule. Every other common type (Boolean, `Integer`, `Int[..]`, `Rational[..]`,
`Decimal[..]`, `Reference<T>`, `Option<T>`, record, tuple and every collection
kind) uses the equality schedule `equality.plan-form`, `equality.plan`, one
`equality.pair` per planned pair and `equality.result-retain`.
`equality.plan-form` charges `occ(left) + occ(right)` work units before plan
formation walks either operand. A top-level Boolean, numeric or reference
comparison is therefore a one-pair plan: two plan-formation work units, then
`value_occurrences = 1`, three more work units and one result unit, five work
units in all, with no `decimal.*`, `integer-*` or other family charge. Inside a plan, every leaf, text, enumeration and quantity leaves
included, charges only its planned pair.

## Equality conversions

An operand of `=` or `!=` whose expression, ignoring parentheses, is
`convert<T>(e)` is an equality conversion from the static type `S` of `e`. It is
admitted only when (`S`, `T`) is in the closed table below and the other operand
then has type `T`. Every other equality conversion is
`refused { code: ill_typed }` before any charge, even when the particular value
would fit. The same `convert` reached another way, such as through a `let`
binding, remains an ordinary FR-140, FR-142 or FR-148 conversion with its loss
record, and the equality then compares the completed values. Admission is
decided from declared bounds alone and guarantees that every member of `S`
converts with no rounding step, loss, undefined or refused outcome.

| Source `S` | Target `T` | Admitted exactly when |
| --- | --- | --- |
| any type | the identical type | always |
| `Int[lo,hi]` | `Integer` | always |
| `Int[lo,hi]` | `Int[lo2,hi2]` | `lo2 <= lo` and `hi <= hi2` |
| `Int[lo,hi]` | `Rational[n1,n2;d1,d2]` | `n1 <= lo`, `hi <= n2` and `d1 <= 1 <= d2` |
| `Int[lo,hi]` | `Decimal[c1,c2;s1,s2;m]` | `c1 <= lo * 10^s1` and `hi * 10^s1 <= c2` |
| `Rational[n1,n2;d1,d2]` | `Rational[n3,n4;d3,d4]` | `n3 <= n1`, `n2 <= n4`, `d3 <= d1` and `d2 <= d4` |
| `Rational[n1,n2;d1,1]` | `Integer`, `Int[..]` or `Decimal[..]` | the row for source `Int[n1,n2]` admits it |
| `Decimal[c1,c2;s1,s2;m]` | `Rational[n1,n2;d1,d2]` | `n1 <= min(c1,0)`, `max(c2,0) <= n2`, `d1 <= 1` and `10^s2 <= d2` |
| `Decimal[c1,c2;s1,s2;m]` | `Decimal[c3,c4;s3,s4;m2]` | `s3 <= s1`, `s2 <= s4`, and either `s3 = s1` with `c3 <= c1` and `c2 <= c4`, or `s3 < s1` with `c3 <= min(c1,0)` and `max(c2,0) <= c4` |
| `Decimal[c1,c2;0,0;m]` | `Integer` or `Int[..]` | the row for source `Int[c1,c2]` admits it |
| quantity in unit `U` | quantity in unit `V` with an exact rational value | FR-142 admits the conversion |

An unbounded `Integer` source converts only to `Integer`. No conversion to or
from `Float32` or `Float64`, and no text-profile, enumeration, record, tuple,
option, collection-kind or reference conversion, is an equality conversion.
FR-148 intrinsics take FR-148 conversions under FR-148's own rules. In operand
order before the comparison: an admitted FR-142 unit conversion charges its
`unit.*` schedule; an admitted `Decimal`-to-`Decimal` conversion charges the
accounting decimal-conversion schedule with no `decimal.rounding`; and an
admitted `Int[..]`-to-`Decimal[..]`, `Rational[..;d,1]`-to-`Decimal[..]` or
`Decimal[..]`-to-`Rational[..]` conversion charges the accounting decimal
schedule for that conversion, with `scale_expansion` and `integer_bits` sized
analytically. Every other admitted equality conversion keeps the source
magnitude, has no charge point and produces only the comparison value.

## Occurrence-pair plan

The plan is the tree of occurrence-path pairs rooted at `$`, formed from the
two completed operands of one type after `equality.plan-form` has charged the
walk:

- A Boolean, numeric, text, enumeration, quantity or reference pair is a
  terminal leaf.
- An option pair of two `none` values is one terminal equal pair; `none` with
  `present` is one terminal unequal pair; two `present` values add one child
  pair for their payloads.
- A record pair has one child pair per field in declaration order, and a tuple
  pair has one per position. A field pair whose two slots are present is the
  pair of the two field values. Two `absent` slots, or two `null` slots, form
  one terminal equal pair; every other slot-state combination forms one
  terminal unequal pair with no descent.
- A sequence or ordered-set pair with different lengths is one terminal unequal
  pair. Otherwise it has one child pair per index.
- A set pair with different member counts, or a bag pair with different
  occurrence counts, is one terminal unequal pair. Otherwise both sides are
  ordered by the FR-144 canonical key, which every admitted element type has,
  a bag listing each occurrence, and the pair has `n` child pairs matched by
  rank.

The pair count is the number of nodes in this tree. It depends only on the two
values, never on representation sharing, insertion order or hash order, and
there is no early exit after an unequal pair. A set or bag is equal when every
rank pair is equal. If plan formation finds a reference pair whose universes
differ, the equality is `refused { code: foreign_reference }` after
`equality.plan-form` and before `equality.plan`, with no further charge and no
Boolean.

Before descent, `equality.plan` computes the complete semantic occurrence-pair
plan and reserves its pair/work/result allowance atomically. Records, tuples,
sequences, ordered sets, options and recursive records use declaration/index
depth-first order. Sets and bags use rank order under their FR-144 canonical
element key, so the pair count and Boolean are independent of insertion and
iteration order. Once the plan is reserved, an implementation's evaluation
order is unobservable.

Every planned semantic occurrence-path pair receives one `equality.pair`
charge. Immutable DAG sharing never reduces that normative charge: shared and
duplicated representations of the same occurrence tree have identical
accounting under `quire.value.accounting/v1`. An implementation may memoize as
an unobservable optimization but cannot change charges, outcome or identity.
Object references are terminal and
do not descend into referenced state. If evaluation required to construct a nested operand returns
undefined, refused or incomplete, no completed outer value exists and the outer
expression returns that same disposition. When several nested operands stop,
the first one in FR-143 construction order (field declaration order, position
order, then the left operand before the right) propagates, and no later operand
is evaluated. If comparison traversal exhausts its
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
| FR-149-AC-8 | Set/bag equality matches both sides by FR-144 canonical-key rank, so insertion and iteration order change neither equality nor accounting. | Test (TC-194) |
| FR-149-AC-9 | Top-level Boolean, numeric and reference equality charge exactly `equality.plan-form` and the one-pair equality schedule, while `=` on an IEEE-bearing type is `ill_typed`. | Test (TC-194) |
| FR-149-AC-10 | Exactly the tabled equality conversions are admitted from declared bounds, every other conversion operand of `=` or `!=` is `ill_typed` before any charge even when the value fits, and admitted decimal and rational conversions charge their decimal schedule. | Test (TC-194) |
| FR-149-AC-11 | Plan pair counts follow the structural-mismatch, keyed-rank and cardinality short-circuit rules exactly, every plan walk is charged by `equality.plan-form` before it happens, and a foreign-universe reference pair refuses after that charge and before `equality.plan`. | Test (TC-194) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

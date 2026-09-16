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
  - target: ix://agent-ix/quire-specification/FR-149
    type: references
  - target: ix://agent-ix/quire-specification/FR-204
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

Every admitted set, bag and ordered-set element type has a total canonical key
(below), so set and bag canonical ordering always exists and never uses
insertion or hash order. Bounds are inclusive (`min..max`), checked before
materialization, and count occurrences for sequences/bags and members for
sets/ordered sets.

The canonical representation is an order, not a byte encoding. The canonical
order of a set or bag is exactly the ascending typed total key order below, and
Complete V1 defines no collection byte encoding, sort-key bytes or hash. A
consumer that serializes a collection value uses its own owning serialization
contract over this order.

## Bound and type identity

The cardinality bound is part of the collection type. `K<T>[a,b]` and
`K2<T2>[c,d]` are the same type exactly when `K = K2`, `T` and `T2` are the same
type, `a = c` and `b = d`; element-type identity includes every nested bound.
FR-149 admits no collection equality conversion, so `=` or `!=` between
collection types that differ only in bound is
`refused { code: ill_typed, cause: type-mismatch }`. Such values are compared by
first binding an FR-145 same-kind `convert<K<T>[a,b]>(c)` through `let`, whose
loss is empty and whose target bound is checked at run time. A bound is not
only a construction check, and no implicit widening or narrowing exists.

## Collection literal typing

A collection literal (`sequence[...]`, `set[...]`, `bag[...]` or
`orderedSet[...]`) has no standalone type. It is checked against the unique
expected type of its position: a declared field, parameter or result type, a
`convert` target, the element type of an enclosing literal, or the static type
of the other operand of `=` or `!=`. The expected type must be a collection of
the literal's kind, `K<T>[a,b]`; the literal then has exactly that type, and
each element expression is checked against the expected type `T`. A literal
without a unique expected type, such as both operands of `=` or the right side
of an unannotated `let`, is `refused { code: ill_typed, cause: ambiguous-literal }`.
A literal of another kind is `refused { code: ill_typed, cause: type-mismatch }`.
The bound is checked by construction at run time, not by type checking.

## Element admission and canonical keys

The element type of `Set`, `Bag` and `OrderedSet` must admit FR-149 `=`,
because uniqueness and multiplicity are decided by that relation. An element
type that contains `Float32` or `Float64` at any depth admits no `=`, so such a
collection type is `refused { code: ill_typed, cause: operator-ineligible }`.
`Sequence<T>` admits every complete-V1 element type.

Every type that admits FR-149 `=` has a total canonical key, exactly as
follows. Keys are compared as written, and two values have equal keys exactly
when they are FR-149 equal.

| Type | Canonical key |
| --- | --- |
| `Boolean` | `false` before `true` |
| `Integer`, `Int[..]`, `Rational[..]`, `Decimal[..]` | exact mathematical value, ascending |
| Quantity of one unit | exact value in that unit, ascending |
| `Text[..]` | the FR-141 compared form under the type's profile (the normalized scalar sequence, or the UTF-8 bytes for `binary-utf8`), ordered lexicographically by scalar value or byte |
| enumeration | declaration position for an `ordered enum`; otherwise the case identifier's ASCII bytes, lexicographically, never its declaration position |
| `Reference<T>` | the FR-204 identity triple (universe identity, object-type declaration identity, object identity), compared component by component, each component lexicographically by its canonical identity bytes |
| `Option<T>` | `none` before a present value, then the key of the payload |
| record, tuple | lexicographic over fields in declaration order or positions in order; a slot orders `absent` before `null` before a present value, and present values by key |
| `Sequence<T>`, `OrderedSet<T>` | lexicographic over element keys; a proper prefix comes first |
| `Set<T>`, `Bag<T>` | lexicographic over the ascending element-key list, with a bag occurrence listed once per multiplicity |

A recursive record type has a key because every finite value of it does.
References from different universes never meet in one collection, because
their FR-149 comparison refuses construction.

A canonical key is not an ordering operator and is not semantic. The
identifier-byte key of an unordered enumeration and the identity-byte key of a
reference expose no order that the type denies: `<`, `<=`, `>` and `>=` on an
unordered enumeration or a reference remain
`refused { code: ill_typed, cause: operator-ineligible }`, and FR-204 still
declares no semantic order between object identities. A key only fixes
FR-144/FR-145 visiting order, which decides charge order and which member stops
a short-circuiting query. Because declaration position is never used for an
unordered enumeration, reordering its cases changes no visiting order.

## Construction and traversal

Collection element expressions are evaluated in source position order under
FR-143's first-stopped-element rule. The evaluator then forms the kind's
occurrences, decides duplicates with the FR-149 relation and checks the
declared bound. A duplicate decision that FR-149 refuses, such as a
`foreign_reference` pair, refuses construction with that outcome. A count
outside `min..max` returns
`refused { code: cardinality_out_of_bound, cause: below-minimum }` or
`refused { code: cardinality_out_of_bound, cause: above-maximum }` before
materialization, with the collection type, the bound and the formed count.

## Collection charges

Every collection construction, membership comparison and result retention is
metered by the `quire.value.accounting/v1` collection family. A collection
constructor expression such as `set[e1, ..., en]` charges, in this order:

1. For each `i` from 1 to `n`: `collection.element` (one work unit), then the
   evaluation of `ei` with its own charges.
2. Occurrence formation. A sequence makes no membership comparison. For a set,
   bag or ordered set, occurrence 1 is retained, and each occurrence `i >= 2`,
   in source order, is compared with the distinct members retained before it,
   in retention order, stopping at the first equal member. Each comparison of
   candidate `c` with member `m` charges `collection.member-walk`
   (`work_units += occ(c) + occ(m)`), then decides FR-149 plan formation,
   including the `foreign_reference` universe check, then charges
   `collection.member-test` (`work_units += p`, the planned pair count).
3. `collection.bound` with `value_occurrences` equal to the formed bound count,
   then the bound check, which makes no charge.
4. `collection.result-retain` with `result_units += occ(result)`.

Occurrence `i` is retained as a new member when no comparison is equal;
otherwise it adds one occurrence to the equal bag member or is discarded by a
set or ordered set. FR-145 result formation uses steps 2 to 4 over its produced
occurrences. The first unavailable charge returns incomplete, and no partial
collection is exposed. Every amount depends only on the completed occurrences
and their source or visiting order, never on a hash, sort or other internal
algorithm. The equality counts of FR-149 and TC-194 measure only the `=`
expression over completed operands; building those operands charges this
schedule separately.

Where an operation must visit the members of a `Set` or `Bag` one at a time,
it visits them in ascending canonical-key order, and a bag's equal-key
occurrences are consecutive. This order is not semantic. It fixes which charge
is reached first and which element stops a short-circuiting query, so the
outcome never depends on insertion or hash order.

Complete V1 defines no collection ordering (`<`, `<=`, `>`, `>=`) and no
union, intersection, difference, sort, minimum or maximum operator. Such a
comparison is `refused { code: ill_typed, cause: operator-ineligible }`. A call
to such a name resolves only to a checked user or library pure function under
FR-146 and FR-307; otherwise it is
`refused { code: missing_declaration, cause: missing-name }`.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-144-AC-1 | Permuting a set or bag does not change equality, while permuting a sequence or ordered set can. | Test (TC-189) |
| FR-144-AC-2 | Duplicate insertion is preserved by sequence/bag and eliminated by set/ordered set according to their defined order rule. | Test (TC-189) |
| FR-144-AC-3 | Missing or exceeded cardinality bounds refuse with `cardinality_out_of_bound` and its bound cause, without partial materialization. | Test (TC-189) |
| FR-144-AC-4 | Set/bag insertion order never affects equality or canonical order, and canonical order is the typed total key order. | Test (TC-189) |
| FR-144-AC-5 | Bound counting follows occurrence versus unique-member semantics for each collection kind. | Test (TC-189) |
| FR-144-AC-6 | Every `=`-admitting type, `Reference` included, has exactly the tabled total canonical key, and an IEEE-bearing set, bag or ordered-set element type is `ill_typed`. | Test (TC-189) |
| FR-144-AC-7 | Set and bag visiting uses ascending canonical-key order independent of insertion order, and a key never admits an ordering comparison. | Test (TC-189) |
| FR-144-AC-8 | Collection construction charges `collection.element`, `collection.member-walk`, `collection.member-test`, `collection.bound` and `collection.result-retain` in the stated order and amounts, and each denied charge returns incomplete with no partial collection. | Test (TC-189) |
| FR-144-AC-9 | The cardinality bound is part of collection type identity, so equality between bounds that differ is `ill_typed`; an unordered enumeration key uses identifier bytes and never declaration position. | Test (TC-189) |
| FR-144-AC-10 | A collection literal takes exactly its unique expected type, and a literal without one is `ill_typed` with cause `ambiguous-literal`. | Test (TC-189) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

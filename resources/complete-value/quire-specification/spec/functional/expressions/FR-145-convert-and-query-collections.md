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
  - target: ix://agent-ix/quire-specification/FR-146
    type: references
  - target: ix://agent-ix/quire-specification/FR-149
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
reduce over a set or bag admit only the closed step catalog below, fold takes
its declared identity and reduce has no empty value. Conversions that lose order or duplicates report that loss.

## Operation matrix

| Operation | Sequence | Set | Bag | Ordered set |
| --- | --- | --- | --- | --- |
| `map` / `collect` | Preserve source occurrence order and multiplicity. | Produce a set and coalesce equal results. | Preserve result multiplicity. | Preserve first-result order and coalesce later equal results. |
| `filter` | Preserve surviving occurrence order and multiplicity. | Preserve the set of surviving values. | Preserve the multiplicity of surviving values. | Preserve the order of surviving first occurrences. |
| `flatten` | Concatenate nested semantic occurrences in outer then inner order. | Union nested values. | Sum nested multiplicities. | Preserve first occurrence while traversing outer then inner order. |
| `count` | Count occurrences whose predicate is true. | Count members whose predicate is true, so each value contributes zero or one. | Evaluate the predicate once per occurrence and count true occurrences, so each value contributes zero or its multiplicity. | Count members whose predicate is true, so each value contributes zero or one. |
| `fold` | Traverse occurrence order from the declared identity. | Require a commutative and associative function. | Require a commutative and associative function and apply it once per occurrence. | Traverse declared order from the declared identity. |
| `reduce` | Traverse occurrence order; empty input is undefined. | Require a commutative and associative function; empty input is undefined. | Require a commutative and associative function and apply it once per occurrence; empty input is undefined. | Traverse declared order; empty input is undefined. |

`map` and `collect` are one semantic operation. `flatMap` is exactly
`flatten(map(source, function))`; it is not a separately extensible operator.
An explicit target-kind conversion records each discarded property from
`order`, `uniqueness` and `multiplicity`. An empty fold returns only its declared
identity. An empty reduction has no value; the
[fold and reduce admission](#fold-and-reduce-admission) section states where
that is refused statically and where it is a located undefined outcome.

## Forms, result types and bounds

The source forms are the grammar's `map`, `collect`, `filter`, `flatMap`,
`flatten`, `fold<A>(acc, x in c: step, identity: i)`,
`reduce<A>(acc, x in c: step)`, `forall`, `exists`, `count<N>`, `sum<N>`,
`size`, `contains` and `convert<K<T>[min,max]>(c)`. A binder is evaluated once
per visited occurrence. Sequences and ordered sets are visited in occurrence
order, and sets and bags in FR-144 canonical-key order. `forall` stops at the
first false and `exists` at the first true; the empty cases are true and false.
`contains(c, v)` scans in the same visiting order (a bag's distinct members
once each) and stops at the first equal member, matching the
[state contract](../../../proposals/quire-v1/state-contract.md)'s
first-equality stop; an empty `c` is false. `size`
returns the FR-144 bound count, and `count<N>` and `sum<N>` keep the
`state-contract` result-domain rules for every kind. A `sum` over a set or bag
must prove that every accumulated prefix lies inside the domain for every visit
order, not only for the canonical order.

The static result type is derived from source `K<T>[a,b]`, as follows. `map`
and `collect` over a sequence or bag return `[a,b]`, and over a set or ordered
set return `[min(a,1),b]`. `filter` returns `[0,b]`. For `flatten` of an outer
`K1<K2<T>[c,d]>[a,b]`, the result kind is `K1`, and the bound is `[a*c,b*d]` for
a sequence or bag, or `[min(a*c,1),b*d]` for a set or ordered set. `flatten`
whose outer kind is a sequence or ordered set and whose inner kind is a set or
bag is `refused { code: ill_typed, cause: type-mismatch }`, because the inner kind has no semantic
occurrence order; there is no implicit key-order substitute. Every other
kind combination is admitted by the operation matrix. Because the derived bound
always contains the actual result, only a constructor or `convert` against an
authored bound can return FR-144 `cardinality_out_of_bound`.

## Fold and reduce admission

`A` is the grammar's `qualified-name` type argument and names the accumulator
type. Because `Integer` and `Boolean` are keywords, they are named through an
`alias-decl` such as `type Total = Integer;`, which creates no new type. `acc`
has type `A`, `x` has the element type, and `step` and the fold identity `i`
must each have type `A`. `fold` requires `identity:` and `reduce` forbids it;
each omission or addition, and each type mismatch, is
`refused { code: ill_typed, cause: type-mismatch }`. For `reduce`, `A` must be
the element type, and the first element initializes the accumulator. Admission
checks typing first: `A`, `identity:` presence, and the types of `step` and
`i`. Only a fold or reduction that is well typed is then checked against the
set-and-bag step catalog below, so a step that fails both is reported as
`type-mismatch`. For a
sequence or an ordered set, any pure total `step` of type `A` is admitted.

An empty `reduce` has no value, for every collection kind. Static definedness
applies exactly as in the
[state semantics](../../../proposals/state-core/state-semantics.md) judgment
`D; A; G; F |- e : T ! defined`: linking a clause, predicate or function body
checks every `reduce` on a path that can execute, and requires a proof that
`size(c) >= 1`, from a declared minimum bound of at least one or a sound FR-044
guard fact on that path. Without the proof, linking returns
`refused { code: undefined_expression, cause: unproved-range }` at the
`reduce`, naming the operand and the `size(c) >= 1` obligation. A `reduce` in an
unreachable branch carries no obligation. Therefore no linked subject evaluates
an empty `reduce`. Only direct kernel evaluation of an unlinked expression over
supplied values, which makes no definedness judgment, can meet one, and it
returns a located undefined outcome with no charge.

For a set or bag, a step is commutative and associative only when all of the
following hold: its body is `acc OP t` or `t OP acc`; `acc` does not occur in
`t`; `t` has type `A`; and `OP` with `A` is `+` or `*` on `Integer`, or `and` or
`or` on `Boolean`. Any other set or bag step, including one over a bounded
`Int`, `Rational`, `Decimal` or quantity accumulator, is
`refused { code: ill_typed, cause: operator-ineligible }` and names the failed
commutativity and associativity obligation. A bounded accumulator is refused because
intermediate membership would depend on visit order.

This syntactic catalog is the only way a step's commutativity and associativity
is established. A called function, a library export, a comment or any
declaration of algebraic properties is never trusted and never proves them, so
a step whose body calls a function is refused for a set or bag even when that
function is commutative and associative.

## Explicit kind conversion and loss

`convert<K2<T>[min,max]>(c)` of a `K1<T>` value is the only collection-kind
conversion, and writing `convert` is the explicit acceptance of its loss. The
element type must be identical, and element conversion is
`refused { code: ill_typed, cause: type-mismatch }`. Kind properties are: sequence
{`order`, `multiplicity`}; set {`uniqueness`}; bag {`multiplicity`}; ordered set
{`order`, `uniqueness`}. The completed result carries
`CollectionLoss { discarded }`, where `discarded` is the static set difference
of the `K1` properties minus the `K2` properties, listed in the order `order`,
`uniqueness`, `multiplicity`. It is recorded even when no duplicate actually
occurs. Converting a set or bag to a sequence or an ordered set uses ascending
canonical-key order. The authored target bound is checked after coalescing and before
materialization.

## Accounting

Every form charges the `quire.value.accounting/v1` collection family. A
*visit* is one `collection.visit` (one work unit) charged before the binder is
bound to that occurrence; the body, predicate or step is then evaluated with its
own charges. *Formation* is FR-144 construction steps 2 to 4 (membership
comparisons, `collection.bound` and `collection.result-retain`) over the
produced occurrences in production order. A *scalar retain* is one
`collection.result-retain` with `result_units += 1`, and an *accumulator retain*
is one `collection.result-retain` with `value_occurrences = occ(result)` and
`result_units += occ(result)`. Integer `+` and `*` in a step or summand charge
the accounting `integer-arithmetic` family, and Boolean `and` and `or` charge
`boolean.result-retain`. Each form charges exactly:

| Form | Charges in order |
| --- | --- |
| `map`, `collect` | a visit and body per source occurrence; then formation of the results, with membership comparisons for a set, bag or ordered-set result |
| `filter` | a visit and predicate per source occurrence; then formation of the survivors, with no membership comparison because survivors are already distinct or grouped |
| `flatten` | per outer occurrence, a visit of it and then a visit of each of its inner occurrences; then formation of the inner occurrences in visit order, with membership comparisons for a set, bag or ordered-set result |
| `flatMap` | the `map` charges, then the `flatten` charges |
| `fold` | the identity expression; then a visit and step per occurrence; then an accumulator retain |
| `reduce` | a visit of the first occurrence, which initializes the accumulator; then a visit and step per later occurrence; then an accumulator retain; an empty source reached by direct kernel evaluation charges nothing |
| `forall`, `exists`, `count`, `sum` | a visit and predicate or summand per occurrence until a short-circuit stops the query; each `sum` addition charges what that addition charges; then a scalar retain |
| `size` | a scalar retain only |
| `contains(c, v)` | for each member `m` in scan order until the first equal one: `collection.member-walk` (`occ(v) + occ(m)`), the FR-149 plan-formation checks, then `collection.member-test` (the planned pair count); then a scalar retain |
| `convert` | a visit per source occurrence (for a bag to a set or ordered set, one visit per distinct member); then formation, with membership comparisons only when the source is a sequence and the target is a set, bag or ordered set |

Visits and the charges of evaluated bodies interleave in visiting order. The
first unavailable charge returns incomplete, and no partial collection or
accumulator is exposed. Because every per-element evaluation is preceded by a
named charge, an unbounded source can never evaluate unmetered.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-145-AC-1 | Map/filter/flatten over each collection kind returns the specified kind and deterministic semantic occurrences. | Test (TC-190) |
| FR-145-AC-2 | Fold on an empty collection uses only its required declared identity; a linked `reduce` without a proof of `size(c) >= 1` is `undefined_expression`, direct kernel evaluation of an empty reduction returns located undefined, and a fold without or a reduction with `identity:` refuses. | Test (TC-190) |
| FR-145-AC-3 | A conversion that loses order or multiplicity requires explicit acceptance and records the exact loss. | Test (TC-190) |
| FR-145-AC-4 | The operation matrix determines result kind, occurrence order, uniqueness and multiplicity for every source kind. | Test (TC-190) |
| FR-145-AC-5 | Set and bag folds or reductions refuse a function that is not both commutative and associative. | Test (TC-190) |
| FR-145-AC-6 | Result bounds, flatten kind admission and set/bag canonical-key visit order are derived exactly as specified, and a stopped binder exposes no partial result. | Test (TC-190) |
| FR-145-AC-7 | A kind conversion records exactly the static `CollectionLoss` property difference and refuses element conversion. | Test (TC-190) |
| FR-145-AC-8 | Every query and conversion form charges exactly its tabled visits, membership comparisons, bound, arithmetic and retain charges in order, `contains` stops at the first equal member, and each denied charge returns incomplete with no partial result. | Test (TC-190) |
| FR-145-AC-9 | A set or bag step outside the closed syntactic catalog is `ill_typed` even when it calls a function that declares or has commutative and associative behavior. | Test (TC-190) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

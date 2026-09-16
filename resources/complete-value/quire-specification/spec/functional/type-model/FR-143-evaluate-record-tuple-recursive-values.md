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
  - target: ix://agent-ix/quire-specification/FR-204
    type: references
  - target: ix://agent-ix/quire-specification/FR-307
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

## Declarations and identity

Complete V1 declares composite value types only with the grammar's
`record-decl` and `tuple-decl`. It has no variant, sum or tagged-union
declaration; such a spelling is
`refused { code: invalid_syntax, cause: unexpected-token }` at its first token. A record value expression
`R { f: e, ... }` is the single constructor of record `R`, and a tuple value is
the qualified call `T(e, ...)` of tuple `T`. An `alias-decl` names its target
type and creates no declaration identity.

A record or tuple declaration identity is the opaque I04 node key of its
`composite_type` node whose `semantic_form` is `record` or `tuple`. Complete V1
defines no nominal identity preimage for these nodes, unlike the enum,
dimension and declared-unit keys of FR-141, FR-142 and FR-322. The checked
package producer assigns the key; it is unique within one checked semantic
graph, and no reader or evaluator recomputes it. Two declarations are the same
exactly when their keys are equal within one checked package. Equal qualified
names, owners, field names, field types or shapes never make two keys equal,
and one declaration has exactly one key. An imported declaration has the key
that the importing package's graph assigns to the resolved FR-307 export, so
every dependency path that resolves one export yields one key. No composite
key is stable across checked packages. Mixing is impossible at check time,
because every type named in one checked package, imported ones included,
resolves to that package's own keys, so no source expression can denote a
composite type of another package. Values meet evaluation only through runtime
inputs and snapshots validated against the evaluating package. A supplied
composite value whose declaration key is not a key of that package's graph is
`refused { code: invalid_runtime_input, cause: wrong-value-kind }` during input
validation, before any evaluation or charge. A record field identity is
(declaration key, field identifier), and a tuple position identity is
(declaration key, zero-based position).

## Field presence

A field declared without `?` is required, and its slot holds exactly one
present value. A field declared with `?` holds exactly one of a present value,
`absent` or explicit `null`. In a record value expression, omitting a `?` field
constructs `absent` and `f: null` constructs explicit `null`. Type checking
refuses each of the following with `refused { code: ill_typed }`, located at the
originating field or call: omitting a required field; naming an undeclared
field; naming a field twice; supplying `null` for a required field; and calling
a tuple with an argument count other than its declared arity. `null` is never a
value of `Option<T>`, and `none` is never a field state.

A projection `r.f` of a `?` field is admitted only as the operand of
`present(r.f)` or `value(r.f)`. `present(r.f)` is true exactly when the slot
holds a present value, and false for both `absent` and explicit `null`.
`value(r.f)` returns that value and carries the FR-146 presence obligation, so
without a proved `present(r.f)` a linked body is
`refused { code: undefined_expression, cause: unproved-presence }`. Any other
use of `r.f`, such as an arithmetic operand, call argument, comparison operand
or field value, is `refused { code: ill_typed, cause: type-mismatch }`, because a
field slot is not a value of `T`. Whole-record equality still compares slot
states, so `absent` and `null` stay distinct. A projection of a field declared
without `?` is always admitted.

## Recursion rule

The declaration containment graph has one edge from declaration `D` to each
record or tuple declaration named anywhere in the type of a field or position
of `D`, including through `Option`, `Sequence`, `Set`, `Bag` and `OrderedSet`.
`Reference<T>` contributes no containment edge. An edge is *named* when it
starts at a record field; the field's whole type, including every option or
collection nested inside it, belongs to that field. A tuple position is not a
named constructor field, so an edge that starts at a tuple position is unnamed
even when an option or collection lies on it. An edge *escapes* when its path
passes a `?` field, an `Option<...>` or a collection type whose minimum bound is
`0`.

Type checking admits the containment graph exactly when both the subgraph of
unnamed edges and the subgraph of non-escaping edges are acyclic, self-edges
included. Otherwise every declaration on an offending cycle is
`refused { code: ill_typed }`, and the refusal names the cycle. Thus every
recursion crosses a named field, and every recursion can end in a finite value.
For example, `record List { head: Integer; tail: List?; }` is admitted,
`record Loop { next: Loop; }` is refused because it does not escape, and
`tuple Pair(Integer, Option<Pair>);` is refused because it is unnamed.

## Construction evaluation order

Every construction refusal above is decided by type checking before any field
expression is evaluated. After the last field or argument completes,
construction charges one `composite.result-retain` with
`value_occurrences = occ(result)` and `result_units += occ(result)` under
`quire.value.accounting/v1`, then exposes the value. Record field expressions are evaluated in field
declaration order, regardless of the order of the source fields. Tuple
arguments are evaluated in position order. The first field or argument whose
outcome is undefined, refused or incomplete stops construction. That outcome
becomes the construction's outcome, no later field or argument is evaluated or
charged, and no record or tuple exists. Operands of a binary expression are
evaluated left and then right, so a stopped left operand prevents evaluation of
the right operand.

## Object references

In `Reference<T>`, `T` must name a model object type exported by a bound model;
a record, tuple, enumeration, alias of a value type or other value type is
`refused { code: ill_typed, cause: type-mismatch }`.

No Complete-V1 source literal, constructor or conversion creates an object
identity or a `Reference<T>` value. A reference is obtained only from a bound
model snapshot under the selected state contract, as a model field of
reference type, a receiver, a population member, an `allInstances` member, a
`lookup` result or a relationship-end navigation result (FR-152, FR-153). It can
also arrive through a parameter, `let` binding or composite field that carries
such a value. Its identity is the snapshot-supplied FR-009/FR-204 triple
(universe, object-type declaration identity, declared object identity). Under
the selected `quire.model.complete/v1` definition the universe is the
`quire.model.object-universe/v1` identity, the type is the effective
declaration identity of the object's most-specific type, never the static `T`,
and `Reference<S>` converts to `Reference<T>` only by the FR-149 upcast row. Its retained observation is
not part of identity. `Reference<T>` is terminal in containment, construction
and equality.

## Acceptance Criteria

| ID | Criteria | Verification |
| --- | --- | --- |
| FR-143-AC-1 | Equal finite values of the same declared record/tuple type compare structurally equal. | Test (TC-188) |
| FR-143-AC-2 | Equal field shapes from different declared types do not become interchangeable. | Test (TC-188) |
| FR-143-AC-3 | Cyclic value containment refuses, while bounded recursive construction and object-reference cycles remain distinct. | Test (TC-188) |
| FR-143-AC-4 | Missing/extra record fields, wrong tuple arity and absence/null substitution each refuse at the originating component. | Test (TC-188) |
| FR-143-AC-5 | Canonical field ordering does not make equal-shaped values of different declared record types equal. | Test (TC-188) |
| FR-143-AC-6 | Declaration identity is the producer-assigned record/tuple node key compared only within one checked package, and a variant spelling is `invalid_syntax`. | Test (TC-188) |
| FR-143-AC-7 | The recursion rule admits exactly the declaration graphs whose unnamed-edge and non-escaping-edge subgraphs are both acyclic, and names each refused cycle. | Test (TC-188) |
| FR-143-AC-8 | Record fields are evaluated in declaration order, the first stopped field propagates its outcome with no later field evaluated or charged, and a completed construction charges `composite.result-retain`. | Test (TC-188) |
| FR-143-AC-9 | `Reference<T>` admits only a model object type, and a runtime composite value from another checked package is `invalid_runtime_input` before evaluation. | Test (TC-188) |
| FR-143-AC-10 | A `?` field projection is admitted only under `present` or `value`, an unguarded `value` is `undefined_expression` with cause `unproved-presence`, and any other use is `ill_typed` with cause `type-mismatch`. | Test (TC-188) |
| FR-143-AC-11 | Under `quire.model.complete/v1`, a reference obtained by navigation, lookup or `allInstances` carries the object's most-specific effective type, and no other source creates a reference. | Test (TC-198) |

## Dependencies

- [Complete-V1 grammar](../../../proposals/quire-v1/shared-grammar.md) owns every
  source form used by this requirement.
- [Subsystem and interface index](../../subsystems/index.md) identifies the
  typed producer/consumer boundary and dependency direction.
- Every requirement linked in frontmatter is a normative prerequisite.
  Implementation ordering may add enablement edges but cannot change these
  semantics or infer implementation availability from specification acceptance.

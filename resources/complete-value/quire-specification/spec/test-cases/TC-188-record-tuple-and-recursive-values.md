---
id: TC-188
title: "Record tuple and recursive values"
type: TC
relationships:
  - target: ix://agent-ix/quire-specification/FR-143
    type: verifies
---

# TC-188: Record tuple and recursive values

## Description

Compare same/different declared structures and construct finite versus cyclic recursive values.

## Test Procedure

Use the exact version-locked subject and inputs named by FR-143. Execute its positive case, each declared boundary, and each explicit refusal/non-conclusive mutation through the real local Rust boundary; compare the complete typed result and artifact accounting with the criterion.

Select the `quire.value.complete/v1` definitions at revision `1-draft.1` by
their exact DefinitionRefs in
[`complete-value-lock.json`](../../proposals/quire-v1/definitions/complete-value-lock.json).
Unless a row states limits, run it under limits large enough that no charge is
denied. Integer literals, parameters, `let` names and field projections make no
charge. Every `ill-typed` expectation is a type-checking
`refused { code: ill_typed }` made before any charge.

| Vector | Input | Expected result/disposition |
| --- | --- | --- |
| R01 | `record P { a: Integer; b: Integer?; }` values `P { a: 1 }` and `P { a: 1 }`; `P { a: 1, b: 2 }` and `P { b: 2, a: 1 }` | true; true, because source field order is not identity |
| R02 | `record A { x: Integer; }` and `record B { x: Integer; }` in one package, compare `a = b` for parameters `a` of `A` and `b` of `B`; then `type C = A;` and compare a `C` parameter with an equal `A` parameter | `refused { code: ill_typed, cause: type-mismatch }`; true, because an alias creates no declaration identity |
| R03 | Two checked packages each declare `record R { x: Integer; }`, and one expression of package P compares a parameter of P's `R` with an expression of the other package's `R`; then a runtime input for P's `R` parameter carries the other package's declaration key; then package P imports library L's export `R` through two dependency paths that resolve one FR-307 export | `refused { code: ill_typed, cause: type-mismatch }` at checking, because no well-typed expression can hold both keys; `refused { code: invalid_runtime_input, cause: wrong-value-kind }` during input validation, before any evaluation or charge; the two paths yield one declaration key, and equality of equal values is true |
| R04 | A declaration spelled `variant V { A, B }` | `refused { code: invalid_syntax, cause: unexpected-token }` at `variant`, because the locked grammar has no sum declaration |
| R05 | Recursion rule on `record List { head: Integer; tail: List?; }`, `record N { kids: Sequence<N>[0,2]; }`, `record A { b: B; }` with `record B { a: A?; }`, and `record O { r: Reference<M::Obj>; }` where `M::Obj` is an object type of imported model `M`; then `record P { x: Integer; }` with `record Bad { r: Reference<P>; }` | each of the first four is admitted; `Bad` is `refused { code: ill_typed, cause: type-mismatch }`, because a reference target must be a model object type |
| R06 | Recursion rule on `record Loop { next: Loop; }`, `record M { kids: Sequence<M>[1,2]; }` and `tuple Pair(Integer, Option<Pair>);` | ill-typed, naming the cycles `Loop -> Loop`, `M -> M` (does not escape) and `Pair -> Pair` (unnamed edge) |
| R07 | For `record P { a: Integer; b: Integer?; }`: `P { a: 1 }`, `P { a: 1, b: null }`, `P { b: 2 }`, `P { a: 1, c: 2 }`, `P { a: 1, a: 2 }` and `P { a: null }`; for `tuple T(Integer, Integer);`: `T(1)`; and `null` supplied for an `Option<Integer>` parameter | `b` absent; `b` explicit null, unequal to the first; then ill-typed at `P { b: 2 }` (missing `a`), `c` (undeclared), the second `a` (duplicate), `null` (required field), `T(1)` (arity) and `null` (not an option value) |
| R08 | `record Two { a: Set<Integer>[0,1]; b: Integer; }`, `function f using V(): Integer pure { 1 }`, `function g using V(): Integer pure { f() }` and parameter `q` of `Sequence<Integer>[0,2]` holding `1, 2`; evaluate `Two { b: g(), a: convert<Set<Integer>[0,1]>(q) }` under `ScalarLimitsV1` with every counter unlimited except `work_units: 6` | `refused { code: cardinality_out_of_bound, cause: above-maximum }` after exactly two `collection.visit`, one `collection.member-walk` (`occ(c) + occ(m) = 2`), one `collection.member-test` (`p = 1`) and `collection.bound`: six work units, with no `function.call` and no `composite.result-retain`, because field `a` is evaluated first in declaration order and stops construction. Evaluating `b` first would instead need two more `function.call` units and end incomplete |
| R09 | A snapshot of imported model `M` with objects `o1` and `o2` of object type `M::Node`, whose attribute `peer` of `Reference<M::Node>` in each points at the other; compare `o1.peer = o1.peer` and `o1.peer = o2.peer`; then `convert<Reference<M::Node>>(1)` | the reference cycle is admitted and each comparison is by identity triple only: true, false; the conversion is `refused { code: ill_typed, cause: type-mismatch }`, because no source form creates an object identity |
| R10 | `R { a: 1 }` for `record R { a: Integer; b: Integer?; }` under `ScalarLimitsV1` with every counter unlimited except `result_units: 1`; then unlimited | `incomplete { limit_kind: result_units, limit: 1, consumed: 0, next_charge: 2, charge_point: composite.result-retain }`; the record, after exactly `composite.result-retain` with `occ = 2`: one work unit and two result units |
| R11 | For `record P { a: Integer; b: Integer?; }` and function bodies over parameter `x` of `P`: `if present(x.b) then value(x.b) else 0`; `value(x.b)`; `x.b + 1`; `x.a + 1`; then `present(x.b)` evaluated for `P { a: 1 }`, `P { a: 1, b: null }` and `P { a: 1, b: 2 }` | admitted; `refused { code: undefined_expression, cause: unproved-presence }` when linked; `refused { code: ill_typed, cause: type-mismatch }`, because a `?` slot is not an `Integer` value; admitted; false, false, true |

## Expected Results

Every positive and boundary result matches FR-143; each invalid, unsupported or exhausted mutation returns its exact typed disposition with source/provenance, emits no approximation, and preserves independent sibling results.

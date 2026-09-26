---
id: TC-471
title: "The assembler admits source enums and predicates, which check and lowering then use"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-092
    type: verifies
---
# TC-471: The assembler admits source enums and predicates, which check and lowering then use

## Description

Verify that the assembler admits each enum form as an `EnumBinding` whose
declaration and member keys `check` mints over the unit's source owner, by
QSpec FR-141's nominal preimages; that it admits a predicate as a function
of kind `Predicate`; and that the checked package calls both and lowers them
to the nodes FR-092 fixes.

This catches: members left in source order for an unordered enum (the
preimage sorts them); a display string entering a key; a constant owner;
enum admission reached only from tests; a duplicate case accepted; an enum
name missing from type-name resolution; and a predicate keyed as a
`pure_function`.

Scope: FR-091-AC-27, FR-091-AC-28, FR-091-AC-29, FR-091-AC-30,
FR-092-AC-13.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`. The owner is authority `a`, identity `u` unless stated.

1. Assemble `ordered enum Status { READY, DONE }` and then
   `enum Color { RED, BLUE = "Blue" }`. Read `enums` and
   `declared_type_spans`.
2. Assemble `enum Color { RED, BLUE }`, and assemble step 1's `Status`
   source under (`a`, `w`). Read the declaration keys.
3. Assemble and check `ordered enum Status { READY, DONE }`,
   `function isReady using v(s: Status): Boolean pure { s = Status::READY }`,
   `function ok using v(): Boolean pure { isReady(Status::READY) }` and
   `function later using v(): Boolean pure { Status::READY < Status::DONE }`.
   Read `isReady`'s resolved parameter type, and call `ok` and `later`
   through `CheckedPackage::call`.
4. Assemble and check `enum Color { RED, BLUE }` with
   `function bad using v(): Boolean pure { Color::RED < Color::BLUE }`.
5. Assemble and check step 3's `Status` with
   `function gone using v(): Boolean pure { Status::GONE = Status::READY }`.
6. Assemble `enum E { A, B, A }`, `enum F { X }`, `record F { y: Boolean; }`,
   `function f using v(p: F): Boolean pure { true }` and
   `function g using v(p: Shade): Boolean pure { true }`.
7. Assemble and check
   `predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }` and
   `function three using v(): Boolean pure { Positive(3) }`. Read
   `functions`, `Positive`'s resolved signature and selection, and call
   `three`.
8. Assemble `predicate Q using w(x: Boolean): Boolean { x }`.
9. Assemble and check `predicate R using v(x: Boolean): Boolean { R(x) }`.
10. Lower step 7's checked package and read `Positive`'s node. Lower
    `function Positive using v(x: Int[0, 9]): Boolean pure { x > 0 }` alone
    in another unit and read its node.
11. Lower step 3's checked package and read the nodes keyed N1 and N2.

Tag the tests with the AC ids they back and `TC-471`.

## Expected Results

- Step 1: `enums` holds `Status` and then `Color`. `Status`'s declaration
  key is FR-091 vector N1, with members `READY` then `DONE`, and `READY`'s
  member key is N2. `Color`'s key is N3, with members `BLUE` then `RED`.
  `declared_type_spans` holds the span of `Status` and of `Color`.
- Step 2: the `Color` key is N3; the `Status` key under (`a`, `w`) is N4.
- Step 3: `isReady`'s parameter type is `ValueType::Enum` of `Status`'s
  shape; `ok` and `later` each complete with `true`.
- Step 4: check refuses with `ill_typed`.
- Step 5: check refuses with `missing_declaration`/`missing-name` naming
  `Status::GONE`.
- Step 6: one refusal and no `PackageDeclarations`, holding a
  duplicate-enum-member error (`ambiguous_declaration`/`ambiguous-name`)
  naming `E`, `A` and both `A` spans; an ambiguous-type-name error naming
  `F`, the span of `p`'s type form and both `F` declaration spans; and an
  unresolved-type-name error naming `Shade`.
- Step 7: `functions` holds `Positive` (kind `Predicate`) and then `three`
  (kind `Function`); `Positive` has parameter type `Int[0..9]`, result
  `Boolean` and selection `v`; `three` completes with `true`.
- Step 8: an undeclared-alias error naming `w`.
- Step 9: check refuses with the FR-146 `missing-measure` obligation.
- Step 10: the predicate's node has `node_tag` `function`, `semantic_form`
  `predicate`, `declaration.qualified_name` `["Positive"]` and the unit's
  owner; the function's node has `semantic_form` `pure_function` and a
  different key.
- Step 11: N1 is a `scalar_type`/`enum` node with
  `declaration.qualified_name` `["Status"]`, and N2 a `value`/`enum_value`
  node.

## Status

Not implemented. QSL-275 adds the assembler's enum admission, the predicate
kind and the `predicate` node form; today every enum and predicate
declaration refuses at S2 with `NoDispatchEntry`.

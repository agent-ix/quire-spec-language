---
id: TC-170
title: "check-stage modules move to check with no duplication"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-170: check-stage modules move to check with no duplication

## Description

Verify that `check.rs`, `facts.rs`, `ir.rs` and `termination.rs`, and
`value::expression::mod.rs`'s checking-only methods
(`PackageDeclarations::check`, `CheckedPackage::check_expression`,
`CheckedPackage::check_postcondition_expression`,
`CheckedPackage::check_clause_expression`), are defined exactly once, under
the new `check` module, with no copy — partial or complete — left behind
under `value::expression`. This test is written to fail on duplication, not
only on absence: an implementation that adds the four modules under `check`
while leaving `value::expression`'s `mod check;`, `mod facts;`, `mod ir;` and
`mod termination;` declarations (and their file contents) in place passes an
absence-only check but fails this one. Scope: FR-068-AC-1.

## Test Procedure

1. Read `src/lib.rs` and confirm a `pub mod check;` (or `mod check;`)
   declaration exists at the crate root, alongside `forms`, `model` and
   `package`.
2. Read `src/value/expression/mod.rs` and confirm it declares none of
   `mod check;`, `mod facts;`, `mod ir;`, `mod termination;`.
3. Confirm `src/check/` contains a file (or inlined module) providing each of
   `check`, `facts`, `ir` and `termination`'s current content, and that
   `src/value/expression/` contains no file of the same name with checking
   content.
4. For each of `PackageDeclarations::check`, `CheckedPackage::check_expression`,
   `CheckedPackage::check_postcondition_expression`,
   `CheckedPackage::check_clause_expression`: search the whole compiled crate
   for every location defining a method of that name on that type, and
   record each location found.
5. Build the crate. A build that succeeds with both an old and a new
   definition present (for example, guarded by conditional compilation, or
   because the old file was renamed rather than deleted and is still
   included by some other path) is a duplication finding for this test, not
   a pass.

## Expected Results

- Step 1: `check` is declared at the crate root.
- Step 2: none of the four `mod` declarations remains under
  `value::expression`.
- Step 3: each of the four modules' content exists under `check` and nowhere
  under `value::expression`.
- Step 4: each of the four checking methods has exactly one defining
  location, under `check`; a method found defined a second time, anywhere,
  fails this step and names both locations.
- Step 5: the crate builds with exactly one definition of every item this
  test checks; a second, reachable definition of any one of them fails this
  step.

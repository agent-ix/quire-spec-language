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
absence-only check but fails this one. This test also extends the same
one-defining-location search, and a shape-equality comparison, to every
symbol FR-068-CON-3 names (`CheckCause`, `CheckRefusal`, `Obligation`,
`MeasureObligation`, `CheckingStage`, `CheckingLimitKind`,
`DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`, `Origin`,
`ProvedInterval`, `WrongSnapshotCause`, `CheckedPackage`, `CheckedExpression`,
`CheckedFunction`, `PackageDeclarations`, `CheckingLimits`,
`DepthAboveMaximum`, `DispatchOperation`, `EnumBinding`,
`MAX_CHECKING_DEPTH` and `CheckMode`) — not only the four checking methods — because a
renamed leftover (for example a stray `qsl-eval/src/value/expression/typing.rs` still
defining a second `Typer`-adjacent type under a different file name) is not
by itself a build error, and the four-method-only scan this test originally
ran would not surface it. Scope: FR-068-AC-1, FR-068-CON-1, FR-068-CON-3.

## Test Procedure

1. Read `src/lib.rs` and confirm a `pub mod check;` (or `mod check;`)
   declaration exists at the crate root, alongside `forms`, `model` and
   `package`.
2. Read `qsl-eval/src/value/expression/mod.rs` and confirm it declares none of
   `mod check;`, `mod facts;`, `mod ir;`, `mod termination;`.
3. Confirm `qsl-semantics/src/check/` contains a file (or inlined module) providing each of
   `check`, `facts`, `ir` and `termination`'s current content, and that
   `qsl-eval/src/value/expression/` contains no file of the same name with checking
   content.
4. For each of `PackageDeclarations::check`, `CheckedPackage::check_expression`,
   `CheckedPackage::check_postcondition_expression`,
   `CheckedPackage::check_clause_expression`: search the whole compiled crate
   for every location defining a method of that name on that type, and
   record each location found.
5. Extend step 4's one-defining-location search to every symbol
   FR-068-CON-3 names (`CheckCause`, `CheckRefusal`, `Obligation`,
   `MeasureObligation`, `CheckingStage`, `CheckingLimitKind`,
   `DispatchFunctionRole`, `InvalidDispatchDeclaration`, `Location`,
   `Origin`, `ProvedInterval`, `WrongSnapshotCause`, `CheckedPackage`,
   `CheckedExpression`, `CheckedFunction`, `PackageDeclarations`,
   `CheckingLimits`, `DepthAboveMaximum`, `DispatchOperation`,
   `EnumBinding`, `MAX_CHECKING_DEPTH`, `CheckMode`): search the whole compiled crate for
   every location defining each name, and for every pair of same-named
   definitions found (there should be none), compare their shape
   (variants/fields/signature) as FR-068-CON-3 requires.
6. Separately from name-based search, enumerate every top-level item
   (`struct`, `enum`, `fn`, `const`, `type`) defined anywhere under
   `qsl-semantics/src/check/`'s moved content (the former `check.rs`, `facts.rs`, `ir.rs`,
   `termination.rs` and `refusal.rs`'s check-cause portion) and, for each,
   search the rest of the compiled crate — including under
   `value::expression` — for any other item whose shape (fields, variants,
   or signature) matches it structurally, regardless of name. This catches
   a renamed leftover a name-based scan misses — for example a stray
   `qsl-eval/src/value/expression/typing.rs` still defining a second
   `Typer`-adjacent type under a different name, structurally identical to
   its counterpart in `check` — which is not by itself a build error and
   would pass steps 4-5 untouched.
7. Build the crate. A build that succeeds with both an old and a new
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
- Step 5: each of the twenty-one CON-3-named symbols has exactly one defining
  location, under `check`, matching CON-3's required shape; a symbol found
  defined a second time, anywhere, or whose second definition's shape
  differs, fails this step and names both locations.
- Step 6: no item under `check`'s moved content has a structurally matching
  counterpart defined anywhere else in the crate, under any name; a
  structural match under a different name — a renamed leftover — fails this
  step and names both locations, even though neither is a name collision a
  step-4/5 scan would catch.
- Step 7: the crate builds with exactly one definition of every item this
  test checks; a second, reachable definition of any one of them fails this
  step.

**Amendment (implementation, PR #282 review F1): step 6's scope, as landed.**
`xtask::definition_scan` implements steps 1-5 and 7 as written (a real,
whole-crate scan by name, in `real_check_module...`/
`con3_methods_and_symbols_each_have_exactly_one_defining_location`, and the
crate's own `cargo test` build for step 7). Step 6's literal text — an
arbitrary structural-similarity comparison between every pair of
differently-named items in the crate — would need a general shape-matching
engine, which is speculative build cost against the one concrete scenario
this step exists to catch (a stray file made reachable under a new module
name). That concrete scenario is closed a cheaper way instead: any such
stray file needs its own `mod` declaration somewhere reachable to be part of
the compiled crate at all, so `xtask::definition_scan::mod_declarations`
enumerates *every* `mod` item in `value::expression::mod.rs` (not only the
four this test names) and asserts the resulting set is exactly `{evaluate,
family}` — an unexpected fifth declaration is itself a finding, closing the
one path by which a step-4/5 name-based scan could miss a renamed leftover.
This mirrors TC-171's own established precedent in this repository: where a
criterion's full literal scope needs new build-time machinery this PR does
not otherwise need, the gap is recorded here rather than declared silently
closed. See `xtask/src/definition_scan.rs`'s own module doc, "Scope of what
this catches."

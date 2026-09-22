---
id: TC-243
title: "Typestate constructors are private to their stage module"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-087
    type: verifies
---
# TC-243: Typestate constructors are private to their stage module

## Description

Verify that `CheckedGraph`, `CheckedPackage`, `EmittedPackage`,
`VerifiedPackage` and `ImportView` each have exactly one defining location,
and that every field and every constructor of each type carries no
visibility qualifier reachable from outside its owning stage module. This
is TC-171's own established claim (FR-068-AC-2), extended from three types
to five.

This test is Inspection, not a compiled negative test, for the same reason
TC-171 gives and does not restate as a doctest anyway: `compile_fail` is a
doctest-only attribute in this crate (every existing `compile_fail` site is
a doctest, and there is no `trybuild`/`compiletest` dependency to run an
in-crate compile-fail check instead), and a doctest links the crate
externally. That external vantage can show only that a value outside the
whole crate cannot construct these types; it cannot show the claim this
test actually needs — that `value::expression` and `package` (in-crate
siblings of `check`, and of each other) cannot construct `check`'s,
`package`'s or `library`'s private state either. Where a field is already
module-private, not merely `pub(crate)`, a doctest also cannot distinguish
"private because this test's module boundary requires it" from "private
for an unrelated pre-existing reason," so it demonstrates nothing about
this test's specific claim either way. This test therefore reads declared
visibility and the accessor surface directly, as TC-171 does. Scope:
FR-087-AC-1.

## Test Procedure

1. For each of `CheckedGraph`, `CheckedPackage`, `EmittedPackage`,
   `VerifiedPackage`, `ImportView`: search the whole compiled crate for
   every location defining the type, and record each location found.
2. For each type found in step 1 (there should be exactly one location
   each), read its field and constructor visibility: `pub`, `pub(crate)`,
   `pub(super)`, `pub(in path)`, or no qualifier.
3. Confirm every field and constructor recorded in step 2 carries no
   visibility qualifier — private in the plain Rust sense.
4. For each type, read its owning module's accessor surface and confirm any
   consumer outside the module — specifically including `value::expression`
   reading `CheckedPackage`'s state, `package` reading `CheckedGraph`'s
   state, and `check` reading `VerifiedPackage`/`ImportView`'s state, the
   three concrete in-crate-sibling cases a doctest's external vantage
   cannot rule out — reaches the type's state only through a named accessor
   method, never a re-exported field or a module-path trick.

## Expected Results

- Step 1: exactly one defining location per type.
- Steps 2-3: every field and constructor is private; any qualifier found
  fails this step and names the type and the qualifier.
- Step 4: every external read, including each of the three named
  in-crate-sibling cases, goes through a named accessor; a direct field
  access from any of them fails this step and names the location.

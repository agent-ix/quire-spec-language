---
id: TC-171
title: "CheckedPackage/CheckedExpression/CheckedFunction constructors are private to check"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-068
    type: verifies
---
# TC-171: CheckedPackage/CheckedExpression/CheckedFunction constructors are private to check

## Description

Verify that `CheckedPackage`, `CheckedExpression` and `CheckedFunction` are
defined exactly once, in `check`, with every field and each struct's
constructor carrying no visibility qualifier at their `check`-module
definition site, so that `value::expression` (and every other module) can
reach a value of these types only through `check`'s own checking entry
points and public accessors. This test catches a split where the module
boundary is cosmetic: files moved into `qsl-semantics/src/check/`, but a field or
constructor left reachable (`pub`, `pub(crate)`, `pub(super)`, or a field
re-exported under a different name) so `value::expression` still builds the
type directly, or reaches its state without going through `check`'s API.

This test is Inspection, not a compiled negative test, and round 1 of this
PR's review wrongly specified it as a `compile_fail` unit test placed inside
the crate. `compile_fail` is a doctest-only attribute in this repository:
every existing `compile_fail` site (`src/package.rs:188`,
`src/temporal/mapping.rs:125` and the remainder) is a doctest, and there is no
`trybuild` or `compiletest` dependency to run an in-crate compile-fail check
instead. A doctest links the crate externally — the same vantage an
integration test in `tests/` has, from which `CheckedPackage`'s,
`CheckedExpression`'s and `CheckedFunction`'s fields are already
unconstructible today (module-private, not even `pub(crate)`, at
`value/expression/mod.rs:58-79` on the pre-move baseline), for a reason
unrelated to this move. A doctest-based before/after comparison would
therefore pass identically on the pre-move and post-move tree and could not
demonstrate anything: it is not merely a weaker check, it has no
discriminating power at all here. Rather than adding a new build-time
dependency (`trybuild`) to a spec-only PR whose implementation lands in a
separate PR, to back a single criterion, this test is restated as a direct
inspection of declared visibility, which needs no new tooling and
demonstrates the same privacy boundary FR-068-AC-2 cares about: whether
these three types' state is reachable from outside `check` at all, not
merely which diagnostic a particular external-crate build produces. Scope:
FR-068-AC-2.

**Amended by FR-087 (owner ruling on QSL-158, 2026-09-21): `CheckedPackage`
relocates out of `check` into layer-4 `package`.** This test's own privacy
claim for `CheckedPackage` is superseded in place, not merely narrowed:
after FR-087's implementation, `CheckedPackage` is no longer defined in
`check` at all, so steps 1-2 and 4 below no longer have a `check`-module
`CheckedPackage` to inspect. FR-087-AC-1/TC-243 restate this test's own
reasoning (constructor/field privacy plus accessor-only reachability) for
`CheckedPackage` at its new site in `package`, extending it from three
types to five. This test's remaining scope, post-FR-087, is
`CheckedExpression` and `CheckedFunction` only, which stay in `check`
unchanged.

## Test Procedure

1. Read `check`'s definitions of `CheckedPackage`, `CheckedExpression` and
   `CheckedFunction`. For each type, record the declared visibility of every
   field and of the type's constructor (the implicit struct-literal
   constructor, since none of the three declares an explicit `new`):
   `pub`, `pub(crate)`, `pub(super)`, `pub(in path)`, or no qualifier
   (private).
2. Confirm every field and constructor recorded in step 1 carries no
   visibility qualifier at all — private in the plain Rust sense, not merely
   `pub(crate)` or `pub(super)`.
3. Read `check`'s own accessor surface for these three types (the methods
   evaluation actually calls: name, body, slots, parameters, scope, dispatch
   tables) and confirm each field's state is exposed only through a named
   accessor method, never re-exported as `pub` under a different name or
   through a module-path trick.
4. Separately, read `value::expression`'s `CheckedPackage::call`,
   `CheckedPackage::evaluate` and the argument-admission logic (today's
   `validate`) and confirm every place they read one of these three types'
   state goes through an accessor method step 3 records, with no direct
   field access anywhere in `value::expression`.

## Expected Results

- Steps 1-2: every field and constructor of the three types is private, with
  no visibility qualifier of any kind; any field or constructor bearing
  `pub`, `pub(crate)`, `pub(super)` or `pub(in path)` fails this step and
  names the type and the qualifier found.
- Step 3: every accessor is a genuine method over private state, not a
  field re-exported under an alias; a field made reachable by any path other
  than a named accessor method fails this step.
- Step 4: every field read `value::expression` performs on these three
  types' state goes through an accessor method; a direct field access
  anywhere in `value::expression` fails this step and names the location.

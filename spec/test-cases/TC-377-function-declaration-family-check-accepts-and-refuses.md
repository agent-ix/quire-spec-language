---
id: TC-377
title: "Function declaration checking accepts a well-typed declaration, reports its calls, and refuses an ill-typed body"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-377: Function declaration checking accepts a well-typed declaration, reports its calls, and refuses an ill-typed body

## Description

Verify the behavior of `Value`'s function-declaration checking entry point
(`check::family::check_declaration_body`, the function the composed
checker's pre-migration per-declaration typing/definedness pass is replaced
by): given a real declaration, it accepts a well-typed body, correctly
reports every call the body makes (for the whole-package termination pass
that runs after it), and refuses a body whose type disagrees with the
declared result. This test verifies accept/refuse behavior, not the presence
or absence of any pre-migration symbol; see this test case's own Status
section for why.

## Test Procedure

1. Declare `f(x: Boolean) -> Boolean = x` and `g() -> Boolean = f(true)`.
   Check `g`'s body against both signatures.
2. Declare `g() -> Boolean = 1` (an `Integer` body against a declared
   `Boolean` result). Check `g`'s body.

## Expected Results

- Step 1: checking succeeds; the returned declaration reports exactly one
  call, whose callee is `f`'s signature index, and zero local evaluation
  slots (the declaration binds no `let`).
- Step 2: checking refuses with a type-mismatch cause.

## Status

**Backed.**
`check_declaration_body_accepts_a_well_typed_declaration_and_reports_its_calls`
and `check_declaration_body_refuses_an_ill_typed_body`
(`src/check/family.rs`, `checking_tests`), tagged
`#[trace("TC-377", "FR-065-AC-5")]`.

**Behavioral, not structural, per team-lead testing-policy ruling (QSL-148,
2026-09-21).** FR-065-AC-5 is worded as a symbol-absence test (a
grep-equivalent search of compiled symbols) plus a compile-fail fixture
(`E0004` on a reintroduced enum variant with no matching dispatch arm).
TC-164 verifies that shape directly. The underlying fact TC-164 verifies for
is true of the delivered code -- the composed checker's pre-migration
per-declaration entry point (the inline `Typer` construction and definedness
pass previously in `check/mod.rs`'s per-declaration loop, and `Typer::call`
for applications) is deleted, not merely hidden; `check_declaration_body`
and `check_application` are its sole replacements -- but this test case does
not re-verify that by symbol search, on explicit instruction: test what the
family check accepts and refuses, not which symbols exist or are absent.
TC-377 instead exercises `check_declaration_body` directly, showing it
performs the real per-declaration typing, definedness and call-reporting
work the composed checker used to.

Termination checking itself is not part of this entry point and is not
exercised by this test case -- see `check_declaration_body`'s own doc
comment in `src/check/family.rs` and this ticket's report for why a
whole-package, cross-declaration call-graph analysis cannot move into a
per-declaration `FamilyContract::check` call. `check::termination::check`
remains an unchanged, separate whole-package pass in `check/mod.rs`, run
after every declaration's `check_declaration_body` call, over the `calls`
this test case's step 1 shows being reported correctly.

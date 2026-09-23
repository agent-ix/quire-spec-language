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
3. Declare `v(o: Option[Integer]) -> Integer = value(o)` (a body that is
   well-typed but statically undefined: `value(o)` with no proved
   `present(o)` guarding it). Check `v`'s body.

## Expected Results

- Step 1: checking succeeds; the returned declaration reports exactly one
  call, whose callee is `f`'s signature index, and zero local evaluation
  slots (the declaration binds no `let`).
- Step 2: checking refuses with a type-mismatch cause.
- Step 3: checking refuses with a definedness cause naming the unproved
  `present` obligation, not a type-mismatch cause -- the body type-checks
  against the declared result and is refused only because that obligation
  is unproved.

## Status

**Backed for behavior (all three steps); tagged for this test case only,
not for FR-065-AC-5, which stays unbacked (PR #303 review, findings
1/2/8/N2).**
`check_declaration_body_accepts_a_well_typed_declaration_and_reports_its_calls`,
`check_declaration_body_refuses_an_ill_typed_body` and
`check_declaration_body_refuses_an_undefined_body` (step 3, added this
round) (`qsl-semantics/src/check/family.rs`, `checking_tests`), all tagged
`#[trace("TC-377")]`.

**Behavioral, not structural, per the
[testing-policy ruling](https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837)
(Peter, QSL-148, 2026-09-22, relayed by the QSL team lead).** FR-065-AC-5
is worded as a symbol-absence test (a grep-equivalent search of compiled
symbols) plus a compile-fail fixture (`E0004` on a reintroduced enum
variant with no matching dispatch arm) *and* requires that the composed
checker's input form-kind enum carry neither a function-declaration nor a
function-application variant. TC-164 targets the symbol-absence half
directly; see TC-164's own Status section. The enum-shape half is not met:
`Expression::Call` is still present in `Expression`, so FR-065-AC-5 itself
is unbacked regardless of test coverage -- see FR-065's own Status
section, AC-5 row, for the full reasoning. These three tests instead
exercise `check_declaration_body` directly, showing it performs the real
per-declaration typing, definedness and call-reporting work the composed
checker's pre-migration path used to -- genuine, valuable coverage of the
checking-decision half of this migration, but not a demonstration of
AC-5's own condition.

Termination checking itself is not part of this entry point and is not
exercised by this test case -- see `check_declaration_body`'s own doc
comment in `qsl-semantics/src/check/family.rs` and this ticket's report for why a
whole-package, cross-declaration call-graph analysis cannot move into a
per-declaration `FamilyContract::check` call. `check::termination::check`
remains an unchanged, separate whole-package pass in `check/mod.rs`, run
after every declaration's `check_declaration_body` call, over the `calls`
this test case's step 1 shows being reported correctly.

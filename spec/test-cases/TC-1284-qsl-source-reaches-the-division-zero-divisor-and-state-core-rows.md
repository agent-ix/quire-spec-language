---
id: TC-1284
title: "QSL source reaches the division, zero-divisor and state-core rows"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-495
    type: verifies
---
# TC-1284: QSL source reaches the division, zero-divisor and state-core rows

## Description

Verify that each QSpec scalar row named by FR-495 is reachable from QSL source
with the two observations the requirement states: the check refusal under
`CheckMode::Linked` and the kernel outcome under `CheckMode::Kernel`; and that a
state-core declaration is refused as a declaration before any of its
expressions is classified; and that an integer division over unbounded
`Integer` operands settles `requires-bound` against a finite backend without
narrowing. Scope: FR-495-AC-1 to FR-495-AC-4, FR-495-AC-6, FR-495-AC-7 and,
through QSpec TC-266, FR-495-AC-5.

## Test Procedure

1. Under `quire.value.complete/v1` and each of the three integer-division
   definitions, check the functions `s1` to `s9` of FR-495-AC-1 under
   `CheckMode::Linked`.
2. Check the bounded-result forms of QSpec TC-192 DIV-05, DIV-06, DIV-12,
   DIV-15 and DIV-16 under `CheckMode::Kernel`, evaluate each with the operands
   and `ScalarLimitsV1` tuple the row states, once per division definition, and
   compare the outcome, cause, `expected` domain and charge point with the row.
3. Check the divisions of FR-495-AC-3 under `CheckMode::Linked`, then check
   the same bodies under `CheckMode::Kernel` and evaluate them at a zero
   divisor with the limit tuples of TC-185 D06 and D15, TC-192 DIV-01 and
   DIV-11 and TC-187 U27.
4. Under a header profile naming `quire.state.core/v1` alone, check the
   functions and the `invariant`, `pre` and `post` clauses of FR-495-AC-4; then
   check the same function bodies under `quire.value.complete/v1` and evaluate
   them.
   Also check `7 / 3 = rational(7, 3)` as a clause body under each profile, with and
   without a rational type expected (FR-495-AC-6).
5. Run QSpec TC-192 DIV-14 through the registered SMT backend, once with a cvc5
   provider in the lock and once without.
6. Under each division definition, check `q` of FR-495-AC-7 and its `rem` and
   `mod` variants; read each operator's requirement record; write the request
   with only a test descriptor advertising (`value-validity`, `bounded`) and a
   `requires-bound` disposition, once with no SMT provider in the lock, and
   settle each item.

Tag the test `#[trace("FR-495-AC-1", "FR-495-AC-2", "FR-495-AC-3", "FR-495-AC-4", "FR-495-AC-6", "TC-1284")]`; step 5 additionally carries `FR-495-AC-5`, `QSpec-FR-147-AC-7` and `QSpec-FR-147-AC-8` and depends on the provider; step 6 carries `FR-495-AC-7`, `QSpec-FR-147-AC-3` and `QSpec-FR-147-AC-8`.
Vectors are referenced by QSpec row ID, never copied.

## Expected Results

- Step 1: `s1` to `s5` check; `s6`, `s7` and `s8` refuse
  `undefined_expression`/`unproved-range` at the operator; `s9` refuses
  `undefined_expression`/`unproved-nonzero`.
- Step 2: every row gives the outcome its QSpec row states; the refused rows
  expose no quotient or remainder and charge nothing after the domain charge.
- Step 3: each division refuses `undefined_expression`/`unproved-nonzero` at
  the operator under `CheckMode::Linked` unless its divisor range excludes zero
  or an exact nonzero guard holds; under `CheckMode::Kernel` each zero divisor
  evaluates to `Undefined::DivisionByZero` (`division-by-zero`) or to the
  `incomplete` outcome of its row, with no value.
- Step 4: each function declaration refuses
  `unsupported_construct`/`declaration-form` once and none of its operators is
  refused as an expression; each clause body refuses
  `unsupported_construct`/`expression-form` at its operator; under complete
  value the function bodies check and evaluate. The `7 / 3` clause body
  refuses `ill_typed`/`type-mismatch` at the `/` under state core; under complete
  value it checks only with the rational type expected.
- Step 5: with a provider, the holding obligation is `proved` and the violated one `refuted`
  with a replayable counterexample; without one the row is reported pending on the provider.
- Step 6: each record's extent is `Unbounded` over `a` and `b`; each item is
  written with `finite_bound_available` true and settles `requires-bound` with
  no harness and no evaluation, with and without an SMT provider; no operand
  is narrowed to an `Int[..]`.

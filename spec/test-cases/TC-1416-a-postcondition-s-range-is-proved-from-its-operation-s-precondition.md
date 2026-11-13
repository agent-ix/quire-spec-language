---
id: TC-1416
title: "A postcondition's range is proved from its operation's precondition"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-577
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-121
    type: verifies
---
# TC-1416: A postcondition's range is proved from its operation's precondition

## Description

Verify that the `test/bank` withdrawal postcondition
`self.balance = pre(self.balance) - amount` checks under its sibling
precondition `amount <= self.balance`. Without that precondition it refuses
at the difference, and the refusal carries the operand, the intervals and
the region through `qsl_replay::call_site`. Also verify each fact rule of
FR-577 (strict, flipped, negated, repeated, joined, cross-clause and
unreachable facts), each form that yields or transfers no fact, the
redefinition-family and context exclusions, and evaluation outside the
precondition.

Scope: FR-577-AC-1 to FR-577-AC-35 (AC-29 retired); FR-121-AC-37.

## Test Procedure

Fixture: FR-577's bank fixture and bank unit, and `Debited1`, as its
Acceptance Criteria preamble writes them. The unit selects the package as
`model Bank = ...`. Every compile of the bank unit runs through `qsl_replay::call_site` with
the `Bank::Account::withdraw` operation selection. Steps 3 and 12 compile a
unit that declares only a function through `qsl_replay::call_site` with that
function's selection (`g`, `f`).

1. Compile the bank unit (AC-1). Compile it with `CanAfford` moved after
   `Debited` and compare clause node identities (AC-2).
2. Compile the bank unit without `CanAfford` (AC-3, AC-4, FR-121-AC-37).
3. Compile a unit declaring only FR-577-AC-5's function `g`, with the
   function selection `g` (AC-5).
4. Compile each precondition-fact variant: AC-6 (strict, `Debited1`),
   AC-7 (flipped), AC-8 (negated), AC-9 (repeated pair in one clause),
   AC-10 (repeated pair across two clauses).
5. Compile each join variant: AC-11 (two bounds, larger `c` kept) and AC-12
   (a bound on one alternative only).
6. Compile the in-clause guard variants: AC-13 (`pre` read guard) and AC-14
   (`post` read guard).
7. Compile the unrecognised shapes: AC-16 (equality), AC-17 (arithmetic
   operand), AC-18 (sum ordering with `Credited`), AC-19 (transitive chain).
8. Compile the binder variants: AC-20 (`let` binder, not an alias) and AC-21
   (`let` alias of `self.balance`).
9. Compile the unreachable variants: AC-22 (one clause) and AC-23 (empty
   intersection across two clauses).
10. Compile the context and family variants: AC-15 (`deposit`), AC-24 and
    AC-25 (`Premium` inheriting `withdraw`), AC-26 and AC-27 (`Premium`
    redefining `withdraw`).
11. Compile the non-postcondition targets: AC-28 (another precondition),
    AC-30 (a kept `requires`). AC-29 is retired.
12. Compile a unit declaring only FR-577-AC-31's function `f`, with the
    function selection `f` (AC-31).
13. From step 1's package, replay a `Debited` envelope through
    `qsl_replay::replay_state_clause` with pre `balance = 3`, `amount = 5`
    and post `balance = 0` (AC-32).
14. Build a protocol over step 1's package whose attempt of `withdraw` lists
    `contracts [Debited]` only, and explore it from a state with
    `balance = 3` and `limit = 0`, applying the attempt with `amount = 5`
    (AC-33).
15. Compile the bank unit over the fixture whose `withdraw` returns a Boolean
    `result`, with and without `CanAfford` (AC-34).
16. Compile the AC-35 function `f` for each of its three guard and value
    pairs, with the function selection `f`.

Tag each test `#[trace("TC-1416", "FR-577-AC-n")]` for the criterion it
checks, and step 2's region assertion also
`#[trace("TC-1416", "FR-121-AC-37")]`.

## Expected Results

- Step 1: both compile; `pre(self.balance) - amount` is typed
  `Int[0, 100]`; the two compiles give equal clause node identities.
- Step 2: refused at `pre(self.balance) - amount` with required `[0, 100]`
  and proved `[-100, 100]`. `CallSiteRefusal::Compile` has code
  `undefined_expression`, its message contains, in order,
  `pre(self.balance) - amount`, `[0, 100]` and `[-100, 100]`, and its
  `region` spans exactly the bytes of `pre(self.balance) - amount`.
- Step 3: refused at `n - 1`; the message contains `[-inf, +inf]`.
- Step 4: all five compile.
- Step 5: AC-11 refuses at `(pre(self.balance) - amount) - 1`; AC-12 refuses
  at `pre(self.balance) - amount`.
- Step 6: AC-13 compiles; AC-14 refuses at `pre(self.balance) - amount`.
- Step 7: AC-16, AC-17 and AC-19 refuse at `pre(self.balance) - amount`;
  AC-18 refuses at `pre(self.balance) + amount`.
- Step 8: AC-20 refuses at `pre(self.balance) - x`; AC-21 compiles.
- Step 9: both refuse at `pre(self.balance) - amount`.
- Step 10: each refuses at `pre(self.balance) - amount`.
- Step 11: AC-28 refuses at `self.balance - amount` in `Fits`; AC-30 refuses
  at `pre(self.balance) - amount`.
- Step 12: compiles.
- Step 13: the evaluation refuses `IntegerOutOfDomain` at
  `pre(self.balance) - amount`; the replay is `inconclusive` with cause
  `NoValue`, not a reproduced violation.
- Step 14: one `ModelFinding::ContractUndetermined` for `Debited` per
  candidate post-state of the frame, 101 in all (one per `balance` value in
  `[0, 100]`), and no successor for that application.
- Step 15: with `CanAfford` the unit checks; without it, it refuses at
  `pre(self.balance) - amount`.
- Step 16: each pair refuses at its value expression (`a + b`, `b + a`,
  `-b + a`).

Every refusal above is `undefined_expression`/`unproved-range` unless the
step names another.

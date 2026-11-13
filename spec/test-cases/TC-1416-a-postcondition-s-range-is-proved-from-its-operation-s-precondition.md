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
at the difference, and the refusal carries the operand and its region
through `qsl_replay::call_site`. Also verify the fact forms that do not
transfer and the evaluation of an invocation outside the precondition.

Scope: FR-577-AC-1 to FR-577-AC-6; FR-121-AC-37.

## Test Procedure

Fixture: a domain package for `test/bank` declaring model `Bank`, object
type `Account` with `balance: Int[0, 100]`, population `accounts` of
`Account`, and operation `withdraw(amount: Int[0, 100])` with frame
`modifies [Account/balance]`. The unit selects it as `model Bank = ...` and
authors `CanAfford`, `Debited` and `NonNegative` as FR-577-AC-1 writes them.
Every step compiles through `qsl_replay::call_site` with the
`Bank::Account::withdraw` operation selection.

1. Compile the unit. Compile it again with `CanAfford` moved after `Debited`.
2. Compile the unit without `CanAfford`.
3. Compile the unit with `CanAfford`'s body `amount < self.balance`, then
   `self.balance >= amount`. Compile the step 2 unit with `Debited`'s body
   `amount <= pre(self.balance) implies self.balance = pre(self.balance) - amount`.
4. Compile the step 2 unit with `Debited`'s body
   `amount <= self.balance implies self.balance = pre(self.balance) - amount`.
   Add `deposit(amount: Int[0, 100])` to the package and compile the unit
   with `CanAfford` on `Bank::Account::deposit`.
5. Compile the unit with `CanAfford`'s body `amount = self.balance`, then
   `amount + 0 <= self.balance`. Compile it with `CanAfford`'s body
   `self.balance + amount <= 100` and `Debited` replaced by FR-577-AC-5's
   `Credited`.
6. From step 1's package, replay a `Debited` envelope through
   `qsl_replay::replay_state_clause` with pre `balance = 3`, `amount = 5`
   and post `balance = 0`.

Tag the tests `#[trace("TC-1416", "FR-577-AC-n")]`, and step 2's region
assertion also `#[trace("TC-1416", "FR-121-AC-37")]`.

## Expected Results

- Step 1: both compile. `pre(self.balance) - amount` is typed
  `Int[0, 100]`, the first compile's clauses are `CanAfford` then `Debited`, and the two
  compiles give equal clause node identities.
- Step 2: `CallSiteRefusal::Compile` with code `undefined_expression`. Its
  `region` spans exactly the bytes of `pre(self.balance) - amount`. Its
  message contains `unproved-range`, `pre(self.balance) - amount`,
  `[0, 100]` and `[-100, 100]`.
- Step 3: all three compile.
- Step 4: each refuses `undefined_expression`/`unproved-range` at
  `pre(self.balance) - amount`.
- Step 5: the first two refuse at `pre(self.balance) - amount`. The third
  refuses at `pre(self.balance) + amount`. Each is
  `undefined_expression`/`unproved-range`.
- Step 6: `inconclusive` with cause `NoValue`, not a reproduced violation.

---
id: TC-1336
title: "A quantity integer power lowers to quantity.pow"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-1336: A quantity integer power lowers to quantity.pow

## Description

Verify that `e ^ n` over a non-affine quantity and a signed integer literal
parses, checks to the compound unit of the operand's exponents times `n`,
lowers to one `quire.op.quantity.pow` node and evaluates through the owned
quantity core with QSpec FR-142's charges; that a negative exponent carries a
nonzero obligation on the base; and that an affine, carried or non-quantity
operand and a non-literal exponent are refused. Scope: FR-093-AC-58 and the
`e ^ n` row of FR-091-AC-3 (QSpec TC-187 U12, U13 and U21).

## Test Procedure

1. Declare the units of FR-093-AC-58. Check, lower and evaluate `w1` to `w7`;
   read each `quire.op.quantity.pow` node, its arguments and its
   `result_type`, and pass each package to IR's v2 reader.
2. Check `w8`, then evaluate its first power node with QSpec TC-187 U13's
   tuple as the meter's remaining budget at that node.
3. Check `r1` and `r2` under `CheckMode::Linked`; check `r2` under
   `CheckMode::Kernel` and evaluate its first power node with U21's
   `L(1,0,0,1,1,0)`, then `L(1,0,0,1,0,0)`, as the remaining budget.
4. Check `r3`, `r4` and `r5`, counting the charges made.
5. Parse `d ^ k`, `d ^ (2)`, `d ^ 2 ^ 3` and `d ^ rational(1, 2)`, and map
   `2 m ^ 2` and `-1 m ^ 2` through S2.

Tag the test `#[trace("FR-093-AC-58", "TC-1336")]`; step 5's mapping also
carries `FR-091-AC-3`. The expected outcomes of U12, U13 and U21 are QSpec
TC-187's, referenced by row ID.

## Expected Results

- Step 1: every function checks and evaluates to `true`; `w1`'s power is one
  `quire.op.quantity.pow` node over the `2 m` literal and the `Integer` literal
  `"2"`, sharing the product's compound-unit node, with no law, mode, member or
  leaves; `w3`'s and `w4`'s powers have the dimensionless compound-unit node;
  `w5`'s power has the unit `s` to the power 1; IR's v2 reader admits each
  package.
- Step 2: `w8` checks; the power is U13's `incomplete` record, with
  `next_charge` `36893488147419103232` reported exactly, before the power is
  computed.
- Step 3: `r1` and `r2` refuse `undefined_expression`/`unproved-nonzero` at the
  first `^` under `CheckMode::Linked`; under `CheckMode::Kernel` the zero base
  under `-1` is `Undefined::DivisionByZero` after one `unit.identity-read`,
  then U21's `incomplete` record at `unit.identity-read`.
- Step 4: `r3` refuses `ill_typed`/`operator-ineligible` at the `^` with no
  charge and no node; `r4` refuses `ill_typed`/`type-mismatch` at `i`; `r5`
  refuses `ill_typed`/`type-mismatch` at the power.
- Step 5: each malformed exponent refuses `invalid_syntax`/`unexpected-token`
  before S2 (at the second `^` for `d ^ 2 ^ 3`); `2 m ^ 2` maps to
  `Power{base: 2 m, exponent: 2}` and `-1 m ^ 2` to `Negate(Power{1 m, 2})`.

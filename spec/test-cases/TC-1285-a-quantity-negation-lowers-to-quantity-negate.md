---
id: TC-1285
title: "A quantity negation lowers to quantity.negate"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-093
    type: verifies
---
# TC-1285: A quantity negation lowers to quantity.negate

## Description

Verify that a unary `-` over a non-affine quantity checks as the operand's own
type, lowers to one `quire.op.quantity.negate` node and evaluates to the exact
negation; that a direct or nested negation of a nonzero quantity literal
proves a quantity divisor nonzero; and that negated zero, a negated parameter,
and affine-unit or non-quantity operands are refused. Scope: FR-093-AC-49
(QSpec TC-187 U39, FR-146-AC-9).

## Test Procedure

1. Declare the units of FR-093-AC-49. Check, lower and evaluate the functions
   `a`, `b`, `c`, `c2`, `z`, `q`, `r`, `k`, `k2` and `v2`; read each negate
   node and pass each package to IR's v2 reader.
2. Check `n`, `n2`, `k3`, `k4` and `b2`, counting the charges made.

Tag the test `#[trace("FR-093-AC-49", "TC-1285")]`. The expected values are
QSpec TC-187 U39's, referenced by row ID.

## Expected Results

- Step 1: every function checks; `a` is `-2 m`, `b` is `-1 m`, `c` and `c2` are
  `-3 ms` in `ms` and lower to the same nodes, `z` is `0 m`, `q` is `true` over
  two nested negate nodes, `r` has a `quire.op.quantity.eq` over `d` and the
  negate node, `k` is `true` at `d = 6 ms` with both dimensionless sides equal
  to `-2`, `k2` is `true` at `d = 6 ms` with both dimensionless sides equal to
  `2`, and `v2` negates in the compound unit. Each negate node has the
  operand's unit node as `result_type` and no law, mode, member or leaves; no
  unit edge is traversed; IR's v2 reader admits each package. `k`'s divisor is
  the direct negation of the nonzero literal `3 ms`; `k2`'s divisor is a nested
  negation rooted in that literal.
- Step 2: `n` and `n2` refuse `ill_typed`/`operator-ineligible` at the negation
  with no charge and no node; `k3` refuses `undefined_expression`/
  `unproved-nonzero` at division by `-(0 ms)`; `k4` refuses the same cause at
  division by `-d` for `d: Quantity<ms>`; `b2` refuses
  `ill_typed`/`type-mismatch` at the operand. No refused division emits a
  division or negation node.

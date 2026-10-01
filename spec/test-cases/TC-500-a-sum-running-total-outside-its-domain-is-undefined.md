---
id: TC-500
title: "A sum seed or running total outside its domain is a located undefined outcome"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-096
    type: verifies
---
# TC-500: A sum seed or running total outside its domain is a located undefined outcome

## Description

Verify FR-096's `sum` rule (QSpec FR-044, FR-145): at S6a a `sum<N>` whose
seed or running total leaves `N`'s domain returns the kernel undefined
reason `Undefined::SumOutOfDomain` at the summand or the `sum` node, never
the `integer_out_of_domain` refusal. This catches an evaluator that checks
only the final total, one that refuses instead of returning undefined, and
one that keeps adding after the failed decision.

Scope: FR-096-AC-14.

## Test Procedure

1. Check `sum<Small>(x in q: x)`, with `type Small = Int[0, 3]` and `q` of
   `Sequence<Int[0, 3]>[0, 2]`, under `CheckMode::Kernel`. Evaluate it at
   S6a for `q` holding `2, 2`, recording the meter's charges.
2. Evaluate the same checked expression for `q` holding `1, 2`.
3. Check `sum<Small>(x in q: x)` with `q` of `Sequence<Int[0, 9]>[0, 2]`
   under `CheckMode::Kernel`, and evaluate it for `q` holding `5, 0`.

Tag the tests `#[trace("TC-500", "FR-096-AC-14")]`.

## Expected Results

- Step 1: `FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))`,
  `Evaluation.location` the `sum` node, no refusal record, and no charge
  after `integer-arithmetic.arithmetic`. Not
  `Outcome::Refused(Refusal::IntegerOutOfDomain)`.
- Step 2: completes with `3`.
- Step 3: `FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))`,
  `Evaluation.location` the summand node, with no addition charged.

## Status

Passing locally. S6a's `sum` returns
`Undefined::SumOutOfDomain` for a seed or running total outside `N`'s
domain: located at the summand node for a seed and at the `sum` node for an
addition, with no charge after the failed decision and no final-total
refusal.

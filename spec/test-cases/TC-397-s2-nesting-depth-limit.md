---
id: TC-397
title: "S2 bounds nesting by its explicit depth limit and refuses past it without overflowing the stack"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-397: S2 bounds nesting by its explicit depth limit and refuses past it without overflowing the stack

## Description

Verify that S2's recursion is bounded by its explicit nesting-depth limit
(ADR-011 §2.3 Limits) and that exceeding it is a limit refusal, not a
truncated form, not a different cause and not a stack overflow.

Scope: FR-091-AC-8.

## Test Procedure

1. Choose an S2 nesting-depth bound `L` (for example 32) well below S1's
   own depth ceiling.
2. Run S2 on a unit whose one function body is `L` nested `not` operators
   around `a`.
3. Run S2 on the same body with `L + 1` nested `not` operators.
4. Run S2 on a body nested to the deepest level S1 admits under its default
   limits, with S2's bound still `L`.

Tag the test `#[trace("FR-091-AC-8", "TC-397")]`.

## Expected Results

- Step 2 builds a form.
- Step 3 gives a limit refusal naming limit kind nesting depth, bound `L`
  and the span of the `not` construct at depth `L + 1`, with no parsed
  unit.
- Step 4 gives the same kind of limit refusal and the test process does not
  abort.

## Status

Planned; no test backs this case.

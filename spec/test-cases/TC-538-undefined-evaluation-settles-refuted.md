---
id: TC-538
title: "An undefined-evaluation counterexample settles refuted with its cause"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-127
    type: verifies
---
# TC-538: An undefined-evaluation counterexample settles refuted with its cause

## Description

Verify that `model_check` settles an undefined claim evaluation `refuted`
through the existing V-4 row, carrying `UndefinedEvaluation{where, cause}`
as its counterexample's `kind`, and that a replay disagreement settles
`inconclusive`.

Scope: FR-127-AC-6.

## Test Procedure

1. Settle TC-537 step 3's first outcome with its FR-128 replay result.
2. Settle the same outcome with its payload's cause changed to
   `precondition-false`, with that payload's replay result.

Tag the test `#[trace("TC-538", "FR-127-AC-6")]`.

## Expected Results

- Step 1: `refuted`, basis `decisive-counterexample`, `TerminalValue::Refuted`,
  category violation; the record's counterexample has `kind:
  UndefinedEvaluation{where: 2, cause: division-by-zero}`.
- Step 2: `inconclusive`, `unsettled`, `Inconclusive(ReplayParity)`.

---
id: TC-053
title: "Check Boolean guard-fact soundness against an independent truth table"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-016
    type: verifies
---
# TC-053: Check Boolean guard-fact soundness against an independent truth table

## Description

Property, priority P1. Verifies FR-016-AC-1 and FR-016-AC-7 for the bounded native
presence-fact derivation. This is planned evidence, not a claimed model-checking run.

## Test Procedure

Generate a bounded family from present(self.parent), self.n=0, true, false and
their negations, then every and/or/implies pairing of those base formulas.
Add the explicit common-fact join and contradictory-presence examples. For each
guard, check guard implies deref(value(self.parent)).n < self.n through the real
native frontend. Independently evaluate the test's small Boolean formula model
under all four assignments of parent presence and n=0, without using native AST
evaluation, proof facts, proof graph or IR evaluation as the oracle.

## Expected Results

Every accepted clause has no assignment where its guard is true and parent is
absent. Any rejected supported clause is classified as undefined_expression,
not a model setup failure. The direct-presence, impossible-path and common-fact
join controls are accepted; a guard that can be true with absent parent refuses.
The test records how many formulas and assignments were exercised. Conservative
refusal of another logically safe formula is permitted; vacuous refusal of all
formulas cannot satisfy the mandatory positive controls.

---
id: TC-809
title: "Evaluation selects the dispatched body from the linked table by the receiver's most-specific type"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-809: Evaluation selects the dispatched body from the linked table by the receiver's most-specific type

## Description

Verify that spine `compile` links a dispatched call's family at S3 and that
S6a runs the body the linked table selects for the receiver's most-specific
type. Scope: FR-302-AC-6.

Catches an evaluator that always runs the declaring type's body, runs the
first candidate found, or ignores the receiver's most-specific type.

## Test Procedure

1. Compile a unit whose precondition calls `size()` on `self`, where
   `size` has an `operation-body` at `A` returning 1 and a redefinition
   at `B` (`supertypes: [A]`) returning 2, and the precondition is
   `self.size() == 2`.
2. Evaluate the precondition with a receiver of most-specific type `B`,
   then with a receiver of type `A`.

## Expected Results

1. The compile succeeds.
2. The `B` receiver gives `true`, the `A` receiver gives `false`.

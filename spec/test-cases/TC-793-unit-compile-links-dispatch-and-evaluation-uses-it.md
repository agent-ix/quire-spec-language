---
id: TC-793
title: "Unit compile links dispatch and evaluation selects from the linked table"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-302
    type: verifies
---
# TC-793: Unit compile links dispatch and evaluation selects from the linked table

## Description

Verify that spine `compile` links a dispatched call's family at S3 and that
S6a selects through it. Scope: FR-302-AC-1.

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

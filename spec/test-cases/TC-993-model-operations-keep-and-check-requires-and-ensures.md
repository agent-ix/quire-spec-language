---
id: TC-993
title: "Model operations keep and check their own requires and ensures clauses"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-103
    type: verifies
---
# TC-993: Model operations keep and check their own requires and ensures clauses

## Description

Verify that intake keeps a domain-package operation's `pre` and `post` clause
text as its `requires` and `ensures` clauses with source identity, and that
the checker evaluates them: a `requires` clause that reads `pre(e)` refuses,
and an `ensures` clause is bound as the operation's postcondition. These are
QSpec [TC-235](ix://agent-ix/quire-specification/TC-235) M06 run 7 and M15
run 6. Scope: FR-103-AC-7 and FR-103-AC-8.

## Test Procedure

1. Read TC-235's K2 domain package. Give `OrderLifecycle/place` the `post`
   entry `self.placed` (language `quire`) and read the package (M06 run 7).
2. Inspect the `OperationMemberRecord` of `place`.
3. Give `OrderLifecycle` field `cancelled` and `place` the `pre` entry
   `not self.placed` and the `post` entry
   `self.placed and not self.cancelled`. Select the package in a `1-draft`
   unit and compile it.
4. Replace the `pre` entry with `pre(self.placed) = false` and compile again
   (M15 run 6).
5. Replace the `post` entry with `self.placed and not pre(self.placed)` and
   keep the `pre` entry of step 3, and compile again.

## Expected Results

1. The package admits after the same 21 charges as K2 and adds no declaration.
2. `place` holds one `ensures` clause: language `quire`, text `self.placed`,
   position 0, and the operation key, artifact id and span of its `origin`. It
   holds no `requires` clause.
3. The compile succeeds.
4. The compile refuses with one refusal, `wrong_snapshot`/`forbidden-pre-read`,
   naming `ix://test/orders/OrderLifecycle/place` at `Operation.pre[0]`, and
   produces no checked package.
5. The compile succeeds: an `ensures` clause may read `pre(e)`.

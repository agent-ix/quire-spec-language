---
id: TC-786
title: "run's exit statuses come from the exit function, with undefined at 10"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-100
    type: verifies
---
# TC-786: run's exit statuses come from the exit function, with undefined at 10

## Description

Verify that every exit status QSL `command`'s run returns is FR-285's exit code of the outcome's category.

Scope: FR-100-AC-10, FR-100-AC-11.

## Test Procedure

1. With `type Pos = Int[1, 9]` checked under `CheckMode::Kernel`, evaluate `sum<Pos>(x in q: x)` through S6a for `q` of `Sequence<Int[1, 9]>[0, 2]`, once empty and once holding `4`, and convert each outcome with the outcome mapping; convert a `FamilyResult::Undefined` with reason `precondition-false` the same way.
2. Run FR-100-AC-4's `invalid_runtime_input` cases, FR-100-AC-5's record-result function, FR-100-AC-6's `work_units` 0 call, and a call that returns `CallFailure::Fault`.
3. Call `execute` over each step 1 and 2 input and map its outcome through FR-285.

Tag the tests `#[trace("TC-786", "<AC id>")]`.

## Expected Results

- Step 1: the empty `q` gives `Outcome::Undefined(Undefined::SumOutOfDomain)` with `Evaluation.location` at the `sum` node, rendered `{"kind": "undefined", "reason": "sum-out-of-domain"}`, exit 10; `q` holding `4` completes with integer `"4"`, exit 0; the family outcome writes kind `undefined` with reason `precondition-false`, exit 10.
- Step 2: exits 20, 21, 22 and 30.
- Step 3: each exit code equals the command's.

---
id: TC-757
title: "A cancelled operation stops within one charge and emits nothing"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-276
    type: verifies
---
# TC-757: A cancelled operation stops within one charge and emits nothing

## Description

Verify `Cancel`: a pre-cancelled handle, a cancellation from another thread during `check` and `execute`, a cancellation inside a two-item `analyze`, the category of every cancelled failure, and the evaluator result beneath a cancelled `execute`.

Scope: FR-276-AC-1 to FR-276-AC-6.

## Test Procedure

1. Call each lifecycle operation over its TC-755 input with a handle cancelled with `Requested` before the call.
2. Generate a source of 200,000 declarations. Start `check` with a test work-meter observer; when the observer has seen 1,000 charges, cancel a clone of the handle with `Deadline` from a second thread.
3. Run `execute` of a function whose evaluation charges more than 10,000,000 work units, every accounting limit `u64::MAX`, cancelled from a second thread after 1,000 charges.
4. Run `analyze` over two items with a test engine that settles the first and blocks on the second until the handle is cancelled; cancel it.
5. Map `StageFailure::Cancelled(Requested)`, `StageFailure::Cancelled(Deadline)` and `CallFailure::Cancelled(Requested)` through FR-285.
6. Cancel a handle with `Requested`, install it in a meter, and `call` FR-262-AC-1's `count` with `3` over that meter; then `execute` the same function with the same handle.

Tag the tests `#[trace("TC-757", "<AC id>")]`.

## Expected Results

- Step 1: each `Staged` operation returns `StageFailure::Cancelled(Requested)` with no output; `execute` returns `CallFailure::Cancelled(Requested)`.
- Step 2: `check` returns `StageFailure::Cancelled(Deadline)` and no `CheckedPackage`; the observer records at most one charge after the cancellation.
- Step 3: `execute` returns `CallFailure::Cancelled(Requested)` and no value.
- Step 4: the first record is unchanged; the second is `incomplete` with cause cancelled.
- Step 5: each has category incomplete and exit code 22.
- Step 6: `call` returns an evaluation whose outcome is `incomplete` at the `function.call` charge, `limit_kind` `work_units`, `limit` equal to `consumed`, with no admitted charge, and `tripped()` returns `Requested`; `execute` returns `CallFailure::Cancelled(Requested)` and no value.

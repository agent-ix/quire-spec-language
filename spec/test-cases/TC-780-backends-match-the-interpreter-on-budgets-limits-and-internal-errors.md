---
id: TC-780
title: "Backends match the interpreter on budgets, limits and internal errors, and the outcome names no backend"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-295
    type: verifies
---
# TC-780: Backends match the interpreter on budgets, limits and internal errors, and the outcome names no backend

## Description

Verify EB-4 rules 4 to 6 and the absence of a backend member in the outcome, on each linked non-interpreter backend.

Scope: FR-295-AC-2 to FR-295-AC-4.

## Test Procedure

1. Run FR-100-AC-6's `seven` with `work_units` 0 to 5, and a property-generated set of programs and accounting limits, on each backend and on the interpreter.
2. Call FR-262-AC-1's `count` with `100000` on each backend, on a thread with a 512 KiB stack, with `work_units` sized for it and again with `work_units` one below the run's measured spend; call `prepare` with a `PrepareLimits` code-size bound of 1.
3. Inject an internal error into a backend and call an entry; serialize one call's outcome document (FR-286) on the interpreter and on each backend.

Tag the tests `#[trace("TC-780", "<AC id>")]`.

## Expected Results

- Step 1: each `Incomplete` names the same limit at the same charge point as the interpreter's.
- Step 2: each backend returns the interpreter's outcome for both budgets and the thread does not overflow; `prepare` returns a `Limit` failure naming the code-size bound.
- Step 3: the call returns `Fault`, exit 30, and no value; the outcome documents are byte-equal.

---
id: TC-784
title: "One Value-family source runs through check, package and execute with documents and exit codes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-299
    type: verifies
---
# TC-784: One Value-family source runs through check, package and execute with documents and exit codes

## Description

Verify that QSL's library operations serve the source the driver's `quire check`, `compile`, `run` and `prove` use, with and without a supplied library, and that cancellation stops them.

Scope: FR-299-AC-1 to FR-299-AC-4.

## Test Procedure

1. Over a `1-draft` source declaring `p(x: Int[0, 9]): Boolean { x < 10 }` and `q(x: Int[0, 9]): Boolean { x < 5 }` with a claim on each, call `check`, then `package`, then `execute` of `p` with `x = 3` on the interpreter; compile the same source with the FR-278 composition; map each outcome's category through FR-285.
2. Serialize each step 1 outcome to its FR-286 document, twice.
3. Repeat step 1 over a source importing `test/geometry`, with that library supplied.
4. Call `check` over the step 1 source with its `Cancel` handle cancelled before the call.

Tag the tests `#[trace("TC-784", "<AC id>")]`.

## Expected Results

- Step 1: `check` succeeds, exit 0; `package`'s bytes equal the spine compile's; `execute` completes `true`, exit 0.
- Step 2: each document's `operation`, `category` and `artifacts` match its outcome, and the two serializations are byte-equal.
- Step 3: the same dispositions and exit codes as step 1.
- Step 4: `StageFailure::Cancelled`, exit 22.

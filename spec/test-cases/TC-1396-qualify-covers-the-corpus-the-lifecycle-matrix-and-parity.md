---
id: TC-1396
title: "qualify covers the corpus, the lifecycle matrix and parity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-567
    type: verifies
---
# TC-1396: qualify covers the corpus, the lifecycle matrix and parity

## Description

Verify that the `qualify` verdict requires all three parts of the final
path.

Scope: FR-567-AC-1 to FR-567-AC-4.

## Test Procedure

1. Run `qualify` over FR-353-AC-1's passing fixture corpus with the matrix
   cases and a linked test backend that delegates to the interpreter.
2. Repeat with the matrix case set replaced by one whose `check`/`refusal`
   case expects exit 21.
3. Repeat step 1 with FR-295-AC-1's rounding backend linked instead.
4. Repeat step 1 with no non-interpreter backend linked, and with a parity
   reference that returns `Fault`.

Tag the tests `#[trace("TC-1396", "<AC id>")]`.

## Expected Results

- Step 1: `complete-v1: qualified`, three parts passed, exit 0.
- Step 2: `not-qualified` naming `lifecycle-matrix/check/refusal`, exit 1,
  with the `corpus` part passed.
- Step 3: `not-qualified` naming `parity/jit` and its finding, exit 1.
- Step 4: parity `no-backend` and `complete-v1: qualified`, exit 0; then
  `reference-fault`, no verdict, exit 2.

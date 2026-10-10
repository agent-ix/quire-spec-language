---
id: TC-903
title: "A deep-input fuzz target drives the parser and the checker"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-356
    type: verifies
---
# TC-903: A deep-input fuzz target drives the parser and the checker

## Description

Verify that the deep-input fuzz target generates deeply nested sources and
that the parser and the checker answer every one without a panic, abort or
stack overflow.

Scope: FR-356-AC-7.

## Test Procedure

1. Run the fuzz target for 10,000 inputs, generating nested brackets, sums,
   `else if` chains, nested `let`s and nested `Option` types from 1 to
   100,000 levels deep.
2. Record each input's outcome from the S1 parser and the S3 checker.

The target is `fuzz/fuzz_targets/deep_input.rs`; run `make fuzz-deep-input`.
It raises every size and work limit to fit each input. Until ADR-030 slice 1
deletes the S2 forms and S3 checker depth caps, an input nested deeper than
either cap ends in that cap's limit outcome and does not yet reach the walks
past it.

## Expected Results

- Every input returns a result or a stated limit outcome naming its setting.
- No input panics, aborts or overflows the stack.


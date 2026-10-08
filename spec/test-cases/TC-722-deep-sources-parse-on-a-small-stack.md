---
id: TC-722
title: "Deep sources parse on a small stack under raised S1 limits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-256
    type: verifies
---
# TC-722: Deep sources parse on a small stack under raised S1 limits

## Description

Verify that the complete-V1 parser parses 100,000-deep inputs without native
recursion that grows with depth.

Scope: FR-256-AC-1.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Through the complete-V1 parser, with `s1.input_bytes`, `s1.tokens`,
   `s1.nodes` and `s1.work_units` raised to fit each input, parse a function
   body of: 100,000 nested parentheses around `1`; `not` applied 100,000
   times to `a`; a `+` chain of 100,000 terms; an `else if` chain of 100,000
   branches; and 100,000 nested `let … in`.

Tag the tests `#[trace("TC-722", "FR-256-AC-1")]`.

## Expected Results

- Step 1: each input parses, and the thread completes.

## Status

Passed locally: `qsl-cst/tests/it/deep_sources.rs`.

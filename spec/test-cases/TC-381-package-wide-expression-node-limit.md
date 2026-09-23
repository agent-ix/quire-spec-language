---
id: TC-381
title: "The expression-node limit bounds the whole checked package, not each declaration"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-062
    type: verifies
---
# TC-381: The expression-node limit bounds the whole checked package, not each declaration

## Description

Verify that the expression-node limit configured through
`CheckingLimits::new(nodes, depth)` is one budget shared by every declaration
in a checked package: two declarations that each fit under it are refused
together when their combined node count exceeds it. Scope: FR-062-AC-11.

## Test Procedure

1. Declare `a() -> Integer = 1 + 1`. Check a package holding `a` alone with
   `CheckingLimits::new(4, 128)`.
2. Declare `b() -> Integer = 1 + 1`. Check a package holding `a` and `b`
   with `CheckingLimits::new(100, 128)`.
3. Check the package holding `a` and `b` with `CheckingLimits::new(4, 128)`.

## Expected Results

- Step 1: the package is admitted.
- Step 2: the package is admitted.
- Step 3: the package is refused with
  `ResourceExhausted{stage: Typing, kind: Nodes, limit: 4}`; the reported
  limit is the caller's configured value, not a remaining amount.

## Status

Backed: `nodes_limit_is_enforced_across_the_whole_package_not_per_declaration`
(`qsl-eval/tests/it/total_functions.rs`), tagged `#[trace("TC-381", "FR-062-AC-11")]`.

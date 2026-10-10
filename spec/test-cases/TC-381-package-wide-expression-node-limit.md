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
`CheckingLimits::new(nodes)` is one budget shared by every declaration
in a checked package: two declarations that each fit under it are refused
together when their combined node count exceeds it. Scope: FR-062-AC-11.

## Test Procedure

1. Declare `a() -> Integer = 1 + 1`. Check a package holding `a` alone with
   `CheckingLimits::new(4)`.
2. Declare `b() -> Integer = 1 + 1`. Check a package holding `a` and `b`
   with `CheckingLimits::new(100)`.
3. Check the package holding `a` and `b` with `CheckingLimits::new(4)`.

`PackageDeclarations::check` reports a family's `StageFailure::Limit` as a
`CheckRefusal` whose cause is `CheckCause::ResourceExhausted` carrying the
stage, kind, bound and actual counter (ADR-013 §7 slice S-5b, FR-096); that
cause's code is always `stage_limit_exceeded` and its cause the limit kind's
(`check/refusal.rs`, `CheckCause::code` and `CheckCause::cause`).
The test asserts the full cause (`Typing`, node count, bound 4, actual 5),
which fixes `stage_limit_exceeded`/`node-count-exceeded`; it does not compare
the code string itself.

## Expected Results

- Step 1: the package is admitted.
- Step 2: the package is admitted.
- Step 3: the package stops with `StageFailure::Limit` of kind node count,
  bound 4, code `stage_limit_exceeded`/`node-count-exceeded`; the reported
  bound is the caller's configured value, not a remaining amount.

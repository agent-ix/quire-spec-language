---
id: TC-399
title: "Source compiled through S1, S2 and the assembler checks and evaluates a called function"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-399: Source compiled through S1, S2 and the assembler checks and evaluates a called function

## Description

Verify the end-to-end path from complete-V1 source to a checked, callable
function through S1, S2 and the assembler. Also verify the assembler's
`aliases` output and its check-owned resolved signatures.

This catches an assembler that drops the alias, leaves a parameter type
unresolved, reorders functions, or builds a function that cannot be called
by name.

Scope: FR-091-AC-12, FR-091-AC-13.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Source: `type Digit = Int[0, 9];`,
   `function inc using v(x: Digit): Int[0, 10] pure { x + 1 }` and
   `function two using v(): Int[0, 10] pure { inc(1) }`.
2. Run `qsl_cst::parse`, then S2, then the assembler.
3. Read `aliases`, the order of `functions`, and each function's resolved
   signature.
4. Check the result with `PackageDeclarations::check`, link the checked
   graph into a `CheckedPackage`, and call `two` with no arguments through
   `CheckedPackage::call` with an unlimited meter.

Tag the test `#[trace("FR-091-AC-12", "FR-091-AC-13", "TC-399")]`.

## Expected Results

- `aliases` equals `[("Digit", Int(0..=9))]`.
- `functions` is `inc` then `two`. `inc`'s resolved signature has parameter
  type `Int(0..=9)`, and both results are `Int(0..=10)`.
- Checking succeeds.
- The call returns a completed outcome whose value is the integer `2`.

## Status

Planned; no test backs this case.

---
id: TC-404
title: "format takes the qsl-cst ParsedSource, formats complete-V1 source and refuses inadmissible input"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: verifies
---
# TC-404: format takes the qsl-cst ParsedSource, formats complete-V1 source and refuses inadmissible input

## Description

Verify that `format` and `format_with_limit` take the S1 output
`qsl_cst::ParsedSource` (ADR-011 §6.1 tool layer, §7.3 M-6a), that
`src/format.rs` does not depend on the arena parse, and that inadmissible
input is refused with a typed cause and no panic.

Scope: FR-003-AC-7, FR-003-AC-8.

## Test Procedure

1. Resolve every `use` edge and inline path in `src/format.rs`.
2. Parse, with `qsl_cst::parse`, complete-V1 source that holds a comment and
   a nested block. Confirm that the native arena parser refuses the same
   bytes.
3. Call `format` on the parsed source. Parse the output and format it again.
4. Call `format` and `format_with_limit` on a parse whose CST carries a
   recovery, and again on an admissible parse after `prepend_diagnostic`.

Tag the test `#[trace("FR-003-AC-7", "FR-003-AC-8", "TC-404")]`.

## Expected Results

- Step 1 finds no edge to the arena `syntax` or native `parser` modules,
  and `format`'s parameter type is `&qsl_cst::ParsedSource`.
- Step 3 keeps every token spelling in order and keeps the comment, and the
  second pass gives identical bytes.
- Every step-4 call returns a refusal with a typed cause and no string, and
  none panics.

## Status

Planned; no test backs this case.

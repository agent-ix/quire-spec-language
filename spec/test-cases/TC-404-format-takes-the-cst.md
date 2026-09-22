---
id: TC-404
title: "format takes the qsl-cst ParsedSource and formats complete-V1 source"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: verifies
---
# TC-404: format takes the qsl-cst ParsedSource and formats complete-V1 source

## Description

Verify that `format` and `format_with_limit` take the S1 output
`qsl_cst::ParsedSource` (ADR-011 §6.1 tool layer, §7.3 M-6a) and that
`src/format.rs` does not depend on the arena parse.

Scope: FR-003-AC-7.

## Test Procedure

1. Resolve every `use` edge and inline path in `src/format.rs`.
2. Parse complete-V1 source that holds a comment and a nested block with
   `qsl_cst::parse`, and confirm the native arena parser refuses the same
   bytes.
3. Call `format` on the parsed source, then parse and format the output
   again.

Tag the test `#[trace("FR-003-AC-7", "TC-404")]`.

## Expected Results

- Step 1 finds no edge to the arena `syntax` or native `parser` modules,
  and `format`'s parameter type is `&qsl_cst::ParsedSource`.
- Step 3 returns formatted text that keeps every token spelling in order
  and the comment, and the second pass gives identical bytes.

## Status

Planned; no test backs this case.

---
id: TC-931
title: "The formatter output budget is a named caller setting"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-003
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-255
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-277
    type: verifies
---

# TC-931: The formatter output budget is a named caller setting

## Description

Exercise the formatter's output-byte ceiling through its typed limits value
and through QSL's settings operation, using the concrete FR-255 row.

## Test Procedure

1. Parse an admissible complete-V1 source whose formatted output contains
   indentation, a comment and the final newline; record the exact UTF-8 byte
   length of the formatted text.
2. Call `format_with_limits` with `FormatLimits::default()` and with
   `with_output_bytes` set to one byte below, exactly at and above that length.
3. Call the settings operation with `format.output_bytes=<n>`, pass its
   resulting `FormatLimits` to `format_with_limits`, and repeat the boundary
   cases. Include zero and a value above the 1 MiB default.
4. Inspect each refusal and verify the formatter's `FormatLimits` mapping has
   no second field or setting.

## Expected Results

Every bound below the required output length returns
`resource_exhausted`/`input-bytes-exceeded`, names `format.output_bytes` and the
configured bound, and returns no partial string. The exact bound succeeds with
the final newline included. The raised value from the settings operation is
the `FormatLimits.output_bytes` value consumed by the formatter, and a value
above the default is honored. Formatting the successful output again is
idempotent and preserves the checked package identity.

## Status

Planned. This case is the QSL integration contract for QSL-673 and QSL-605.

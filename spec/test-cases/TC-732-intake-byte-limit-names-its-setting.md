---
id: TC-732
title: "The intake byte limit names its setting and clears when raised"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-260
    type: verifies
---
# TC-732: The intake byte limit names its setting and clears when raised

## Description

Verify the intake byte limit's outcome and its setting through the library builder
and FR-255's settings operation.

Scope: FR-260-AC-4.

## Test Procedure

1. Set `intake.input_bytes` to `B` and admit a document of `B + 1` bytes.
2. Raise it to `B + 1` through the intake limits' builder, and through
   FR-255's settings operation given `intake.input_bytes=<B + 1>`, and admit the same document.

Tag the tests `#[trace("TC-732", "FR-260-AC-4")]`.

## Expected Results

- Step 1: `resource_exhausted`/`intake-limit-exceeded` naming the
  input-bytes limit, bound `B`, actual `B + 1` and setting
  `intake.input_bytes`.
- Step 2: each run judges the document on its content.

## Status

Planned.

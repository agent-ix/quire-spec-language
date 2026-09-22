---
id: TC-403
title: "For a unit with no import or model selection, the assembler fills only aliases, functions and types"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-403: For a unit with no import or model selection, the assembler fills only aliases, functions and types

## Description

Verify that the assembler fills no `PackageDeclarations` member other than
`aliases`, `functions` and `types` from parsed forms.

Depends on FR-091-OQ-1 for `enums` and FR-091-OQ-2 for `ieee_profile`.

Scope: FR-091-AC-16.

## Test Procedure

1. Assemble the TC-399 source.
2. Read `enums`, `model_operations`, `dispatch_operations`,
   `dispatch_tables`, `model_correspondence` and `ieee_profile`.

Tag the test `#[trace("FR-091-AC-16", "TC-403")]`.

## Expected Results

- The first five are empty.
- `ieee_profile` is `None`.

## Status

Planned; no test backs this case.

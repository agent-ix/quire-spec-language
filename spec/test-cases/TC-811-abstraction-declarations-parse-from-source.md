---
id: TC-811
title: "Abstraction declarations parse from source and refuse unsupported or malformed forms"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-304
    type: verifies
---
# TC-811: Abstraction declarations parse from source and refuse unsupported or malformed forms

## Description

Verify the S2 parse of the QSpec FR-450 source form through spine `compile`.
Scope: FR-304-AC-8.

## Test Procedure

1. Compile source text whose `abstraction-decl` holds FR-304-AC-1's three
   bindings (`type`, `population` and `operation` forms) in a package that
   selects `quire.model.complete/v1`.
2. Compile the same declaration in a package that does not select
   `quire.model.complete/v1`.
3. Compile a declaration with no binding.
4. Compile a declaration whose `type` binding's Rust path is unquoted.

## Expected Results

1. The compile succeeds, and its `CheckedAbstractionRelation` equals the
   one TC-797 step 1 checks.
2. Refused `unsupported_construct` at the declaration's span.
3. Refused `invalid_syntax` at the offending token.
4. Refused `invalid_syntax` at the unquoted path.

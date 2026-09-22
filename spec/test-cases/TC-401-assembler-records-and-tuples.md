---
id: TC-401
title: "The assembler builds record and tuple declarations with check-minted keys and resolves names to them"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-401: The assembler builds record and tuple declarations with check-minted keys and resolves names to them

## Description

Verify that record and tuple forms become composite declarations whose
keys `check` mints (FB-13, ADR-013 O-04), and that a parameter typed by a
record name resolves to `ValueType::Composite` of that key.

Depends on FR-091-OQ-3: the key preimage needs the package's name and
version.

Scope: FR-091-AC-14.

## Test Procedure

1. Assemble a unit with `record Point { x: Int[0, 9]; y: Int[0, 9]; }`,
   `tuple Pair(Int[0, 9], Int[0, 9]);` and
   `function px using v(p: Point): Int[0, 9] pure { p.x }`.
2. Read `types` and `px`'s parameter type.
3. Check the result with `PackageDeclarations::check`.

Tag the test `#[trace("FR-091-AC-14", "TC-401")]`.

## Expected Results

- `types` holds one record `Point` with fields `x`, `y` and one tuple
  `Pair` with two elements, with distinct keys.
- `px`'s parameter type is `ValueType::Composite` of `Point`'s key.
- Checking succeeds.

## Status

Planned; no test backs this case.

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
keys `check` mints from the required checked-package name and declared paths
(QSpec FR-322, FR-480-AC-5), and that a parameter typed by a record name resolves
to `ValueType::Composite` of that key.

This catches keys minted over a constant package identity, keys that differ
between two compiles of one source, and keys tied to the source-owner unit label.

Scope: FR-091-AC-18.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Assemble, with the same required `package_name` in every run and under source owner authority `a`, identity `u`, a unit with
   `record Point { x: Int[0, 9]; y: Int[0, 9]; }`,
   `tuple Pair(Int[0, 9], Int[0, 9]);` and
   `function px using v(p: Point): Int[0, 9] pure { p.x }`.
2. Read `types` and `px`'s resolved parameter type.
3. Check the result with `PackageDeclarations::check`, and read `px`'s
   checked node id.
4. Repeat steps 1 to 3 under owner (`a`, `u`), then under (`a`, `w`) with the same `package_name`, and once under (`a`, `w`) with a different required `package_name`.
5. Search `src/` for an item named `DEFAULT_PACKAGE_IDENTITY`.

Tag the test `#[trace("FR-091-AC-18", "TC-401")]`.

## Expected Results

- `types` holds one record `Point`, with fields `x` and `y`, and one tuple
  `Pair`, with two elements, under distinct keys.
- `px`'s resolved parameter type is `ValueType::Composite` of `Point`'s key.
- Checking succeeds.
- Under (`a`, `u`) again, the `Point` key, the `Pair` key and `px`'s node id
  equal step 1's. Under (`a`, `w`) with the same `package_name`, all three
  still equal step 1's and have QSpec FR-322 structural package/path/ordinal components. With a different required `package_name`, all three keys differ in their package component.
- Step 5 finds no such item.

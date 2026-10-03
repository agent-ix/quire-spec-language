---
id: TC-759
title: "The front-end operations compose to spine compile"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-278
    type: verifies
---
# TC-759: The front-end operations compose to spine compile

## Description

Verify `parse`, `select`, `check` and `package` against FR-027's `compile` command.

Scope: FR-278-AC-1 to FR-278-AC-3.

## Test Procedure

1. Compose the four operations over `tests/fixtures/spine-compile.native`, FR-027-AC-9's domain-package request and FR-027-AC-10's library request, and compare each with the bytes FR-027's `compile` command writes; read each result back with QSL's I2 reader.
2. Call `parse` on a source with a syntax error; `select` on a unit whose `model` declaration names an unsupplied document; `check` on FR-100-AC-5's `inv` source. Compile the same three sources with FR-027's `compile` command.
3. Take the `EmittedPackage` of step 1's first source and give its source provision to FR-098's `replay`.

Tag the tests `#[trace("TC-759", "<AC id>")]`.

## Expected Results

- Step 1: each byte string is equal and reads back Verified.
- Step 2: `parse` refuses `invalid_syntax`, `select` refuses at the `model` declaration and `check` refuses `ill_typed`; each stage and cause code equals the `compile` command's.
- Step 3: the provision's digests name every source, and `replay` recompiles to the same `package_id`.

---
id: TC-163
title: "Function identity and provenance survive checking and package conversion"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-065
    type: verifies
---
# TC-163: Function identity and provenance survive checking and package conversion

## Description

Verify that the function packaging/lowering public API accepts only checked
objects or verified checked-package bytes, that a function declaration's
identity and a call's source occurrence survive checking, linking and v2
emission unchanged, and that the `infer_form` function arms are reduced to
one call each with no inline branching. Scope: FR-065-AC-1 through
FR-065-AC-4.

## Test Procedure

1. Attempt to call the function packaging/lowering public API with a raw CST
   node and with a raw source string; attempt the same call with a checked
   node and with verified checked-package bytes.
2. Parse a source file declaring one function and one call to it. Check it,
   read the function declaration's identity immediately after `check`. Link
   the checked graph into a checked package and read the identity again
   after linking. Emit v2 bytes and decode them, reading the identity a
   third time after decoding.
3. Reorder unrelated top-level declarations in the source file from step 2
   (leaving the function and its call unchanged) and repeat step 2.
4. From the same source file, resolve the call's source occurrence
   (identity, role, ordinal) before linking, after linking, and after
   decoding from v2 bytes.
5. Hand-build an alternate package whose source map region for that
   occurrence has one byte corrupted, and resolve the occurrence against it.
6. Inspect the `infer_form` function-declaration and function-application
   arms' source: count calls into `Value` family check code and count any
   other conditional, lookup or loop statement present directly in the arm.

## Expected Results

- Step 1: the CST and raw-string calls fail to compile (no accepting
  overload or conversion exists); the checked-node and checked-package-bytes
  calls succeed.
- Step 2: the identity read after `check`, after linking, and after decoding
  are all equal.
- Step 3: the identity from step 2 is unchanged by the reorder.
- Step 4: all three occurrence resolutions return the same byte span.
- Step 5: the corrupted package's resolved span differs from step 4's,
  showing the assertion reads the actual region rather than a fixed
  constant.
- Step 6: each arm contains exactly one call into `Value`'s family check
  code and no other conditional, lookup or loop; a code-shape check against a
  fixed budget fails if either arm regains inline branching.

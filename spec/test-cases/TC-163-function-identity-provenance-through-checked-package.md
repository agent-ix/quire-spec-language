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
emission unchanged, and that the identities equal QSpec's vectors. Scope:
FR-065-AC-1 through FR-065-AC-3 and FR-065-AC-8.

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
6. Under source owner (`a`, `u`), check `both` and `nb` as FR-065-AC-8 gives
   them, and read the identity of `both` and of the call `both(a, true)`.
   Run the application-node key builder over every `operation_vectors`
   preimage of QSpec's `node-identity-vectors.json`, read at run time.

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
- Step 6: `both`'s identity is FR-092 vector F2 and the call's is E2, in
  domain `quire.checked-semantic-node/v1`; each QSpec operation vector's
  recomputed key equals its recorded `sha256`.

## Status

Step 6 (FR-065-AC-8) is unbacked until QSL-156 slice A4b switches the
checker's minter to the FR-092 and FR-093 keys.

Steps 1-5 (FR-065-AC-1 through AC-3) are covered by
`identity_survives_v2_round_trip` and
`function_identity_survives_reordering_check_linking_and_a_v2_round_trip`
(`qsl-eval/tests/it/dispatch_calls.rs`) -- see FR-065's own Status section for the
current per-AC accounting; AC-1 and AC-3 remain unbacked (owner QSL-154).

FR-065-AC-4 moved to TC-376 when QSL-148's spec lane made it a
behavioural criterion; this test case no longer carries a code-shape step.

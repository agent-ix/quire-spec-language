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
one call each with no inline branching, and that the identities equal
QSpec's vectors. Scope: FR-065-AC-1 through FR-065-AC-4 and FR-065-AC-8.

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
7. Check a fixture function declaration and a fixture node whose body
   contains a call, both taken from QSpec's `node-identity-vectors.json` and
   FR-322 application-node vectors, and read each checked identity.

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
- Step 7: each identity equals its QSpec vector's digest in domain
  `quire.checked-semantic-node/v1`.

## Status

Step 7 (FR-065-AC-8) is unbacked until QSL-156 slice A4b switches the
checker's minter to the FR-322 preimages.

Steps 1-5 (FR-065-AC-1 through AC-3) are covered by
`identity_survives_v2_round_trip` and
`function_identity_survives_reordering_check_linking_and_a_v2_round_trip`
(`qsl-eval/tests/it/dispatch_calls.rs`) -- see FR-065's own Status section for the
current per-AC accounting; AC-1 and AC-3 remain unbacked (owner QSL-154).

Step 6 (FR-065-AC-4) is not implemented by a code-shape/AST test in the
delivered code, per the [testing-policy ruling](https://linear.app/agent-ix/issue/QSL-148#comment-2a4d2837)
(Peter, QSL-148, 2026-09-22, relayed by the QSL team lead: test what the
family check accepts and refuses, not the arm's code shape or placement).
The fact step 6 would verify is true of the delivered code --
`infer_form`'s `Call` arm (`qsl-semantics/src/check/check.rs`) is exactly one call into
`super::family::check_application` and holds no other conditional, lookup
or loop -- but AC-4 is **true by inspection, not backed**: `TC-376`'s
behavioral tests of `check_application` itself would keep passing even if
a future change reintroduced a conditional directly into `infer_form`'s
`Call` arm, since none of them examine the arm's shape. See FR-065's
Status section, AC-4 row, for the full reasoning (PR #303 review,
finding 3).

---
id: TC-733
title: "Library preimage and observation reads judge deep documents on their content"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-261
    type: verifies
---
# TC-733: Library preimage and observation reads judge deep documents on their content

## Description

Verify that the three D-4.10 sites read through the shared reader, keep
malformed input apart from limits and from wrong shapes.

Scope: FR-261-AC-1, FR-261-AC-2, FR-261-AC-3.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Run the library package identity read on: a canonical preimage object
   whose members are valid except that `edition` holds an array nested
   100,000 deep; the
   bytes `not json`; and a top-level array.
2. Run observation digest admission on a snapshot whose field value is
   nested 100,000 deep, within `observation.input_bytes`, first under its
   own digest and then under a different digest.
3. Read a snapshot whose population field holds a value nested 100,000
   levels deep that the field's declared type does not admit. First with
   `observation.input_bytes` and `observation.values` raised to fit; then
   with `observation.values` at 50,000, which the nested value alone
   crosses.

Tag the tests `#[trace("TC-733", "FR-261-AC-1")]`, `#[trace("TC-733", "FR-261-AC-2")]`, `#[trace("TC-733", "FR-261-AC-3")]`.

## Expected Results

- Step 1: `PreimageDefect::MemberType("edition")`; the malformed-input defect
  with the first malformed byte's offset; and the not-an-object defect.
- Step 2: the digest is read and admission proceeds to FR-106's later
  checks; under the other digest, `stale_dependency`/`content-mismatch`.
- Step 3: refused `invalid_runtime_input`/`wrong-value-kind` at the field;
  then refused naming
  `observation.values`, bound 50,000 and the count reached, 50,001.

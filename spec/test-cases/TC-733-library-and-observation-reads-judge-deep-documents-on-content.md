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
malformed input apart from limits and from wrong shapes, and admit deep
documents.

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
3. With `observation.input_bytes` and `observation.values` raised to fit,
   admit a snapshot whose population field holds a recursive value 100,000
   levels deep that its declared type admits. Then set `observation.values`
   one below the document's value count, and then raise it through
   `ObservationLimits`' builder.

Tag the tests `#[trace("TC-733", "FR-261-AC-1")]`, `#[trace("TC-733", "FR-261-AC-2")]`, `#[trace("TC-733", "FR-261-AC-3")]`.

## Expected Results

- Step 1: `PreimageDefect::MemberType("edition")`; the malformed-input defect
  with the first malformed byte's offset; and the not-an-object defect.
- Step 2: the digest is read and admission proceeds to FR-106's later
  checks; under the other digest, `stale_dependency`/`byte-digest-mismatch`.
- Step 3: admitted; then refused naming `observation.values`, its bound and
  the count reached; then admitted.

## Status

Planned.

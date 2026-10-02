---
id: TC-729
title: "QSL identities, digests and malformed-input mapping over quire-canonical at any depth"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: verifies
---
# TC-729: QSL identities, digests and malformed-input mapping over quire-canonical at any depth

## Description

Verify QSL's identities, digests and malformed-input mapping over
`quire-canonical` at 100,000 deep. The crate's own behaviour is tested in
its repository.

## Test Procedure

Run every step on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Mint the simulation state key of a 100,000-long recursive list value,
   on the small stack and on a thread with an 8 MiB stack.
2. Compute the intake `sha256-jcs` digest of a 100,000-deep JSON package
   document, with `intake.input_bytes` raised to fit, on both stacks, and
   the SHA-256 of the document's RFC 8785 text.
3. Run observation digest admission over a document holding `"\ud800"` and
   over one holding `1e400`.

Tag the tests `#[trace("TC-729", "FR-259-AC-3")]`.

## Expected Results

- Step 1: the two keys are equal.
- Step 2: the three digests are equal.
- Step 3: each is refused with observation admission's malformed-input
  refusal, carrying the byte offset of the escape or the number.

## Status

Planned.

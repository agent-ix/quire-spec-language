---
id: TC-729
title: "QSL identities, digests and allocation failures over quire-canonical at any depth"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-259
    type: verifies
---
# TC-729: QSL identities, digests and allocation failures over quire-canonical at any depth

## Description

Verify QSL's identities and digests over `quire-canonical` at 100,000
deep, and the one outcome QSL reports for its allocation failure. The
crate's own behaviour is tested in its repository. FR-260-AC-2 (TC-730) and
FR-261-AC-1 (TC-733) cover the malformed-input refusals of the sites that
parse.

## Test Procedure

Run steps 1 and 2 on a thread spawned with a 512 KiB stack unless the step says otherwise.

1. Mint the simulation state key of a 100,000-long recursive list value,
   on the small stack and on a thread with an 8 MiB stack.
2. Compute the intake `sha256-jcs` digest of a 100,000-deep JSON package
   document, with `intake.input_bytes` raised to fit, on both stacks, and
   the SHA-256 of the document's RFC 8785 text.
3. Hand an allocation failure of 4096 requested bytes, from the reader and
   from the encoder, to domain package intake, observation digest admission
   and package identity.

Tag the tests of steps 1 and 2 `#[trace("TC-729", "FR-259-AC-3")]` and
those of step 3 `#[trace("TC-729", "FR-259-AC-5")]`.

## Expected Results

- Step 1: the two keys are equal.
- Step 2: the three digests are equal.
- Step 3: each site refuses `resource_exhausted`/`allocation-failed`
  carrying 4096, and none reports a limit, a malformed value or malformed
  input.


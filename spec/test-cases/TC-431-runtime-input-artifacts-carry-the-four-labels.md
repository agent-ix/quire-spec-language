---
id: TC-431
title: "Runtime input artifacts carry the two labels, and bytes missing one refuse"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-024
    type: verifies
---
# TC-431: Runtime input artifacts carry the two labels, and bytes missing one refuse

## Description

Verify that a native-state-input/1 snapshot is constructed, emitted and
read back under the two labels, that a blank label refuses, and that bytes
naming only `identity` refuse as a missing member.

Scope: FR-018-AC-8, FR-024-AC-6.

## Test Procedure

1. Construct a snapshot under (`agent-ix`, `s`), then a snapshot with
   different content under the same labels.
2. Construct it with an empty authority, then with a blank identity.
3. Read the step 1 bytes back under their reference, and validate them
   against the local schema.
4. Remove `authority`, then `identity`, from the step 1 bytes,
   and read each edited copy under a reference whose digest is that copy's
   digest, so the read passes the selection stage.

Tag the tests `#[trace("TC-431", "FR-018-AC-8")]` and
`#[trace("TC-431", "FR-024-AC-6")]`.

## Expected Results

- Step 1: the bytes name the two labels and no other identity member; the
  two snapshots differ in bytes and digest.
- Step 2: each refuses with `invalid_source_identity`, cause `blank-label`;
  field `label` is `authority`, then `identity`.
- Step 3: the artifact's reference retains the two labels, and the schema
  accepts the bytes.
- Step 4: each refuses at the envelope stage with `invalid_runtime_input`,
  whose retained JSON error names that member, and the schema rejects the
  bytes.


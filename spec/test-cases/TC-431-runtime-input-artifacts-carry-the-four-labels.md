---
id: TC-431
title: "Runtime input artifacts carry the four labels, and two-label bytes refuse"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-018
    type: verifies
  - target: ix://agent-ix/quire-spec-language/FR-024
    type: verifies
---
# TC-431: Runtime input artifacts carry the four labels, and two-label bytes refuse

## Description

Verify that a native-state-input/1 snapshot is constructed, emitted and
read back under the four labels, that a blank label refuses, and that bytes
naming only `identity` and `revision` refuse as a missing member.

Scope: FR-018-AC-8, FR-024-AC-6.

## Test Procedure

1. Construct a snapshot under (`agent-ix`, `s`, `git`, `1`), then under
   (`agent-ix`, `s`, `semver`, `1`).
2. Construct it with an empty authority, then with a blank revision
   namespace.
3. Read the step 1 bytes back under their reference, and validate them
   against the local schema.
4. Read the same bytes without `authority`, then without
   `revision_namespace`.

Tag the tests `#[trace("TC-431", "FR-018-AC-8")]` and
`#[trace("TC-431", "FR-024-AC-6")]`.

## Expected Results

- Step 1: the bytes name the four labels; the two snapshots differ in bytes
  and digest.
- Step 2: each refuses with `invalid_source_identity`.
- Step 3: the artifact's reference retains the four labels, and the schema
  accepts the bytes.
- Step 4: each refuses with `invalid_runtime_input`/`missing-member` naming
  that member, and the schema rejects the bytes.

## Status

Planned. ADR-013 §7 slice S-4b (QSL-233).

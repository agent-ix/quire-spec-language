---
id: TC-150
title: "Vendor exact pinned bytes and verify external digests"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/NFR-011, type: verifies }
  - { target: ix://agent-ix/quire-spec-language/FR-036, type: references }
  - { target: ix://agent-ix/quire-spec-language/FR-055, type: references }
---
# TC-150: Vendor exact pinned bytes and verify external digests

## Description

Verify that vendored bytes are exactly the pinned git blob (or, for an
externally hosted file, exactly its recorded digest), for both the manifest
model and the two live `resources/native-v1` and `resources/complete-value`
trees. Scope: NFR-011-AC-2.

## Test Procedure

1. Build a manifest with one source of each kind (`qspec`, `self`,
   `external_url`) and confirm each source's destination path applies its
   `dest_prefix` correctly.
2. Save a manifest to disk and load it back; confirm its destination list is
   unchanged.
3. Load each of the two checked-in `VENDOR.json` manifests and run
   `revendor_check` against the corresponding live tree.
4. Tamper an `external_url` file's on-disk bytes and call `revendor`.

## Expected Results

- Step 1's destination list matches each source kind's `dest_prefix`/`path`
  or `dest` exactly.
- Step 2's round trip is lossless.
- Step 3 reports zero drifted and zero stray paths for both trees.
- Step 4 refuses with `Error::ExternalDrift` and leaves the on-disk bytes
  untouched; `revendor` never fetches the file to "fix" it.

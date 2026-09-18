---
id: TC-152
title: "Detect drift, missing and stray files offline, and replace a dropped pin wholesale"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/NFR-011, type: verifies }
---
# TC-152: Detect drift, missing and stray files offline, and replace a dropped pin wholesale

## Description

Verify that `revendor-check` finds every way a vendored tree can disagree
with its manifest without a clone or network access, and that `revendor`
itself removes a file a new pin no longer lists. Scope: NFR-011-AC-4,
NFR-011-AC-5.

## Test Procedure

1. Vendor one file into a synthetic tree, confirm `revendor_check` is clean,
   then change the file's bytes and check again.
2. Check a manifest entry whose file was never created on disk.
3. Check a tree holding a file the manifest does not mention; confirm
   `VENDOR.json` and `README.md` are excused. On a Unix host, add a symlink
   instead of a plain file and check again.
4. Vendor a tree with two files, drop one from the manifest, and run
   `revendor`.

## Expected Results

- Step 1 is clean before the change; after it, exactly one drifted entry
  names the changed destination with its expected and actual digests.
- Step 2 reports the entry drifted with `actual_sha256: "missing"`.
- Step 3's unmentioned file is reported stray; `VENDOR.json`/`README.md`
  never are; the symlink, added under a destination the manifest does not
  list, is reported stray the same way rather than silently skipped.
- Step 4 removes the dropped file, reports it in `RevendorReport::removed`,
  and leaves the still-listed file and `README.md` in place.

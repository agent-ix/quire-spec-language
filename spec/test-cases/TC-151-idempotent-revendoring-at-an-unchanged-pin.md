---
id: TC-151
title: "Idempotent re-vendoring at an unchanged pin"
type: TC
relationships:
  - { target: ix://agent-ix/quire-spec-language/NFR-011, type: verifies }
---
# TC-151: Idempotent re-vendoring at an unchanged pin

## Description

Verify that running `revendor` twice at the same pin is a no-op the second
time, using a disposable git repository this test creates on the spot (so it
needs no history beyond one commit it makes itself, and passes under a
shallow checkout), plus an optional deep check against a real
`quire-specification` clone. Scope: NFR-011-AC-3.

## Test Procedure

1. Initialize a throwaway git repository, commit one file, and pin a `self`
   source at that commit. Run `revendor` into a scratch destination.
2. Run `revendor` again with the same manifest and destination.
3. Tamper the destination file's bytes directly (not the immutable pinned
   commit), then run `revendor` a third time at the same pin.
4. Given a real local `quire-specification` clone (`QSPEC_CLONE_PATH`), copy
   each checked-in tree to a scratch directory and run `revendor` against
   the clone at the manifest's recorded pin.

## Expected Results

- Step 1 writes the file and records its digest.
- Step 2 writes and removes nothing (`RevendorReport::is_noop` is `true`),
  and the manifest's recorded digest is unchanged.
- Step 3 detects the tampered bytes and rewrites the file back to the
  pinned content -- proving step 2's "no-op" assertion is not vacuous.
- Step 4 (skipped cleanly when `QSPEC_CLONE_PATH` is unset) writes and
  removes nothing for either tree, and the manifest saved from the scratch
  copy is byte-identical to the checked-in one; the checked-in
  `resources/` tree is never revendored in place.

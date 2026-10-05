---
id: TC-772
title: "One manifest conversion serves compile-time providers and plugins"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-288
    type: verifies
---
# TC-772: One manifest conversion serves compile-time providers and plugins

## Description

Verify layer R's manifest-to-descriptor conversion and that `BackendId` is the backend identity alone.

Scope: FR-288-AC-1 to FR-288-AC-6.

## Test Procedure

1. Convert one FR-331 manifest as a compile-time provider's, and the same manifest as the body of a plugin `hello` frame.
2. Compare the descriptor's `BackendId` with the manifest's backend identity; change in turn one advertised kind, one domain, one option and one bound, and convert each.
3. Convert a manifest missing its advertised kinds, then build a registry from it and one valid manifest.
4. Route an item to a test provider whose counterexample replay reproduces; read the `BackendId` from the registry's descriptor, the replay envelope and the terminal record. Then convert two manifests that differ only in a tool version member.

5. Build descriptors with origin `Linked` and with origin `Process` and read the origin back.
6. Register the `Linked` and the `Process` descriptor of one identity in each order, then register one descriptor twice.

Tag the tests `#[trace("TC-772", "<AC id>")]`.

Layer R owns steps 5 and 6 (FR-288-AC-5, FR-288-AC-6), tested in `qsl-route`. Steps 1 to 4 (FR-288-AC-1 to FR-288-AC-4) run through the driver's registry builder, which owns the manifest conversion (IR-609 lane); no `qsl-route` test covers them.

## Expected Results

- Step 1: the two descriptors are equal except for their origin.
- Step 2: the `BackendId` equals the backend identity; each change gives different advertised capabilities and the same `BackendId`.
- Step 3: the conversion refuses with the FR-331 reader's cause; the registry holds only the valid manifest.
- Step 4: the three `BackendId` values are equal; the two manifests give the same `BackendId`.
- Step 5: each descriptor holds the origin it was built with, with the same `BackendId`.
- Step 6: the differing origins conflict in both orders: both are withdrawn and the digest is recorded; the repeat is idempotent.

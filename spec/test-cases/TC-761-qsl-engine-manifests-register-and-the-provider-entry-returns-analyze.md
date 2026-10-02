---
id: TC-761
title: "QSL engine manifests register, and the provider entry returns analyze's records"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-280
    type: verifies
---
# TC-761: QSL engine manifests register, and the provider entry returns analyze's records

## Description

Verify what QSL supplies to the driver's `prove`: each engine's FR-331 manifest and the provider entry.

Scope: FR-280-AC-1, FR-280-AC-2.

## Test Procedure

1. Read each QSL engine's provider manifest with the FR-331 reader and convert it with FR-288's conversion.
2. For each TC-762 item, build an FR-331 provider request routing it to its engine and pass it, with the `EmittedPackage` of the same source, to the provider entry; call `analyze` with the same item.

Tag the tests `#[trace("TC-761", "<AC id>")]`.

## Expected Results

- Step 1: each converts to a `BackendDescriptor` whose (kind, mode) pairs equal the claim kinds the engine settles in TC-762.
- Step 2: each pair of terminal records is equal.

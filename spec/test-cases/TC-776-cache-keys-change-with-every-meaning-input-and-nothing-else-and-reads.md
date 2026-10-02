---
id: TC-776
title: "analyze requests and records have one canonical form that changes only with key members"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-292
    type: verifies
---
# TC-776: analyze requests and records have one canonical form that changes only with key members

## Description

Verify the canonical key-member bytes of an `analyze` request, the determinism of `analyze` records, and the typed record reader the cache uses.

Scope: FR-292-AC-1 to FR-292-AC-3.

## Test Procedure

1. Serialize the key members of TC-762's `analyze` request, then of requests that each change exactly one claim bound, one result-affecting option or the state budget; then of requests that each change only the request file path, the working directory, the wall-clock deadline or the execution backend.
2. Run TC-762's `analyze` request twice from different working directories and request file paths, and serialize the records to RFC 8785 bytes.
3. Read each step 2 record back with the typed reader; then read the same bytes with whitespace inserted between two members, and with two members swapped.

Tag the tests `#[trace("TC-776", "<AC id>")]`.

## Expected Results

- Step 1: each meaning change gives different bytes; each other change gives the base bytes.
- Step 2: the byte strings are equal.
- Step 3: each record reads back equal to the one serialized; the altered byte strings are refused.

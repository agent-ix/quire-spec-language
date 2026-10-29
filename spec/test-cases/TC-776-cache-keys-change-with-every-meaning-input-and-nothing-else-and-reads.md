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

Scope: FR-292-AC-1 to FR-292-AC-4.

## Test Procedure

1. Serialize the key members of TC-762's `analyze` request, then of requests that each change exactly one claim bound, one result-affecting option or the state budget; then of requests that each change only the request file path, the working directory, the wall-clock deadline, `max_solver_time` or the execution backend.
2. Run TC-762's `analyze` request twice from different working directories and request file paths, and serialize the records to RFC 8785 bytes.
3. Read each step 2 record back with the typed reader; then read the same bytes with whitespace inserted between two members, and with two members swapped.
4. Parse TC-762's canonical request bytes and re-encode them; compare each `items` entry with the item's QSpec FR-331 frame-form spelling, `candidates` included, plus its `settlement`; serialize the request with one item's settlement moved to another engine. Serialize requests that each change one `initial` state key, one universe key or one domain package identity, one that lists the domain packages in another order, and ones with `deadline_ms` 1 and absent.

Tag the tests `#[trace("TC-776", "<AC id>")]`.

## Expected Results

- Step 1: each meaning change gives different bytes; each other change gives the base bytes.
- Step 2: the byte strings are equal.
- Step 3: each record reads back equal to the one serialized; the altered byte strings are refused.
- Step 4: the bytes equal their re-encoding, with members exactly `format` (`quire.analyze-request/1`), `binding`, `items`, `limits`, and each item matches; the three binding changes and the moved settlement give different bytes; the reordering and the two deadlines give the base bytes.

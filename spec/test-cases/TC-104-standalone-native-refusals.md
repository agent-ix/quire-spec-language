---
id: TC-104
title: "Preserve standalone workflow failures and limits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
---
## Description

Integration, P1: exercise request, file, digest and runtime failures at the real
command boundary without substituting compiler or execution logic.

## Test Procedure

Mutate selected bytes, request fields/format, paths and budgets; inspect the
actual stage/code and available provenance. Retry an unchanged default request.

## Expected Results

Malformed requests and I/O exit 2; semantic refusals exit 1; budget exhaustion
exits 3 without truth. Fresh requests recover and no hosted service is needed.

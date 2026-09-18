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
Validate every emitted run envelope against its schema and resolve error codes
through the native catalog. Reject missing result fields or truth attached to
refusal. Adapt an independently typed cancelled package error and require exit 22.
Exercise command-specific arity errors before I/O, plus absolute and parent-relative
file selections in a request stored separately from the current working directory.

## Expected Results

On FR-301's six-code contract, malformed requests, I/O, invalid command usage
and semantic refusals other than a real, profile-gated capability this build
does not admit all exit 20; that capability disposition exits 21; budget
exhaustion exits 22 without truth. Fresh requests recover and no hosted
service is needed.

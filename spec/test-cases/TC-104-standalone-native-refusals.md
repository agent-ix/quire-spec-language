---
id: TC-104
title: "Preserve standalone workflow failures and limits"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
---
## Description

Integration, P1: exercise request, file, digest and runtime failures through
`command` in process without substituting compiler or execution logic.

## Test Procedure

Mutate selected bytes, request fields/format, paths and budgets; inspect the
actual stage/code and available provenance. Retry an unchanged default request.
Validate every emitted run envelope against its schema and resolve error codes
through the native catalog. Reject missing result fields or truth attached to
refusal. Adapt an independently typed cancelled package error and require exit 22.
Exercise `request_bytes` and `dependent_bytes` one byte short and at size, and
absolute and parent-relative file selections in a request stored separately
from the current working directory.

## Expected Results

On FR-301's six-code contract, the outcome's FR-285 exit code is 20 for
malformed requests, I/O and semantic refusals other than a real, profile-gated capability this build
does not admit; 21 for that capability disposition; 22 for budget
exhaustion, without truth. A limit refusal names its setting and configured
value. Fresh requests recover and no hosted
service is needed.

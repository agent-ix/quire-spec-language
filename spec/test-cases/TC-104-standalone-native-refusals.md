---
id: TC-104
title: "Native-v1 request intake failures, limits and path resolution"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
---
## Description

Integration, P1: exercise request, file and digest failures, the intake
limits and path resolution through `command` in process with `1-draft`
requests, without substituting compiler or execution logic.

Scope: FR-026-AC-3 to FR-026-AC-5.

## Test Procedure

1. Call `command` with a `1-draft` native-run/1 request with a stale program
   source digest, a malformed request, an unknown format and a missing
   program file. Validate each command-error envelope against the native
   result schema and resolve its code through the native catalog.
2. Call it with a request one byte over `request_bytes`, then one whose
   dependent files total one byte over `dependent_bytes`. Repeat each with
   that limit raised to its size. Then call it with a fresh request under
   the default limits.
3. Store a request outside the working directory whose file selections are
   relative, absolute and parent-relative, and call `command` with its path
   from two different process working directories.

Capture the process streams around every call. Tag the tests
`#[trace("TC-104", "FR-026-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: each refuses at its actual stage with its catalogued code and the
  available source details, with no result document and exit code 20; every
  envelope is `native-run-result/2` and validates.
- Step 2: each refusal names the setting and its configured value; the
  raised requests run; the fresh request succeeds.
- Step 3: both calls return equal outcomes; each file selection resolves
  against the request's directory, or as written when absolute.
- No call writes a byte to stdout or stderr.

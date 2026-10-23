---
id: TC-104
title: "Native-v1 request intake failures, limits and arity"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
---
## Description

Integration, P1: exercise request, file and digest failures, the intake
limits and command arity at the real `run` command boundary with `1-draft`
requests, without substituting compiler or execution logic.

Scope: FR-026-AC-3 to FR-026-AC-5.

## Test Procedure

1. Run a `1-draft` native-run/1 request with a stale program source digest,
   a malformed request, an unknown format and a missing program file.
   Validate each emitted command-error envelope against the native result
   schema and resolve its code through the native catalog.
2. Run a request one byte over `request_bytes`, then one whose dependent
   files total one byte over `dependent_bytes`. Rerun each with that limit
   raised to its size. Then run a fresh request under the default limits.
3. Run `run` and `compile` with missing and with excess operands, with no
   request file present. Run a request stored outside the working directory
   whose file selections are relative, absolute and parent-relative.

Tag the tests `#[trace("TC-104", "FR-026-AC-n")]` with the AC each backs.

## Expected Results

- Step 1: each refuses at its actual stage with its catalogued code and the
  available source details, writes nothing to stdout and exits 20; every
  envelope validates.
- Step 2: each refusal names the limit, its configured value and its CLI
  option; the raised requests run; the fresh request succeeds.
- Step 3: each usage error exits 20 before any file is read; each operand
  form resolves against the request's directory, whatever the working
  directory.

---
id: TC-105
title: "Export and reread exact standalone compiler output"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
## Description

Integration, P1: call `command` in process to emit a selected native package
without runtime inputs, then reread its exact bytes through the existing API.

## Test Procedure

Call `command` with compile requests over source-only files and compare the
outcome's package bytes with the existing static pipeline.
Reread the output with exact bindings. Mutate format, fields, source selection
and syntax.
Copy only the request-selected sources into a fresh compile directory. Check
a request past `dependent_bytes` and the same request with the limit raised,
a library source taking the request one byte over `dependent_bytes` and the
same request with the limit raised, a request of 65 small library files
under the limit, and unchanged catalog spellings. Capture the process streams and require
that `command` writes nothing to them.

## Expected Results

Successful bytes/digest match with no wrapper or newline. Refusals preserve their
stage/code and carry no package bytes, and each outcome's FR-285 exit code is
the status FR-027 states.

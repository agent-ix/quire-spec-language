---
id: TC-105
title: "Export and reread exact standalone compiler output"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-027
    type: verifies
---
## Description

Integration, P1: use the real compile binary to emit a selected native package
without runtime inputs, then reread its exact bytes through the existing API.

## Test Procedure

Run compile on source-only files and compare with the existing static pipeline.
Reread the output with exact bindings. Mutate format, fields, source selection
and syntax; run the existing standalone and syntax command tests.
Copy only the request-selected sources into a fresh compile directory. Check
command-specific arity errors before file I/O, named count exhaustion for model,
snapshot and invocation groups, and unchanged catalog spellings. On Linux,
redirect the actual binary's artifact output to /dev/full and require exit 30,
[FR-301](ix://agent-ix/quire-specification/FR-301)'s code for tool failure.

## Expected Results

Successful bytes/digest match with no wrapper or newline. Refusals preserve their
stage/code and produce no package bytes; other commands retain their behavior.

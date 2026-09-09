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

## Expected Results

Successful bytes/digest match with no wrapper or newline. Refusals preserve their
stage/code and produce no package bytes; other commands retain their behavior.

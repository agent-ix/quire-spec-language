---
id: TC-099
title: "Read and execute native input artifacts"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-024
    type: verifies
---
## Description

Round-trip real native snapshots/invocations and every flat value variant.

## Test Procedure

Read exact constructor bytes and whitespace/reordered encodings under their
selected references. Compare original drafts, bytes and digests. Execute
reread inputs through actual state and operation packages.
Validate the real envelopes against the local Draft 2020-12 schema, including
all observations, all ten value variants, populated nested records, duplicate
vector occurrences and explicit null invocation results.

## Expected Results

All fields and duplicate vector occurrences survive. Healthy/violating truth
and frame refusals agree with the original native inputs.

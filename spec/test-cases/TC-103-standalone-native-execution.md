---
id: TC-103
title: "Execute source and runtime files through command"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
---
## Description

Integration, P1: call `command` in process with source-derived models and
selected runtime files for healthy/violating aggregates and recorded operation
frames.

## Test Procedure

Write public-API artifacts into an isolated directory, call `command` with the
request's path from a different working directory and inspect the outcome's
result, FR-285 exit code, identities and counters. Capture the process
streams and require that `command` writes nothing to them.

## Expected Results

Actual truth appears only on completed execution; operation frame defects refuse.
Selected bytes and authored identities agree with the supplied files.

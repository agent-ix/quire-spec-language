---
id: TC-103
title: "Execute source and runtime files through the CLI"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-026
    type: verifies
---
## Description

Integration, P1: run the real binary with source-derived models and selected
runtime files for healthy/violating aggregates and recorded operation frames.

## Test Procedure

Write public-API artifacts into an isolated directory, invoke run from a different
working directory and inspect output, exit status, identities and counters.
Run the existing parse/format suite unchanged.

## Expected Results

Actual truth appears only on completed execution; operation frame defects refuse.
Selected bytes and authored identities agree with the supplied files.

---
id: TC-093
title: "Refuse unqualified and exhausted projections"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-009
    type: verifies
---
# TC-093: Refuse unqualified and exhausted projections

## Description

Verify whole-package refusal and bounded fresh lowering work.

## Test Procedure

After successful native checking, request lowering for reference dereference,
numeric comparison, locals, conditionals and quantification, including a bad
clause after a healthy clause. Exercise mixed authored packages and revisions.
For a healthy fixture independently count nodes/depth and emitted bytes; request
zero, exact, one-below and above-hard limits. Retry with default limits.

## Expected Results

Unsupported forms and owner populations identify the affected clause and source.
Limits refuse before publishing any output. Exact limits succeed and the retry
has fresh accounting. Every obligation remains in the unchanged native package.

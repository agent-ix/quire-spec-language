---
id: TC-109
title: "Execute Markdown through the standalone command"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-031
    type: verifies
---
## Description

Integration, P1: run the actual binary on licensed generated Markdown, model and
snapshot/invocation files; assert values, byte-map correspondence and refusals.

## Test Procedure

Run healthy/violating/refused cases under LF and CRLF. Exercise native syntax and
extraction failures, malformed selection records, stale source, disabled feature,
unsupported command combinations and exhausted budgets followed by fresh retry.

## Expected Results

Actual native results retain original and body identity plus untouched producer
observations. Incomplete/refused paths supply no truth or partial artifact.

---
id: TC-017
title: "Native CLI OS argument boundary"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: verifies
---

## Description

Native CLI OS argument boundary. Type: E2E; priority P1. Traces: FR-010-AC-3, FR-010-AC-6, FR-010-AC-7.

## Test Procedure

On Unix invoke the real compiler with byte FF in each command/label position, then parse unchanged valid source from a temporary non-UTF-8 path. Verify labels/digest and missing-file behavior.

## Expected Results

Invalid command/label encoding returns usage exit 2 without panic; valid OS paths reach actual I/O and parsing, preserving exact source labels and digest.


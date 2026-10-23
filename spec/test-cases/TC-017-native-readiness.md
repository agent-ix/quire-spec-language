---
id: TC-017
title: "Source intake at a non-UTF-8 path"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: verifies
---

## Description

Source intake at a non-UTF-8 path. Type: integration; priority P1. Traces: FR-010-AC-6, FR-010-AC-7.

## Test Procedure

On Unix, read unchanged valid source from a temporary non-UTF-8 path through FR-001's source intake and call the `parse` operation over it. Verify labels/digest and missing-file behavior.

## Expected Results

Valid OS paths reach actual I/O and parsing, preserving exact source labels and digest; a missing path refuses with exit code 20 and no outcome document.


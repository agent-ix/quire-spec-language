---
id: TC-015
title: "Parse and format outcomes"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-010
    type: verifies
---

## Description

Parse and format outcomes. Type: integration; priority P1. Traces: FR-010-AC-1, FR-010-AC-2, FR-010-AC-4, FR-010-AC-5, FR-010-AC-7, FR-010-AC-9.

## Test Procedure

Call the `parse` and `format` operations in process for an empty identity, a missing source file and an oversized temporary source. Compare the parse outcome's source identity with the supplied labels. Parse a source whose recognized-but-unsupported construct and whose genuinely malformed syntax each reach the parser's boundary, and distinguish their outcomes' FR-285 exit codes.

## Expected Results

FR-285 exit codes are 0/20/21/22 for success/invalid-or-refused-input/recognized-but-unsupported/incomplete, [FR-301](ix://agent-ix/quire-specification/FR-301)'s contract. Only a successful parse produces a `success` outcome document, and no path claims logical evaluation.


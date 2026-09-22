---
id: TC-402
title: "The assembler lives in the check core and has no edge to qsl-cst"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-402: The assembler lives in the check core and has no edge to qsl-cst

## Description

Verify that the assembler is a layer-3 `check`-core module that reads
parsed forms only (ADR-011 §3 FB-01, §6.1).

Scope: FR-091-AC-15.

## Test Procedure

1. Locate the assembler module and confirm its path is under the `check`
   core.
2. Resolve every `use` edge and inline path in it and in its test module.

Tag the test `#[trace("FR-091-AC-15", "TC-402")]`.

## Expected Results

- The module is under the `check` core.
- No edge resolves to `qsl_cst` or to a type re-exported from it.
- Its tests build input through the S2 entry or from parsed-form values.

## Status

Planned; no test backs this case.

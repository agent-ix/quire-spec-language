---
id: TC-402
title: "The assembler lives in the check core and its non-test code has no edge to qsl-cst"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-402: The assembler lives in the check core and its non-test code has no edge to qsl-cst

## Description

Verify that the assembler is a layer-3 `check`-core module whose non-test
code reads parsed forms only (ADR-011 §3 FB-01, §6.1). Its `#[cfg(test)]`
code may reach `qsl_cst` to run S1 and S2, so the scan excludes it.

Scope: FR-091-AC-20.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Locate the assembler module and confirm that its path is under the
   `check` core.
2. Resolve every `use` edge and inline path in its non-`#[cfg(test)]` items.
3. Resolve every `qsl_cst` edge in its `#[cfg(test)]` items.

Tag the test `#[trace("FR-091-AC-20", "TC-402")]`.

## Expected Results

- The module is under the `check` core.
- Step 2 finds no edge to `qsl_cst`, or to any type it re-exports.
- Every step-3 edge is a call to `qsl_cst::parse` or `parse_source` that
  feeds the S2 entry.

## Status

Planned; no test backs this case.

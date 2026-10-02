---
id: TC-397
title: "S2 builds every body S1 admits, whatever its depth"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-397: S2 builds every body S1 admits, whatever its depth

## Description

Verify that S2 takes no limit set and builds a form for each body S1 admits,
whatever its depth (FR-091 "S2 depth", ADR-030 D-4.2). Depth counts
`Expression` nodes, with the body root at depth 1.

Scope: FR-091-AC-9.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. For each of the bodies `not`×8 `a`, `not`×20 `a` and `not`×129 `a`, parse
   the unit at the default S1 limits and confirm `is_admissible()`.
2. Run S2 on each source, with no limit set.

Tag the test `#[trace("FR-091-AC-9", "TC-397")]`.

## Expected Results

- Step 1 confirms that all three sources are admissible.
- Each builds a form whose deepest expression node is at depth 9, 21 and
  130 respectively.
- No S2 outcome names a depth.

## Status

Planned for the form above. The present
`the_nesting_depth_bound_refuses_past_its_limit`
(`qsl-forms/tests/it/value_forms.rs`) drives the S2 depth limit ADR-030
deletes and is replaced by this case.

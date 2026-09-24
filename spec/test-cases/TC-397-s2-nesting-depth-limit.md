---
id: TC-397
title: "S2 bounds expression depth by its explicit limit, independently of S1"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-397: S2 bounds expression depth by its explicit limit, independently of S1

## Description

Verify that S2's recursion is bounded by its explicit nesting-depth limit
(ADR-011 §2.3 Limits), and that exceeding it gives S2's limit refusal
rather than a truncated form or another cause. Depth counts `Expression`
nodes, with the body root at depth 1.

Each source is confirmed admissible at S1 first, so every refusal observed
here is S2's, not S1's parser-nesting clamp.

Scope: FR-091-AC-9.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Set the S2 nesting-depth bound to `L = 8`.
2. For each of the bodies `not`×7 `a`, `not`×8 `a` and `not`×20 `a`, parse
   the unit and confirm `is_admissible()`.
3. Run S2 on each source.

Tag the test `#[trace("FR-091-AC-9", "TC-397")]`.

## Expected Results

- Step 2 confirms that all three sources are admissible.
- `not`×7 `a` (deepest node at depth 8) builds a form.
- `not`×8 `a` gives a limit refusal naming limit kind nesting depth, bound
  `8`, and the span of the node at depth 9 (the `Name("a")` node). It returns no parsed unit.
- `not`×20 `a` gives the same kind of limit refusal, with the span of its
  first node at depth 9.

## Status

Backed by `qsl-forms/tests/it/value_forms.rs`: `the_nesting_depth_bound_refuses_past_its_limit` (QSL-141).

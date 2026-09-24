---
id: TC-403
title: "Every Value Expression node carries the span of its CST node"
type: TC
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-091
    type: verifies
---
# TC-403: Every Value Expression node carries the span of its CST node

## Description

Verify that S2 carries a span on every `Expression` node. Once FB-01 is in
force, S2 is the only source of the nested regions that E3's occurrence-keyed
source map and E9's nested-span resolution need (ADR-011 §2.2 E2, E3 and E9
rows).

Scope: FR-091-AC-10.

## Test Procedure

Every fixture unit below starts with the complete-V1 header
(`language "ix:native" edition "1-draft";`) and one profile selection whose
alias is `v`.

1. Run S2 on a unit whose one function body is `if a then b else c + d`.
   Read the spans of the `If`, `Add`, `Name("c")` and `Name("d")` nodes.
2. Run S2 on the body `(a + b) * c`, and read the spans of the `Multiply`
   and `Add` nodes.
3. Walk every `Expression` node of both bodies and confirm that each has a
   span inside its parent's span.

Tag the test `#[trace("FR-091-AC-10", "TC-403")]`.

## Expected Results

- `If` covers the whole body, `Add` covers `c + d`, and each `Name` node
  covers its single identifier.
- `Multiply` covers `(a + b) * c`. `Add` covers `a + b`, without the
  parentheses.
- Every node carries a span, and each span lies inside its parent's.

## Status

Steps 1 and 2 are backed by `qsl-forms/tests/it/value_forms.rs`: `every_expression_node_carries_its_span`. Step 3 holds by construction: `ExpressionSpans::push_child` refuses a child outside its parent, and a form with no fitting spans carries none.

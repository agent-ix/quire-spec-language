---
id: SR-1761002
title: "PR #1761 (QSL-1186d) gap analysis: FR-466 predicate join, AC-15"
type: SpecReview
analysis: gap-analysis
scope: "agent-ix/quire-spec-language; qsl-package/src/emit/tests/protocol_graph.rs, qsl-semantics/src/check/protocol_controls.rs tests at 44c004e6a"
review_set: subset
relationships:
  - target: ix://agent-ix/quire-spec-language/FR-218
    type: reviews
---

# PR #1761 gap analysis

## Summary

Ticket: QSL-1186 (1186d). Plan completion: not assessed.

**Covered:**
- **Positive case (FR-466-AC-4 and AC-15).** `a_predicate_join_references_its_function_and_reads_back` checks:
  - the `join` member names are `[policy, predicate]`;
  - `predicate` references the `function` node `both`;
  - `joined` is `[a, b]`;
  - IR verifies the package.
- **The `ill_typed`/`type-mismatch` signature rows of AC-15** (three parameters, an integer parameter, an integer result), plus a missing name and a non-function: S3's `predicate_join_checks_name_kind_and_signature`, which is pre-existing.

## Verdict

Mergeable. There is one low gap.

## Findings

| ID | Severity | Summary | Refs |
| --- | --- | --- | --- |
| FND-001 | low | Now that the join is written, nothing tests the positional binding at emission. A join whose labels are declared out of ascending order with an asymmetric function (FR-206-AC-11's `[c, a]`) should be pinned: `joined` sorted, the function's `k`-th parameter for the `k`-th label. So should the cycle the written reference makes reachable, a predicate function that reads its own protocol, which refuses `ReferenceCycle`. My probes show both behave correctly. | FR-466-AC-15, FR-206-AC-11 |
